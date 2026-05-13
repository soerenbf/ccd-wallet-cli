pub mod network;

use crate::commands::config::network::{NetworkCommand, NetworkSubcommand};
use anyhow::Result;

pub async fn run(command: NetworkCommand) -> Result<()> {
    match command.command {
        NetworkSubcommand::Add(args) => network::add(args).await,
    }
}
