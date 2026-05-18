use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension};

pub const CURRENT_SCHEMA_VERSION: u32 = 1;
pub const LEGACY_SCHEMA_VERSION_MAX: u32 = 4;

const BASELINE_SCHEMA: &str = include_str!("migrations/001_initial_schema.sql");

pub const MIGRATIONS: &[(u32, &str)] = &[(CURRENT_SCHEMA_VERSION, BASELINE_SCHEMA)];

pub fn run(conn: &Connection) -> Result<()> {
    let version = current_version(conn)?;

    match version {
        0 => {
            conn.execute_batch(BASELINE_SCHEMA).with_context(|| {
                format!("failed to apply consolidated database schema {CURRENT_SCHEMA_VERSION}")
            })?;
        }
        CURRENT_SCHEMA_VERSION => {
            if !matches_current_baseline_schema(conn)? {
                bail!(
                    "wallet database uses an older development schema layout; remove the local database and let the CLI recreate it"
                );
            }
        }
        2..=LEGACY_SCHEMA_VERSION_MAX => {
            bail!(
                "wallet database uses an older development schema version {version}; remove the local database and let the CLI recreate it"
            );
        }
        _ => {
            bail!(
                "wallet database schema version {version} is newer than supported version {CURRENT_SCHEMA_VERSION}"
            );
        }
    }

    Ok(())
}

fn current_version(conn: &Connection) -> Result<u32> {
    let has_schema_version: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_version')",
            [],
            |row| row.get(0),
        )
        .context("failed to check wallet database schema version table")?;

    if !has_schema_version {
        return Ok(0);
    }

    let version: Option<u32> = conn
        .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
            row.get(0)
        })
        .optional()
        .context("failed to read wallet database schema version")?;

    Ok(version.unwrap_or(0))
}

fn matches_current_baseline_schema(conn: &Connection) -> Result<bool> {
    for table in [
        "schema_version",
        "wallet_state",
        "seeds",
        "seed_vaults",
        "identities",
        "identity_private_payloads",
        "accounts",
        "account_private_payloads",
        "imported_account_vaults",
        "imported_account_payloads",
        "governance_key_vaults",
        "governance_keys",
        "governance_key_payloads",
    ] {
        if !has_table(conn, table)? {
            return Ok(false);
        }
    }

    for (table, column) in [
        ("identities", "expires_at"),
        ("accounts", "source_kind"),
        ("accounts", "imported_vault_id"),
        ("accounts", "import_kind"),
        ("accounts", "source_metadata_json"),
    ] {
        if !has_column(conn, table, column)? {
            return Ok(false);
        }
    }

    if !has_index(conn, "accounts_derived_tuple_unique")? {
        return Ok(false);
    }

    Ok(true)
}

fn has_table(conn: &Connection, name: &str) -> Result<bool> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        [name],
        |row| row.get(0),
    )
    .with_context(|| format!("failed to check table '{name}'"))
}

fn has_index(conn: &Connection, name: &str) -> Result<bool> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = ?1)",
        [name],
        |row| row.get(0),
    )
    .with_context(|| format!("failed to check index '{name}'"))
}

fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .with_context(|| format!("failed to prepare column inspection for table '{table}'"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .with_context(|| format!("failed to inspect table '{table}' columns"))?;

    for name in rows {
        if name.with_context(|| format!("failed to read column from table '{table}'"))? == column {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    fn foreign_key_delete_action(conn: &Connection, table: &str) -> String {
        conn.query_row(&format!("PRAGMA foreign_key_list({table})"), [], |row| {
            row.get(6)
        })
        .unwrap()
    }

    #[test]
    fn run_initializes_fresh_database() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();

        let version: u32 = conn
            .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);

        for table in [
            "schema_version",
            "wallet_state",
            "seeds",
            "seed_vaults",
            "identities",
            "identity_private_payloads",
            "accounts",
            "account_private_payloads",
            "imported_account_vaults",
            "imported_account_payloads",
            "governance_key_vaults",
            "governance_keys",
            "governance_key_payloads",
        ] {
            let count: u32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    params![table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "missing table {table}");
        }

        assert!(has_column(&conn, "identities", "expires_at").unwrap());
        assert!(has_column(&conn, "accounts", "source_kind").unwrap());
        assert!(has_column(&conn, "accounts", "imported_vault_id").unwrap());
        assert!(has_column(&conn, "accounts", "import_kind").unwrap());
        assert!(has_column(&conn, "accounts", "source_metadata_json").unwrap());
        assert!(has_index(&conn, "accounts_derived_tuple_unique").unwrap());

        assert_eq!(foreign_key_delete_action(&conn, "seed_vaults"), "CASCADE");
        assert_eq!(foreign_key_delete_action(&conn, "identities"), "CASCADE");
        assert_eq!(
            foreign_key_delete_action(&conn, "identity_private_payloads"),
            "CASCADE"
        );
        assert_eq!(foreign_key_delete_action(&conn, "accounts"), "CASCADE");
        assert_eq!(
            foreign_key_delete_action(&conn, "imported_account_payloads"),
            "CASCADE"
        );
        assert_eq!(
            foreign_key_delete_action(&conn, "account_private_payloads"),
            "CASCADE"
        );
        assert_eq!(
            foreign_key_delete_action(&conn, "governance_keys"),
            "CASCADE"
        );
        assert_eq!(
            foreign_key_delete_action(&conn, "governance_key_payloads"),
            "CASCADE"
        );
    }

    #[test]
    fn current_consolidated_schema_version_is_accepted() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(BASELINE_SCHEMA).unwrap();

        run(&conn).unwrap();
    }

    #[test]
    fn legacy_version_one_schema_layout_is_rejected() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_version (version INTEGER NOT NULL);
             CREATE TABLE wallet_state (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL);
             CREATE TABLE seeds (
                 id TEXT PRIMARY KEY NOT NULL,
                 label TEXT UNIQUE NOT NULL,
                 created_at INTEGER NOT NULL,
                 updated_at INTEGER NOT NULL
             );
             CREATE TABLE seed_vaults (
                 seed_id TEXT PRIMARY KEY NOT NULL REFERENCES seeds(id) ON DELETE CASCADE,
                 kdf_algorithm TEXT NOT NULL,
                 kdf_params_json TEXT NOT NULL,
                 salt BLOB NOT NULL,
                 encrypted_dek BLOB NOT NULL,
                 dek_nonce BLOB NOT NULL,
                 cipher_version INTEGER NOT NULL DEFAULT 1,
                 payload_ciphertext BLOB NOT NULL,
                 payload_nonce BLOB NOT NULL
             );
             CREATE TABLE identities (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 seed_id TEXT NOT NULL REFERENCES seeds(id) ON DELETE CASCADE,
                 network_genesis_hash TEXT NOT NULL,
                 ip_identity INTEGER NOT NULL,
                 identity_index INTEGER NOT NULL,
                 label TEXT NOT NULL,
                 status TEXT NOT NULL CHECK(status IN ('pending', 'done')),
                 created_at INTEGER NOT NULL,
                 UNIQUE(network_genesis_hash, seed_id, ip_identity, identity_index),
                 UNIQUE(network_genesis_hash, label)
             );
             CREATE TABLE identity_private_payloads (
                 identity_id INTEGER PRIMARY KEY NOT NULL REFERENCES identities(id) ON DELETE CASCADE,
                 cipher_version INTEGER NOT NULL,
                 ciphertext BLOB NOT NULL,
                 nonce BLOB NOT NULL
             );
             INSERT INTO schema_version (version) VALUES (1);",
        )
        .unwrap();

        let err = run(&conn).unwrap_err();
        assert!(err.to_string().contains("recreate"));
    }

    #[test]
    fn older_development_schema_versions_are_rejected() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_version (version INTEGER NOT NULL);
             INSERT INTO schema_version (version) VALUES (4);",
        )
        .unwrap();

        let err = run(&conn).unwrap_err();
        assert!(
            err.to_string()
                .contains("older development schema version 4")
        );
        assert!(err.to_string().contains("recreate"));
    }

    #[test]
    fn newer_schema_version_is_rejected() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_version (version INTEGER NOT NULL);
             INSERT INTO schema_version (version) VALUES (5);",
        )
        .unwrap();

        let err = run(&conn).unwrap_err();
        assert!(err.to_string().contains("newer than supported"));
    }
}
