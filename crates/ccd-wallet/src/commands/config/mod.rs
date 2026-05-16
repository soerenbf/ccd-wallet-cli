pub mod network;

use crate::commands::config::network::{NetworkCommand, NetworkSubcommand};
use anyhow::Result;
use rusqlite::Connection;

pub async fn run(conn: &Connection, command: NetworkCommand) -> Result<()> {
    match command.command {
        NetworkSubcommand::Add(args) => network::add(*args).await,
        NetworkSubcommand::Delete(args) => network::delete(conn, args).await,
        NetworkSubcommand::List => network::list(conn).await,
        NetworkSubcommand::Rename(args) => network::rename(conn, args).await,
        NetworkSubcommand::Reset(args) => network::reset(conn, args).await,
        NetworkSubcommand::Show(args) => network::show(conn, *args).await,
        NetworkSubcommand::Use(args) => network::use_network(conn, args).await,
    }
}
