use crate::commands::config::network::NetworkCommand;
use ccd_wallet_core::config;
use clap::{Args, Parser, Subcommand};
use concordium_rust_sdk::v2;
use std::path::PathBuf;

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
    Network(Box<NetworkCommand>),
    /// Transaction inspection commands.
    Transaction(Box<TransactionCommand>),
    /// Seed phrase commands.
    Seed(SeedCommand),
    /// Identity issuance commands.
    Identity(Box<IdentityCommand>),
    /// Account creation and management commands.
    Account(Box<AccountCommand>),
    /// Governance key management commands.
    Governance(Box<GovernanceCommand>),
}

#[derive(Debug, Args)]
pub struct TransactionCommand {
    #[command(subcommand)]
    pub command: TransactionSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum TransactionSubcommand {
    /// Show details for a transaction hash.
    Show(Box<TransactionShowArgs>),
}

#[derive(Debug, Args)]
pub struct TransactionShowArgs {
    /// Transaction hash to inspect.
    #[arg(value_name = "HASH")]
    pub hash: concordium_rust_sdk::types::hashes::TransactionHash,

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

#[derive(Debug, Args)]
pub struct SeedCommand {
    #[command(subcommand)]
    pub command: SeedSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum SeedSubcommand {
    /// Add a password-protected seed phrase.
    Add(SeedAddArgs),
    /// Delete a seed phrase.
    Delete(SeedDeleteArgs),
    /// List configured seeds.
    List,
    /// Rename a configured seed.
    Rename(SeedRenameArgs),
    /// Recover identities and accounts for a stored seed.
    Sync(SeedSyncArgs),
    /// Set the active seed by label.
    Use(SeedUseArgs),
    /// Show a seed phrase after password authentication.
    Show(SeedShowArgs),
}

#[derive(Debug, Args)]
pub struct SeedAddArgs {
    /// Local label for the seed. Use letters, digits, dash, or underscore only.
    #[arg(value_name = "LABEL")]
    pub label: Option<String>,

    /// Generate a new random 24-word seed phrase instead of prompting for one.
    #[arg(long)]
    pub random: bool,

    /// Immediately run recovery on the named network after storing the seed.
    #[arg(long, value_name = "NETWORK")]
    pub restore: Option<String>,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

#[derive(Debug, Args)]
pub struct SeedSyncArgs {
    /// Label of the seed to sync. Defaults to the active seed.
    #[arg(value_name = "LABEL")]
    pub label: Option<String>,

    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub network: Option<String>,

    /// Identity provider selection. Repeat for multiple providers, or use `all`.
    #[arg(long = "provider", value_name = "VALUE")]
    pub providers: Vec<String>,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,

    /// Disable silent use of active seed/network defaults and force explicit selection.
    #[arg(long = "no-defaults")]
    pub no_defaults: bool,
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
pub struct SeedDeleteArgs {
    /// Label of the seed to delete.
    #[arg(value_name = "LABEL")]
    pub label: Option<String>,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

#[derive(Debug, Args)]
pub struct SeedRenameArgs {
    /// Existing seed label.
    #[arg(value_name = "OLD_LABEL")]
    pub old_label: Option<String>,

    /// New seed label.
    #[arg(value_name = "NEW_LABEL")]
    pub new_label: Option<String>,

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
    /// List stored identities.
    List(IdentityListArgs),
    /// Issue a new identity.
    New(Box<IdentityNewArgs>),
    /// Rename a stored identity.
    Rename(IdentityRenameArgs),
}

#[derive(Debug, Args)]
pub struct IdentityListArgs {
    /// Seed label to use. Defaults to the active seed.
    #[arg(long, value_name = "LABEL")]
    pub seed: Option<String>,

    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub network: Option<String>,

    /// Filter identities by provider id.
    #[arg(long, value_name = "ID")]
    pub provider: Option<u32>,

    /// Filter identities by status.
    #[arg(long, value_name = "STATUS")]
    pub status: Option<String>,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,

    /// Disable silent use of active seed/network defaults and force explicit selection.
    #[arg(long = "no-defaults")]
    pub no_defaults: bool,
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

    /// Return after receiving the callback code URI without waiting for identity completion.
    #[arg(long = "no-wait")]
    pub no_wait: bool,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive", conflicts_with = "interactive")]
    pub non_interactive: bool,

