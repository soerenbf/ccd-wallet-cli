mod cli;
mod commands;
mod config;
mod store;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    init_tracing();

    let cli = Cli::parse();
    let conn = store::db::open()?;

    match cli.command {
        Command::Node(command) => commands::node::run(&conn, command.command).await,
        Command::Network(command) => commands::config::run(&conn, command).await,
        Command::Seed(command) => commands::seed::run(&conn, command.command).await,
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let _ = fmt().with_env_filter(filter).try_init();
}
