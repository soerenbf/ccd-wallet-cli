pub mod network;

use crate::commands::config::network::{NetworkCommand, NetworkSubcommand};
use anyhow::Result;
use rusqlite::Connection;

pub async fn run(conn: &Connection, command: NetworkCommand) -> Result<()> {
    match command.command {
        NetworkSubcommand::Add(args) => network::add(*args).await,
        NetworkSubcommand::Use(args) => network::use_network(conn, args).await,
    }
}