    /// Disable silent use of active seed/network defaults and force explicit selection.
    #[arg(long = "no-defaults")]
    pub no_defaults: bool,
}

#[derive(Debug, Args)]
pub struct IdentityRenameArgs {
    /// Existing identity label.
    #[arg(value_name = "OLD_LABEL")]
    pub old_label: Option<String>,

    /// New identity label.
    #[arg(value_name = "NEW_LABEL")]
    pub new_label: Option<String>,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

#[derive(Debug, Args)]
pub struct AccountCommand {
    #[command(subcommand)]
    pub command: AccountSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum AccountSubcommand {
    /// Import externally provisioned accounts.
    Import(AccountImportCommand),
    /// List stored accounts.
    List(AccountListArgs),
    /// Create a new account from a stored identity.
    New(Box<AccountNewArgs>),
    /// Rename a stored account.
    Rename(AccountRenameArgs),
}

#[derive(Debug, Args)]
pub struct AccountImportCommand {
    #[command(subcommand)]
    pub command: AccountImportSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum AccountImportSubcommand {
    /// Import a single genesis account JSON file.
    Genesis(AccountImportGenesisArgs),
}

#[derive(Debug, Args)]
pub struct AccountImportGenesisArgs {
    /// Path to a genesis account JSON file.
    #[arg(value_name = "FILE")]
    pub file: PathBuf,

    /// Label for the imported account. If omitted, interactive mode prompts with the file stem as suggestion.
    #[arg(long, value_name = "LABEL")]
    pub label: Option<String>,

    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub network: Option<String>,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,

    /// Disable silent use of active network defaults and force explicit selection.
    #[arg(long = "no-defaults")]
    pub no_defaults: bool,
}

#[derive(Debug, Args)]
pub struct AccountListArgs {
    /// Seed label to use. Defaults to the active seed.
    #[arg(long, value_name = "LABEL")]
    pub seed: Option<String>,

    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub network: Option<String>,

    /// Filter accounts by status.
    #[arg(long, value_name = "STATUS")]
    pub status: Option<String>,

    /// Reveal decrypted account addresses in the output.
    #[arg(long = "show-addresses")]
    pub show_addresses: bool,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,

    /// Disable silent use of active seed/network defaults and force explicit selection.
    #[arg(long = "no-defaults")]
    pub no_defaults: bool,
}

#[derive(Debug, Args)]
pub struct AccountNewArgs {
    /// Local label for the account. Use letters, digits, dash, or underscore only.
    #[arg(value_name = "LABEL")]
    pub label: Option<String>,

    /// Identity label to use. Defaults to an interactive identity selector.
    #[arg(long, value_name = "LABEL")]
    pub identity: Option<String>,

    /// Seed label to use. Defaults to the active seed.
    #[arg(long, value_name = "LABEL")]
    pub seed: Option<String>,

    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub network: Option<String>,

    /// Concordium node gRPC endpoint.
    #[arg(long = "node", value_name = "ENDPOINT")]
    pub node: Option<v2::Endpoint>,

    /// Return after successful submission without waiting for finalization.
    #[arg(long = "no-wait")]
    pub no_wait: bool,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,

    /// Disable silent use of active seed/network defaults and force explicit selection.
    #[arg(long = "no-defaults")]
    pub no_defaults: bool,
}

#[derive(Debug, Args)]
pub struct AccountRenameArgs {
    /// Existing account label.
    #[arg(value_name = "OLD_LABEL")]
    pub old_label: Option<String>,

    /// New account label.
    #[arg(value_name = "NEW_LABEL")]
    pub new_label: Option<String>,

    /// Seed label to use when showing addresses in the selector.
    #[arg(long, value_name = "LABEL")]
    pub seed: Option<String>,

