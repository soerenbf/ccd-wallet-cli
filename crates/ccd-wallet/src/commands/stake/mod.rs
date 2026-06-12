//! Stake command orchestration.

mod configure;
mod remove;
pub(crate) mod shared;
mod show;

use crate::cli::{StakeConfigureSubcommand, StakeSubcommand};
use anyhow::{Result, bail};
use rusqlite::Connection;

/// Run a stake command.
///
/// # Arguments
/// * `conn` - Open wallet store connection.
/// * `command` - Parsed stake subcommand.
///
/// # Errors
/// Returns an error if the selected stake command fails.
pub async fn run(conn: &Connection, command: StakeSubcommand) -> Result<()> {
    match command {
        StakeSubcommand::Show(args) => show::show(conn, *args).await,
        StakeSubcommand::Configure(args) => match args.command {
            StakeConfigureSubcommand::Delegation(args) => {
                configure::configure_delegation(conn, *args).await
            }
            StakeConfigureSubcommand::Validator(_) => {
                bail!(
                    "`stake configure validator` is reserved for future work and is not implemented yet"
                )
            }
        },
        StakeSubcommand::Remove(args) => remove::remove(conn, *args).await,
    }
}
