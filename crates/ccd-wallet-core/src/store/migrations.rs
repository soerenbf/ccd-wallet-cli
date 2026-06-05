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
        "signer_owners",
        "signer_owner_vaults",
        "seed_owner_secrets",
        "ledger_owner_details",
        "identities",
        "identity_private_payloads",
        "accounts",
        "derived_account_private_payloads",
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
        ("signer_owners", "owner_kind"),
        ("signer_owner_vaults", "signer_owner_id"),
        ("seed_owner_secrets", "signer_owner_id"),
        ("ledger_owner_details", "canonical_public_key"),
        ("ledger_owner_details", "enrollment_path"),
        ("identities", "signer_owner_id"),
        ("identities", "expires_at"),
        ("accounts", "source_kind"),
        ("accounts", "signer_owner_id"),
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

    fn has_foreign_key_delete_action(
        conn: &Connection,
        table: &str,
        from_column: &str,
        target_table: &str,
        delete_action: &str,
    ) -> bool {
        let mut stmt = conn
            .prepare(&format!("PRAGMA foreign_key_list({table})"))
            .unwrap();
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(6)?,
                ))
            })
            .unwrap();

        for row in rows {
            let (to_table, from, on_delete) = row.unwrap();
            if to_table == target_table && from == from_column && on_delete == delete_action {
                return true;
            }
        }

        false
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
            "signer_owners",
            "signer_owner_vaults",
            "seed_owner_secrets",
            "ledger_owner_details",
            "identities",
            "identity_private_payloads",
            "accounts",
            "derived_account_private_payloads",
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

        assert!(has_column(&conn, "signer_owners", "owner_kind").unwrap());
        assert!(has_column(&conn, "signer_owner_vaults", "signer_owner_id").unwrap());
        assert!(has_column(&conn, "seed_owner_secrets", "signer_owner_id").unwrap());
        assert!(has_column(&conn, "ledger_owner_details", "canonical_public_key").unwrap());
        assert!(has_column(&conn, "ledger_owner_details", "enrollment_path").unwrap());
        assert!(has_column(&conn, "identities", "signer_owner_id").unwrap());
        assert!(has_column(&conn, "identities", "expires_at").unwrap());
        assert!(has_column(&conn, "accounts", "source_kind").unwrap());
        assert!(has_column(&conn, "accounts", "signer_owner_id").unwrap());
        assert!(has_column(&conn, "accounts", "imported_vault_id").unwrap());
        assert!(has_column(&conn, "accounts", "import_kind").unwrap());
        assert!(has_column(&conn, "accounts", "source_metadata_json").unwrap());
        assert!(has_index(&conn, "accounts_derived_tuple_unique").unwrap());

        assert!(has_foreign_key_delete_action(
            &conn,
            "signer_owner_vaults",
            "signer_owner_id",
            "signer_owners",
            "CASCADE"
        ));
        assert!(has_foreign_key_delete_action(
            &conn,
            "seed_owner_secrets",
            "signer_owner_id",
            "signer_owners",
            "CASCADE"
        ));
        assert!(has_foreign_key_delete_action(
            &conn,
            "ledger_owner_details",
            "signer_owner_id",
            "signer_owners",
            "CASCADE"
        ));
        assert!(has_foreign_key_delete_action(
            &conn,
            "identities",
            "signer_owner_id",
            "signer_owners",
            "CASCADE"
        ));
        assert!(has_foreign_key_delete_action(
            &conn,
            "identity_private_payloads",
            "identity_id",
            "identities",
            "CASCADE"
        ));
        assert!(has_foreign_key_delete_action(
            &conn,
            "accounts",
            "signer_owner_id",
            "signer_owners",
            "CASCADE"
        ));
        assert!(has_foreign_key_delete_action(
            &conn,
            "accounts",
            "imported_vault_id",
            "imported_account_vaults",
            "CASCADE"
        ));
        assert!(has_foreign_key_delete_action(
            &conn,
            "derived_account_private_payloads",
            "account_id",
            "accounts",
            "CASCADE"
        ));
        assert!(has_foreign_key_delete_action(
            &conn,
            "imported_account_payloads",
            "account_id",
            "accounts",
            "CASCADE"
        ));
        assert!(has_foreign_key_delete_action(
            &conn,
            "imported_account_payloads",
            "vault_id",
            "imported_account_vaults",
            "CASCADE"
        ));
        assert!(has_foreign_key_delete_action(
            &conn,
            "governance_keys",
            "vault_id",
            "governance_key_vaults",
            "CASCADE"
        ));
        assert!(has_foreign_key_delete_action(
            &conn,
            "governance_key_payloads",
            "governance_key_id",
            "governance_keys",
            "CASCADE"
        ));
    }

    #[test]
    fn signer_owner_schema_enforces_uniqueness_and_cascades() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        run(&conn).unwrap();

        conn.execute(
            "INSERT INTO signer_owners (id, owner_kind, label, created_at, updated_at)
             VALUES ('owner-1', 'seed', 'main', 1, 1)",
            [],
        )
        .unwrap();
        let duplicate_label = conn
            .execute(
                "INSERT INTO signer_owners (id, owner_kind, label, created_at, updated_at)
                 VALUES ('owner-2', 'ledger', 'main', 1, 1)",
                [],
            )
            .unwrap_err();
        assert_eq!(
            duplicate_label.sqlite_error_code(),
            Some(rusqlite::ErrorCode::ConstraintViolation)
        );

        conn.execute(
            "INSERT INTO signer_owner_vaults (
                signer_owner_id, kdf_algorithm, kdf_params_json, salt, encrypted_dek,
                dek_nonce, cipher_version, created_at, updated_at
             ) VALUES ('owner-1', 'argon2id', '{}', x'01', x'02', x'03', 1, 1, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO seed_owner_secrets (
                signer_owner_id, cipher_version, payload_ciphertext, payload_nonce
             ) VALUES ('owner-1', 1, x'04', x'05')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO identities (
                signer_owner_id, network_genesis_hash, ip_identity, identity_index,
                label, status, created_at
             ) VALUES ('owner-1', 'net', 7, 0, 'identity', 'done', 1)",
            [],
        )
        .unwrap();
        let identity_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO identity_private_payloads (identity_id, cipher_version, ciphertext, nonce)
             VALUES (?1, 1, x'06', x'07')",
            params![identity_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO accounts (
                network_genesis_hash, label, status, source_kind, signer_owner_id,
                ip_identity, identity_index, credential_counter, created_at, updated_at
             ) VALUES ('net', 'account', 'finalized', 'derived', 'owner-1', 7, 0, 0, 1, 1)",
            [],
        )
        .unwrap();
        let account_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO derived_account_private_payloads (account_id, cipher_version, ciphertext, nonce)
             VALUES (?1, 1, x'08', x'09')",
            params![account_id],
        )
        .unwrap();

        conn.execute("DELETE FROM signer_owners WHERE id = 'owner-1'", [])
            .unwrap();

        for table in [
            "signer_owner_vaults",
            "seed_owner_secrets",
            "identities",
            "identity_private_payloads",
            "accounts",
            "derived_account_private_payloads",
        ] {
            let count: u32 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 0, "expected {table} to cascade to zero rows");
        }
    }

    #[test]
    fn account_source_constraints_reject_inconsistent_rows() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();
        conn.execute(
            "INSERT INTO signer_owners (id, owner_kind, label, created_at, updated_at)
             VALUES ('owner-1', 'seed', 'main', 1, 1)",
            [],
        )
        .unwrap();

        let missing_owner = conn
            .execute(
                "INSERT INTO accounts (
                    network_genesis_hash, label, status, source_kind,
                    ip_identity, identity_index, credential_counter, created_at, updated_at
                 ) VALUES ('net', 'bad-derived', 'pending', 'derived', 7, 0, 0, 1, 1)",
                [],
            )
            .unwrap_err();
        assert_eq!(
            missing_owner.sqlite_error_code(),
            Some(rusqlite::ErrorCode::ConstraintViolation)
        );

        let imported_with_owner = conn
            .execute(
                "INSERT INTO imported_account_vaults (
                    id, network_genesis_hash, kdf_algorithm, kdf_params_json, salt,
                    encrypted_dek, dek_nonce, cipher_version, created_at, updated_at
                 ) VALUES ('vault-1', 'net', 'argon2id', '{}', x'01', x'02', x'03', 1, 1, 1)",
                [],
            )
            .and_then(|_| {
                conn.execute(
                    "INSERT INTO accounts (
                        network_genesis_hash, label, status, source_kind, signer_owner_id,
                        imported_vault_id, created_at, updated_at
                     ) VALUES ('net', 'bad-imported', 'finalized', 'imported', 'owner-1', 'vault-1', 1, 1)",
                    [],
                )
            })
            .unwrap_err();
        assert_eq!(
            imported_with_owner.sqlite_error_code(),
            Some(rusqlite::ErrorCode::ConstraintViolation)
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
