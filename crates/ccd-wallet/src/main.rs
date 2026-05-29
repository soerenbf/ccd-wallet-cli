mod cli;
mod commands;
mod smart_contracts;

use anyhow::Result;
use ccd_wallet_core::store;
use clap::Parser;
use cli::{Cli, Command};
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    init_tracing();

    let cli = Cli::parse();
    let mut conn = store::db::open()?;

    match cli.command {
        Command::Node(command) => commands::node::run(&conn, command.command).await,
        Command::Network(command) => commands::config::run(&conn, *command).await,
        Command::Transaction(command) => commands::transaction::run(&conn, command.command).await,
        Command::Contract(command) => commands::contract::run(&conn, command.command).await,
        Command::Seed(command) => commands::seed::run(&mut conn, command.command).await,
        Command::Identity(command) => commands::identity::run(&mut conn, command.command).await,
        Command::Account(command) => commands::account::run(&mut conn, command.command).await,
        Command::Governance(command) => commands::governance::run(&mut conn, command.command).await,
        Command::Connect(args) => commands::connect::run(conn, args).await,
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let _ = fmt().with_env_filter(filter).try_init();
}
