//! Generic wallet-local key/value state storage.

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};

/// Wallet-state key storing the active network label.
pub const ACTIVE_NETWORK_KEY: &str = "active_network";
/// Wallet-state key storing the active user-facing key source label.
pub const ACTIVE_KEY_SOURCE_KEY: &str = "active_key_source";
/// Compatibility alias used by existing seed commands while active key-source UX is introduced.
pub const ACTIVE_SEED_KEY: &str = ACTIVE_KEY_SOURCE_KEY;

/// Read a wallet-state value by key.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `key` - Wallet-state key.
///
/// # Returns
/// The stored value when present.
///
/// # Errors
/// Returns an error if reading the key fails.
///
/// # Examples
///
/// ```ignore
/// let active = wallet_state::get(&conn, wallet_state::ACTIVE_KEY_SOURCE_KEY)?;
/// ```
pub fn get(conn: &Connection, key: &str) -> Result<Option<String>> {
    conn.query_row(
        "SELECT value FROM wallet_state WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .optional()
    .with_context(|| format!("failed to read wallet state key '{key}'"))
}

/// Store or replace a wallet-state value.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `key` - Wallet-state key.
/// * `value` - Value to store.
///
/// # Errors
/// Returns an error if writing the key fails.
///
/// # Examples
///
/// ```ignore
/// wallet_state::set(&conn, wallet_state::ACTIVE_KEY_SOURCE_KEY, "main_seed")?;
/// ```
pub fn set(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO wallet_state (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .with_context(|| format!("failed to write wallet state key '{key}'"))?;

    Ok(())
}

/// Remove a wallet-state key.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `key` - Wallet-state key.
///
/// # Errors
/// Returns an error if deleting the key fails.
///
/// # Examples
///
/// ```ignore
/// wallet_state::remove(&conn, wallet_state::ACTIVE_KEY_SOURCE_KEY)?;
/// ```
pub fn remove(conn: &Connection, key: &str) -> Result<()> {
    conn.execute("DELETE FROM wallet_state WHERE key = ?1", params![key])
        .with_context(|| format!("failed to delete wallet state key '{key}'"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::migrations;

    #[test]
    fn active_key_source_can_be_written_and_read_through_seed_alias() {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();

        set(&conn, ACTIVE_SEED_KEY, "main_seed").unwrap();

        assert_eq!(
            get(&conn, ACTIVE_KEY_SOURCE_KEY).unwrap(),
            Some("main_seed".to_owned())
        );
    }
}
