use crate::commands::config::network::NetworkCommand;
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
    Node(Box<NodeCommand>),
    /// Configuration commands.
    Config(ConfigCommand),
}

#[derive(Debug, Args)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub command: ConfigSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ConfigSubcommand {
    /// Manage network configurations.
    Network(NetworkCommand),
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
    /// Registered network name to resolve from the config store.
    #[arg(long = "network", conflicts_with = "node", value_name = "NAME")]
    pub network: Option<String>,

    /// Concordium node gRPC endpoint.
    #[arg(
        long = "node",
        env = config::NODE_ENDPOINT_ENV,
        conflicts_with = "network",
        value_name = "ENDPOINT"
    )]
    pub node: Option<v2::Endpoint>,
}
