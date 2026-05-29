use crate::commands::config::network::NetworkCommand;
use ccd_wallet_core::config;
use clap::{Args, Parser, Subcommand};
use concordium_rust_sdk::v2;
use std::{net::SocketAddr, path::PathBuf};

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
    /// Smart contract transaction commands.
    Contract(Box<ContractCommand>),
    /// Seed phrase commands.
    Seed(SeedCommand),
    /// Identity issuance commands.
    Identity(Box<IdentityCommand>),
    /// Account creation and management commands.
    Account(Box<AccountCommand>),
    /// Governance key management commands.
    Governance(Box<GovernanceCommand>),
    /// Start a temporary browser pairing session.
    Connect(ConnectArgs),
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
pub struct ContractCommand {
    #[command(subcommand)]
    pub command: ContractSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ContractSubcommand {
    /// Deploy a smart contract module from a local module file.
    DeployModule(Box<ContractDeployModuleArgs>),
    /// Initialize a smart contract instance.
    Init(Box<ContractInitArgs>),
    /// Update a smart contract instance by invoking a receive function.
    Update(Box<ContractUpdateArgs>),
    /// Invoke a smart contract entrypoint without submitting a transaction.
    Invoke(Box<ContractInvokeArgs>),
    /// Show smart contract instance information.
    Show(Box<ContractShowArgs>),
    /// Print a JSON parameter template from an embedded module schema.
    ParameterTemplate(Box<ContractParameterTemplateArgs>),
    /// Download smart contract module source bytes.
    DownloadModule(Box<ContractDownloadModuleArgs>),
}

#[derive(Debug, Args)]
pub struct ContractInitArgs {
    /// Module reference to initialize from.
    #[arg(long = "module-ref", value_name = "REF")]
    pub module_ref: String,

    /// Init function name, for example `init_counter`.
    #[arg(long = "init-name", value_name = "NAME")]
    pub init_name: String,

    /// CCD amount to transfer to the new instance, as a decimal value.
    #[arg(long = "amount", value_name = "CCD")]
    pub amount: Option<String>,

    /// Maximum contract execution energy. If omitted interactively, the CLI prompts with a simulation-derived default when available.
    #[arg(long = "energy", value_name = "ENERGY")]
    pub energy: Option<u64>,

    /// Serialized parameter bytes encoded as hex.
    #[arg(long = "parameter-hex", conflicts_with_all = ["parameter_json", "parameter_json_file"], value_name = "HEX")]
    pub parameter_hex: Option<String>,

    /// Parameter JSON string encoded using the embedded module schema.
    #[arg(long = "parameter-json", conflicts_with_all = ["parameter_hex", "parameter_json_file"], value_name = "JSON")]
    pub parameter_json: Option<String>,

    /// Path to a parameter JSON file encoded using the embedded module schema.
    #[arg(long = "parameter-json-file", conflicts_with_all = ["parameter_hex", "parameter_json"], value_name = "FILE")]
    pub parameter_json_file: Option<PathBuf>,

    /// Run a simulation before approval and show the result.
    #[arg(long = "validate")]
    pub validate: bool,

    /// Account label to sign with. If omitted, interactive mode opens an account selector.
    #[arg(long = "account", value_name = "LABEL")]
    pub account: Option<String>,

    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub network: Option<String>,

    /// Concordium node gRPC endpoint.
    #[arg(long = "node", env = config::NODE_ENDPOINT_ENV, value_name = "ENDPOINT")]
    pub node: Option<v2::Endpoint>,

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
pub struct ContractUpdateArgs {
    /// Contract instance address as `<index,subindex>`, `index,subindex`, or `index`.
    #[arg(long = "contract", value_name = "ADDRESS")]
    pub contract: String,

    /// Fully-qualified receive function name, for example `counter.increment`.
    #[arg(long = "receive", value_name = "NAME")]
    pub receive: String,

    /// CCD amount to transfer to the instance, as a decimal value.
    #[arg(long = "amount", value_name = "CCD")]
    pub amount: Option<String>,

