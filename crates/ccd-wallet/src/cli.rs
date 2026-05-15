use crate::commands::config::network::NetworkCommand;
use ccd_wallet_core::config;
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
    /// Network configuration commands.
    Network(NetworkCommand),
    /// Seed phrase commands.
    Seed(SeedCommand),
    /// Identity issuance commands.
    Identity(Box<IdentityCommand>),
}

#[derive(Debug, Args)]
pub struct SeedCommand {
    #[command(subcommand)]
    pub command: SeedSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum SeedSubcommand {
    /// Add a password-protected seed phrase.
    Add(SeedAddArgs),
    /// Set the active seed by label.
    Use(SeedUseArgs),
    /// Show a seed phrase after password authentication.
    Show(SeedShowArgs),
    /// Remove a seed phrase.
    Remove(SeedRemoveArgs),
}

#[derive(Debug, Args)]
pub struct SeedAddArgs {
    /// Local label for the seed. Use letters, digits, dash, or underscore only.
    #[arg(value_name = "LABEL")]
    pub label: Option<String>,

    /// Generate a new random 24-word seed phrase instead of prompting for one.
    #[arg(long)]
    pub random: bool,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

#[derive(Debug, Args)]
pub struct SeedUseArgs {
    /// Label of the seed to make active.
    #[arg(value_name = "LABEL")]
    pub label: Option<String>,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

#[derive(Debug, Args)]
pub struct SeedShowArgs {
    /// Label of the seed to show. If omitted, the active seed is used.
    #[arg(value_name = "LABEL")]
    pub label: Option<String>,

    /// Disable silent use of active defaults and force explicit selection.
    #[arg(long = "no-defaults")]
    pub no_defaults: bool,
}

#[derive(Debug, Args)]
pub struct SeedRemoveArgs {
    /// Label of the seed to remove.
    #[arg(value_name = "LABEL")]
    pub label: Option<String>,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

#[derive(Debug, Args)]
pub struct IdentityCommand {
    #[command(subcommand)]
    pub command: IdentitySubcommand,
}

#[derive(Debug, Subcommand)]
pub enum IdentitySubcommand {
    /// Issue a new identity.
    New(IdentityNewArgs),
}

#[derive(Debug, Args)]
pub struct IdentityNewArgs {
    /// Local label for the identity. Use letters, digits, dash, or underscore only.
    #[arg(value_name = "LABEL")]
    pub label: Option<String>,

    /// Identity provider id to use directly.
    #[arg(long, value_name = "ID", conflicts_with = "interactive")]
    pub provider: Option<u32>,

    /// Query the node for available providers and choose interactively.
    #[arg(long, conflicts_with = "provider")]
    pub interactive: bool,

    /// Seed label to use. Defaults to the active seed.
    #[arg(long, value_name = "LABEL")]
    pub seed: Option<String>,

    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub network: Option<String>,

    /// Concordium node gRPC endpoint.
    #[arg(long = "node", value_name = "ENDPOINT")]
    pub node: Option<v2::Endpoint>,

    /// Use manual callback paste instead of the default local browser callback.
    #[arg(long = "manual-callback")]
    pub manual_callback: bool,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive", conflicts_with = "interactive")]
    pub non_interactive: bool,

    /// Disable silent use of active seed/network defaults and force explicit selection.
    #[arg(long = "no-defaults")]
    pub no_defaults: bool,
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

    /// Disable silent use of the active network and force explicit selection.
    #[arg(long = "no-defaults")]
    pub no_defaults: bool,
}
