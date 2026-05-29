//! Transaction command orchestration.

pub(crate) mod render;
mod show;

use crate::cli::TransactionSubcommand;
use anyhow::Result;
use rusqlite::Connection;

pub async fn run(conn: &Connection, command: TransactionSubcommand) -> Result<()> {
    match command {
        TransactionSubcommand::Show(args) => show::show(conn, *args).await,
    }
}