    /// Maximum contract execution energy. If omitted interactively, the CLI prompts with a simulation-derived default when available.
    #[arg(long = "energy", value_name = "ENERGY")]
    pub energy: Option<u64>,

    /// Serialized parameter bytes encoded as hex.
    #[arg(long = "parameter-hex", conflicts_with_all = ["parameter_json", "parameter_json_file"], value_name = "HEX")]
    pub parameter_hex: Option<String>,

    /// Parameter JSON string encoded using the embedded module schema.
    #[arg(long = "parameter-json", conflicts_with_all = ["parameter_hex", "parameter_json_file"], value_name = "JSON")]
    pub parameter_json: Option<String>,

    /// Path to a parameter JSON file encoded using the embedded module schema.
    #[arg(long = "parameter-json-file", conflicts_with_all = ["parameter_hex", "parameter_json"], value_name = "FILE")]
    pub parameter_json_file: Option<PathBuf>,

    /// Run a simulation before approval and show the result.
    #[arg(long = "validate")]
    pub validate: bool,

    /// Account label to sign with. If omitted, interactive mode opens an account selector.
    #[arg(long = "account", value_name = "LABEL")]
    pub account: Option<String>,

    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub network: Option<String>,

    /// Concordium node gRPC endpoint.
    #[arg(long = "node", env = config::NODE_ENDPOINT_ENV, value_name = "ENDPOINT")]
    pub node: Option<v2::Endpoint>,

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
pub struct ContractInvokeArgs {
    /// Contract instance address as `<index,subindex>`, `index,subindex`, or `index`.
    #[arg(long = "contract", value_name = "ADDRESS")]
    pub contract: String,

    /// Fully-qualified receive function name, for example `counter.view`.
    #[arg(long = "receive", value_name = "NAME")]
    pub receive: String,

    /// CCD amount to use for the invocation context, as a decimal value.
    #[arg(long = "amount", value_name = "CCD")]
    pub amount: Option<String>,

    /// Maximum contract execution energy for the query.
    #[arg(long = "energy", value_name = "ENERGY")]
    pub energy: Option<u64>,

    /// Account address to use as explicit invoker. Defaults to the node's synthetic zero-account context.
    #[arg(long = "invoker", value_name = "ADDRESS")]
    pub invoker: Option<String>,

    /// Serialized parameter bytes encoded as hex.
    #[arg(long = "parameter-hex", conflicts_with_all = ["parameter_json", "parameter_json_file"], value_name = "HEX")]
    pub parameter_hex: Option<String>,

    /// Parameter JSON string encoded using the embedded module schema.
    #[arg(long = "parameter-json", conflicts_with_all = ["parameter_hex", "parameter_json_file"], value_name = "JSON")]
    pub parameter_json: Option<String>,

    /// Path to a parameter JSON file encoded using the embedded module schema.
    #[arg(long = "parameter-json-file", conflicts_with_all = ["parameter_hex", "parameter_json"], value_name = "FILE")]
    pub parameter_json_file: Option<PathBuf>,

    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub network: Option<String>,

    /// Concordium node gRPC endpoint.
    #[arg(long = "node", env = config::NODE_ENDPOINT_ENV, value_name = "ENDPOINT")]
    pub node: Option<v2::Endpoint>,

    /// Query block selector. Supports `best` and `last-final`.
    #[arg(long = "block", value_name = "BLOCK")]
    pub block: Option<String>,

    /// Print machine-readable JSON output.
    #[arg(long = "json")]
    pub json: bool,

    /// Disable silent use of active network defaults and force explicit selection.
    #[arg(long = "no-defaults")]
    pub no_defaults: bool,
}

#[derive(Debug, Args)]
pub struct ContractShowArgs {
    /// Contract instance address as `<index,subindex>`, `index,subindex`, or `index`.
    #[arg(long = "contract", value_name = "ADDRESS")]
    pub contract: String,

    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub network: Option<String>,

    /// Concordium node gRPC endpoint.
    #[arg(long = "node", env = config::NODE_ENDPOINT_ENV, value_name = "ENDPOINT")]
    pub node: Option<v2::Endpoint>,