    /// Reveal decrypted account addresses in the selector.
    #[arg(long = "show-addresses")]
    pub show_addresses: bool,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

#[derive(Debug, Args)]
pub struct GovernanceCommand {
    #[command(subcommand)]
    pub command: GovernanceSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum GovernanceSubcommand {
    /// Manage imported governance keys.
    Keys(GovernanceKeysCommand),
    /// Sign and submit a governance update.
    Update(Box<GovernanceUpdateArgs>),
}

#[derive(Debug, Args)]
pub struct GovernanceKeysCommand {
    #[command(subcommand)]
    pub command: GovernanceKeysSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum GovernanceKeysSubcommand {
    /// Import governance key JSON files.
    Import(GovernanceKeysImportArgs),
    /// List imported governance keys using live chain authorization state.
    List(GovernanceKeysListArgs),
    /// Remove imported governance keys.
    Remove(GovernanceKeysRemoveArgs),
}

#[derive(Debug, Args)]
pub struct GovernanceKeysImportArgs {
    /// Path to a single governance key JSON file.
    #[arg(value_name = "FILE", conflicts_with = "dir")]
    pub file: Option<PathBuf>,

    /// Path to a directory of governance key JSON files.
    #[arg(long, value_name = "DIR", conflicts_with = "file")]
    pub dir: Option<PathBuf>,

    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub network: Option<String>,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

#[derive(Debug, Args)]
pub struct GovernanceKeysListArgs {
    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub network: Option<String>,

    /// Disable silent use of active network defaults and force explicit selection.
    #[arg(long = "no-defaults")]
    pub no_defaults: bool,

    /// Show full governance verify keys instead of compact abbreviations.
    #[arg(long = "show-full")]
    pub show_full: bool,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

#[derive(Debug, Args)]
pub struct GovernanceKeysRemoveArgs {
    /// Governance key public key to remove.
    #[arg(value_name = "VERIFY_KEY", conflicts_with = "all")]
    pub verify_key: Option<String>,

    /// Remove all governance keys for the selected network.
    #[arg(long, conflicts_with = "verify_key")]
    pub all: bool,

    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub network: Option<String>,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

#[derive(Debug, Args)]
pub struct GovernanceUpdateArgs {
    /// Read a governance update payload from a JSON file. If no file is supplied, prompt for pasted JSON.
    #[arg(long = "json", value_name = "FILE", num_args = 0..=1, conflicts_with = "serialized")]
    pub json: Option<Option<PathBuf>>,

    /// Use a serialized hex governance update payload. If no hex is supplied, prompt for pasted hex.
    #[arg(long = "serialized", value_name = "HEX", num_args = 0..=1, conflicts_with = "json")]
    pub serialized: Option<Option<String>>,

    /// Allow blind signing when a serialized payload cannot be decoded by this wallet.
    #[arg(long = "blind")]
    pub blind: bool,

    /// Governance verify key to sign with. Repeat to select multiple signers.
    #[arg(long = "key", value_name = "VERIFY_KEY")]
    pub keys: Vec<String>,

    /// Authorization family hint for blind signing, such as protocol, root, level1, or create-plt.
    #[arg(long = "sign-as", value_name = "AUTH_FAMILY")]
    pub sign_as: Option<String>,

    /// Override the update sequence number.
    #[arg(long = "sequence-number", value_name = "N")]
    pub sequence_number: Option<u64>,

    /// Effective time for the update: 0, relative duration, RFC3339, or unix seconds.
    #[arg(long = "effective-time", value_name = "TIME")]
    pub effective_time: Option<String>,

    /// Timeout for the update: relative duration, RFC3339, or unix seconds.
    #[arg(long = "timeout", value_name = "TIME")]
    pub timeout: Option<String>,

    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub network: Option<String>,

    /// Return after successful submission without waiting for finalization.
    #[arg(long = "no-wait")]
    pub no_wait: bool,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,

    /// Disable silent use of active network defaults and force explicit selection.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_transaction_show_command() {
        let cli = Cli::parse_from([
            "ccd-wallet",
            "transaction",
            "show",
            "0fda6e284f9cd4429c6f76fd1bf6179aad4fa1bb218fe5ec8ad33916bf84a833",
            "--network",
            "testnet",
        ]);

        match cli.command {
            Command::Transaction(command) => match command.command {
                TransactionSubcommand::Show(args) => {
                    assert_eq!(args.network.as_deref(), Some("testnet"));
                    assert!(args.node.is_none());
                    assert_eq!(
                        args.hash.to_string(),
                        "0fda6e284f9cd4429c6f76fd1bf6179aad4fa1bb218fe5ec8ad33916bf84a833"
                    );
                }
            },
            other => panic!("expected transaction command, got {other:?}"),
        }
    }
}
