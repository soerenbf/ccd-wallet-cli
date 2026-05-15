use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension};

pub const CURRENT_SCHEMA_VERSION: u32 = 2;

const MIGRATION_1: &str = include_str!("migrations/001_initial_schema.sql");
const MIGRATION_2: &str = include_str!("migrations/002_account_creation.sql");

pub const MIGRATIONS: &[(u32, &str)] = &[(1, MIGRATION_1), (2, MIGRATION_2)];

pub fn run(conn: &Connection) -> Result<()> {
    let mut version = current_version(conn)?;

    if version > CURRENT_SCHEMA_VERSION {
        bail!(
            "wallet database schema version {version} is newer than supported version {CURRENT_SCHEMA_VERSION}"
        );
    }

    for (target_version, sql) in MIGRATIONS {
        if version < *target_version {
            conn.execute_batch(sql)
                .with_context(|| format!("failed to apply database migration {target_version}"))?;
            version = *target_version;
        }
    }

    debug_assert_eq!(version, CURRENT_SCHEMA_VERSION);

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

        assert_eq!(foreign_key_delete_action(&conn, "seed_vaults"), "CASCADE");
        assert_eq!(foreign_key_delete_action(&conn, "identities"), "CASCADE");
        assert_eq!(
            foreign_key_delete_action(&conn, "identity_private_payloads"),
            "CASCADE"
        );
        assert_eq!(foreign_key_delete_action(&conn, "accounts"), "CASCADE");
        assert_eq!(
            foreign_key_delete_action(&conn, "account_private_payloads"),
            "CASCADE"
        );
    }

    #[test]
    fn older_development_schema_is_rejected() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_version (version INTEGER NOT NULL);
             INSERT INTO schema_version (version) VALUES (4);",
        )
        .unwrap();

        let err = run(&conn).unwrap_err();
        assert!(err.to_string().contains("newer than supported"));
    }
}