    /// Query block selector. Supports `best` and `last-final`.
    #[arg(long = "block", value_name = "BLOCK")]
    pub block: Option<String>,

    /// Print machine-readable JSON output.
    #[arg(long = "json")]
    pub json: bool,

    /// Disable silent use of active network defaults and force explicit selection.
    #[arg(long = "no-defaults")]
    pub no_defaults: bool,
}

#[derive(Debug, Args)]
pub struct ContractParameterTemplateArgs {
    #[command(subcommand)]
    pub command: ContractParameterTemplateSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ContractParameterTemplateSubcommand {
    /// Print an init parameter JSON template.
    Init(Box<ContractParameterTemplateInitArgs>),
    /// Print a receive parameter JSON template.
    Receive(Box<ContractParameterTemplateReceiveArgs>),
}

#[derive(Debug, Args)]
pub struct ContractParameterTemplateInitArgs {
    /// Init function name, for example `init_counter`.
    #[arg(value_name = "INIT_NAME")]
    pub init_name: String,

    /// Module reference that contains the embedded schema.
    #[arg(long = "module-ref", value_name = "REF")]
    pub module_ref: String,

    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub network: Option<String>,

    /// Concordium node gRPC endpoint.
    #[arg(long = "node", env = config::NODE_ENDPOINT_ENV, value_name = "ENDPOINT")]
    pub node: Option<v2::Endpoint>,

    /// Query block selector. Supports `best` and `last-final`.
    #[arg(long = "block", value_name = "BLOCK")]
    pub block: Option<String>,

    /// Disable silent use of active network defaults and force explicit selection.
    #[arg(long = "no-defaults")]
    pub no_defaults: bool,
}

#[derive(Debug, Args)]
pub struct ContractParameterTemplateReceiveArgs {
    /// Fully-qualified receive function name, for example `counter.increment`.
    #[arg(value_name = "RECEIVE_NAME")]
    pub receive: String,

    /// Contract instance address to resolve the source module from.
    #[arg(
        long = "contract",
        conflicts_with = "module_ref",
        value_name = "ADDRESS"
    )]
    pub contract: Option<String>,

    /// Module reference that contains the embedded schema.
    #[arg(long = "module-ref", conflicts_with = "contract", value_name = "REF")]
    pub module_ref: Option<String>,

    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub network: Option<String>,

    /// Concordium node gRPC endpoint.
    #[arg(long = "node", env = config::NODE_ENDPOINT_ENV, value_name = "ENDPOINT")]
    pub node: Option<v2::Endpoint>,

    /// Query block selector. Supports `best` and `last-final`.
    #[arg(long = "block", value_name = "BLOCK")]
    pub block: Option<String>,

    /// Disable silent use of active network defaults and force explicit selection.
    #[arg(long = "no-defaults")]
    pub no_defaults: bool,
}

#[derive(Debug, Args)]
pub struct ContractDownloadModuleArgs {
    /// Module reference to download. Omit when using `--contract`.
    #[arg(value_name = "MODULE_REF", conflicts_with = "contract")]
    pub module_ref: Option<String>,

    /// Contract instance address to resolve the source module from.
    #[arg(
        long = "contract",
        conflicts_with = "module_ref",
        value_name = "ADDRESS"
    )]
    pub contract: Option<String>,

    /// Output file path.
    #[arg(long = "out", value_name = "FILE")]
    pub out: PathBuf,

    /// Overwrite the output file if it already exists.
    #[arg(long = "force")]
    pub force: bool,

    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub network: Option<String>,

    /// Concordium node gRPC endpoint.
    #[arg(long = "node", env = config::NODE_ENDPOINT_ENV, value_name = "ENDPOINT")]
    pub node: Option<v2::Endpoint>,

    /// Query block selector. Supports `best` and `last-final`.
    #[arg(long = "block", value_name = "BLOCK")]
    pub block: Option<String>,

    /// Disable silent use of active network defaults and force explicit selection.
    #[arg(long = "no-defaults")]
    pub no_defaults: bool,
}

#[derive(Debug, Args)]
pub struct ContractDeployModuleArgs {
    /// Path to the Concordium smart contract module file to deploy.
    #[arg(value_name = "FILE")]
    pub file: PathBuf,

