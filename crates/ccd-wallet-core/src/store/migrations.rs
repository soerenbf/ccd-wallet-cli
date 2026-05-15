use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};

pub const CURRENT_SCHEMA_VERSION: u32 = 4;

const MIGRATION_1: &str = include_str!("migrations/001_initial_schema.sql");
const MIGRATION_2: &str = include_str!("migrations/002_seed_vault_cascade.sql");
const MIGRATION_3: &str = include_str!("migrations/003_identities.sql");
const MIGRATION_4: &str = include_str!("migrations/004_identities_network_scope.sql");

pub const MIGRATIONS: &[(u32, &str)] = &[
    (1, MIGRATION_1),
    (2, MIGRATION_2),
    (3, MIGRATION_3),
    (4, MIGRATION_4),
];

pub fn run(conn: &Connection) -> Result<()> {
    let mut version = current_version(conn)?;

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

    fn foreign_key_delete_action(conn: &Connection) -> String {
        conn.query_row("PRAGMA foreign_key_list(seed_vaults)", [], |row| row.get(6))
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

        assert_eq!(foreign_key_delete_action(&conn), "CASCADE");
    }

    #[test]
    fn run_migrates_version_one_database_to_version_four() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(MIGRATION_1).unwrap();
        conn.execute(
            "INSERT INTO seeds (id, label, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params!["seed-id", "main_seed", 1, 1],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO seed_vaults (
                seed_id, kdf_algorithm, kdf_params_json, salt, encrypted_dek, dek_nonce,
                cipher_version, payload_ciphertext, payload_nonce
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                "seed-id",
                "argon2id",
                "{}",
                vec![0u8; 16],
                vec![1u8; 48],
                vec![2u8; 12],
                1,
                vec![3u8; 48],
                vec![4u8; 12],
            ],
        )
        .unwrap();

        run(&conn).unwrap();

        let version: u32 = conn
            .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(version, 4);
        assert_eq!(foreign_key_delete_action(&conn), "CASCADE");

        let vault_count: u32 = conn
            .query_row("SELECT COUNT(*) FROM seed_vaults", [], |row| row.get(0))
            .unwrap();
        assert_eq!(vault_count, 1);

        let identity_count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'identities'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(identity_count, 1);
    }
}
