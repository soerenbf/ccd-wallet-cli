use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};

pub const ACTIVE_NETWORK_KEY: &str = "active_network";

pub fn get(conn: &Connection, key: &str) -> Result<Option<String>> {
    conn.query_row(
        "SELECT value FROM wallet_state WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .optional()
    .with_context(|| format!("failed to read wallet state key '{key}'"))
}

pub fn set(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO wallet_state (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .with_context(|| format!("failed to write wallet state key '{key}'"))?;

    Ok(())
}
