use crate::store::migrations;
use anyhow::{Context, Result, bail};
use rusqlite::Connection;
use std::path::PathBuf;

pub const DB_PATH_ENV: &str = "CCD_WALLET_DB_PATH";

/// Returns the wallet SQLite database path.
///
/// Defaults to `{data_dir}/ccd-wallet/wallet.db`, using the OS-specific
/// application data directory. Can be overridden with `CCD_WALLET_DB_PATH`.
pub fn db_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os(DB_PATH_ENV) {
        let path = PathBuf::from(path);
        if !path.is_absolute() {
            bail!("{DB_PATH_ENV} must be an absolute path");
        }
        return Ok(path);
    }

    let data_dir = dirs::data_dir()
        .context("could not determine application data directory for wallet database")?;

    Ok(data_dir.join("ccd-wallet").join("wallet.db"))
}

/// Open the wallet database, creating parent directories and applying pending
/// schema migrations before returning the connection.
pub fn open() -> Result<Connection> {
    let path = db_path()?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create wallet database directory at {}",
                parent.display()
            )
        })?;
    }

    let conn = Connection::open(&path)
        .with_context(|| format!("failed to open wallet database at {}", path.display()))?;
    migrations::run(&conn)?;
    Ok(conn)
}