    /// Account label to sign with. If omitted, interactive mode opens an account selector.
    #[arg(long = "account", value_name = "LABEL")]
    pub account: Option<String>,

    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub network: Option<String>,

    /// Concordium node gRPC endpoint.
    #[arg(long = "node", env = config::NODE_ENDPOINT_ENV, value_name = "ENDPOINT")]
    pub node: Option<v2::Endpoint>,

    /// Skip the default check for whether the derived module reference already exists before approval.
    #[arg(long = "no-validate")]
    pub no_validate: bool,

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
    /// Export a stored account as a JSON signer file.
    Export(AccountExportArgs),
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
pub struct AccountExportArgs {
    /// Account label to export. If omitted, interactive mode opens an account selector.
    #[arg(value_name = "LABEL")]
    pub label: Option<String>,

    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub network: Option<String>,

    /// Path to write the exported signer JSON file.
    #[arg(long = "out", value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,

    /// Disable silent use of active network defaults and force explicit selection.
    #[arg(long = "no-defaults")]
    pub no_defaults: bool,
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
pub struct ConnectArgs {
    /// Local WebSocket address for browser pairing.
    #[arg(long = "bind", default_value = "127.0.0.1:22771", value_name = "ADDR")]
    pub bind: SocketAddr,
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

    #[test]
    fn parses_contract_deploy_module_command() {
        let cli = Cli::parse_from([
            "ccd-wallet",
            "contract",
            "deploy-module",
            "contract.wasm.v1",
            "--account",
            "alice",
            "--network",
            "testnet",
            "--no-validate",
            "--no-wait",
        ]);

        match cli.command {
            Command::Contract(command) => match command.command {
                ContractSubcommand::DeployModule(args) => {
                    assert_eq!(args.file, std::path::PathBuf::from("contract.wasm.v1"));
                    assert_eq!(args.account.as_deref(), Some("alice"));
                    assert_eq!(args.network.as_deref(), Some("testnet"));
                    assert!(args.no_validate);
                    assert!(args.no_wait);
                }
                other => panic!("expected contract deploy-module command, got {other:?}"),
            },
            other => panic!("expected contract command, got {other:?}"),
        }
    }

    #[test]
    fn parses_contract_init_command_with_json_file_and_optional_energy() {
        let cli = Cli::parse_from([
            "ccd-wallet",
            "contract",
            "init",
            "--module-ref",
            "0000000000000000000000000000000000000000000000000000000000000000",
            "--init-name",
            "init_counter",
            "--amount",
            "1.25",
            "--parameter-json-file",
            "init.json",
            "--account",
            "alice",
        ]);

        match cli.command {
            Command::Contract(command) => match command.command {
                ContractSubcommand::Init(args) => {
                    assert_eq!(args.init_name, "init_counter");
                    assert_eq!(args.amount.as_deref(), Some("1.25"));
                    assert!(args.energy.is_none());
                    assert_eq!(
                        args.parameter_json_file.as_deref(),
                        Some(std::path::Path::new("init.json"))
                    );
                    assert_eq!(args.account.as_deref(), Some("alice"));
                }
                other => panic!("expected contract init command, got {other:?}"),
            },
            other => panic!("expected contract command, got {other:?}"),
        }
    }

    #[test]
    fn parses_contract_update_command_with_inline_json() {
        let cli = Cli::parse_from([
            "ccd-wallet",
            "contract",
            "update",
            "--contract",
            "42,0",
            "--receive",
            "counter.increment",
            "--energy",
            "30000",
            "--parameter-json",
            "{\"delta\":1}",
        ]);

        match cli.command {
            Command::Contract(command) => match command.command {
                ContractSubcommand::Update(args) => {
                    assert_eq!(args.contract, "42,0");
                    assert_eq!(args.receive, "counter.increment");
                    assert_eq!(args.energy, Some(30000));
                    assert_eq!(args.parameter_json.as_deref(), Some("{\"delta\":1}"));
                }
                other => panic!("expected contract update command, got {other:?}"),
            },
            other => panic!("expected contract command, got {other:?}"),
        }
    }

