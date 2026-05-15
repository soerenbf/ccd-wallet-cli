use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};

pub const ACTIVE_NETWORK_KEY: &str = "active_network";
pub const ACTIVE_SEED_KEY: &str = "active_seed";

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
    fn active_seed_can_be_written_and_read() {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();

        set(&conn, ACTIVE_SEED_KEY, "main_seed").unwrap();

        assert_eq!(
            get(&conn, ACTIVE_SEED_KEY).unwrap(),
            Some("main_seed".to_owned())
        );
    }
}
