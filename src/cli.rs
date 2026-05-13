use crate::config;
use clap::{Args, Parser, Subcommand};
use concordium_rust_sdk::v2;

#[derive(Debug, Parser)]
#[command(
    name = "ccd-wallet",
    version,
    about = "Concordium wallet command-line interface"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Node-related commands.
    Node(NodeCommand),
}

#[derive(Debug, Args)]
pub struct NodeCommand {
    #[command(subcommand)]
    pub command: NodeSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum NodeSubcommand {
    /// Retrieve information from a Concordium node.
    Info(NodeInfoArgs),
}

#[derive(Debug, Args)]
pub struct NodeInfoArgs {
    /// Concordium node gRPC endpoint.
    #[arg(
        long = "node",
        env = config::NODE_ENDPOINT_ENV,
        default_value = config::DEFAULT_NODE_ENDPOINT,
        value_name = "ENDPOINT"
    )]
    pub node: v2::Endpoint,
}