    #[test]
    fn parses_contract_invoke_command() {
        let cli = Cli::parse_from([
            "ccd-wallet",
            "contract",
            "invoke",
            "--contract",
            "42,0",
            "--receive",
            "counter.view",
            "--invoker",
            "3ZfgcQpwewR6Lw7fn6QW3D5nm3mtAWNQKCPUSmWoEUL5H3qE3r",
            "--json",
        ]);

        match cli.command {
            Command::Contract(command) => match command.command {
                ContractSubcommand::Invoke(args) => {
                    assert_eq!(args.contract, "42,0");
                    assert_eq!(args.receive, "counter.view");
                    assert!(args.json);
                    assert!(args.invoker.is_some());
                }
                other => panic!("expected contract invoke command, got {other:?}"),
            },
            other => panic!("expected contract command, got {other:?}"),
        }
    }

    #[test]
    fn parses_contract_parameter_template_init_command() {
        let cli = Cli::parse_from([
            "ccd-wallet",
            "contract",
            "parameter-template",
            "init",
            "init_counter",
            "--module-ref",
            "0000000000000000000000000000000000000000000000000000000000000000",
        ]);

        match cli.command {
            Command::Contract(command) => match command.command {
                ContractSubcommand::ParameterTemplate(args) => match args.command {
                    ContractParameterTemplateSubcommand::Init(args) => {
                        assert_eq!(args.init_name, "init_counter");
                        assert_eq!(
                            args.module_ref,
                            "0000000000000000000000000000000000000000000000000000000000000000"
                        );
                    }
                    other => panic!("expected init parameter-template command, got {other:?}"),
                },
                other => panic!("expected contract parameter-template command, got {other:?}"),
            },
            other => panic!("expected contract command, got {other:?}"),
        }
    }

    #[test]
    fn parses_contract_parameter_template_receive_command() {
        let cli = Cli::parse_from([
            "ccd-wallet",
            "contract",
            "parameter-template",
            "receive",
            "counter.increment",
            "--contract",
            "42,0",
        ]);

        match cli.command {
            Command::Contract(command) => match command.command {
                ContractSubcommand::ParameterTemplate(args) => match args.command {
                    ContractParameterTemplateSubcommand::Receive(args) => {
                        assert_eq!(args.contract.as_deref(), Some("42,0"));
                        assert_eq!(args.receive, "counter.increment");
                    }
                    other => panic!("expected receive parameter-template command, got {other:?}"),
                },
                other => panic!("expected contract parameter-template command, got {other:?}"),
            },
            other => panic!("expected contract command, got {other:?}"),
        }
    }

    #[test]
    fn parses_contract_download_module_command() {
        let cli = Cli::parse_from([
            "ccd-wallet",
            "contract",
            "download-module",
            "0000000000000000000000000000000000000000000000000000000000000000",
            "--out",
            "module.wasm.v1",
        ]);

        match cli.command {
            Command::Contract(command) => match command.command {
                ContractSubcommand::DownloadModule(args) => {
                    assert!(args.module_ref.is_some());
                    assert_eq!(args.out, std::path::PathBuf::from("module.wasm.v1"));
                }
                other => panic!("expected contract download-module command, got {other:?}"),
            },
            other => panic!("expected contract command, got {other:?}"),
        }
    }

    #[test]
    fn parses_account_export_command() {
        let cli = Cli::parse_from([
            "ccd-wallet",
            "account",
            "export",
            "alice",
            "--network",
            "testnet",
            "--out",
            "alice.json",
            "--non-interactive",
        ]);

        match cli.command {
            Command::Account(command) => match command.command {
                AccountSubcommand::Export(args) => {
                    assert_eq!(args.label.as_deref(), Some("alice"));
                    assert_eq!(args.network.as_deref(), Some("testnet"));
                    assert_eq!(
                        args.output.as_deref(),
                        Some(std::path::Path::new("alice.json"))
                    );
                    assert!(args.non_interactive);
                }
                other => panic!("expected account export command, got {other:?}"),
            },
            other => panic!("expected account command, got {other:?}"),
        }
    }
}
