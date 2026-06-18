//! Native CCD command orchestration.

mod schedule;
mod shared;
mod transfer;

use crate::cli::CcdSubcommand;
use anyhow::Result;
use rusqlite::Connection;

/// Run a native CCD command.
pub async fn run(conn: &Connection, command: CcdSubcommand) -> Result<()> {
    match command {
        CcdSubcommand::Transfer(args) => transfer::run(conn, *args).await,
        CcdSubcommand::Schedule(args) => schedule::run(conn, *args).await,
    }
}
