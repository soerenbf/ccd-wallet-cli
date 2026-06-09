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
    /// Ledger hardware-wallet setup commands.
    Ledger(LedgerCommand),
    /// Identity issuance commands.
    Identity(Box<IdentityCommand>),
    /// Account creation and management commands.
    Account(Box<AccountCommand>),
    /// Governance key management commands.
    Governance(Box<GovernanceCommand>),
    /// Protocol-level token commands.
    Token(Box<TokenCommand>),
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

    /// Show the original submitted transaction payload when it can be retrieved from block contents.
    #[arg(long = "show-payload")]
    pub show_payload: bool,
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

    /// Account address or finalized local account label to use as explicit invoker. Defaults to the node's synthetic zero-account context.
    #[arg(long = "invoker", value_name = "ADDRESS_OR_LABEL")]
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
pub struct LedgerCommand {
    #[command(subcommand)]
    pub command: LedgerSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum LedgerSubcommand {
    /// Enroll a Ledger device as a wallet key source.
    Setup(LedgerSetupArgs),
    /// Recover identities and accounts for an enrolled Ledger key source.
    Sync(LedgerSyncArgs),
    /// Show connected Concordium Ledger app information.
    Show,
}

#[derive(Debug, Args)]
pub struct LedgerSetupArgs {
    /// Local key-source label for the Ledger device.
    #[arg(value_name = "LABEL")]
    pub label: Option<String>,

    /// Immediately run recovery on the named network after enrolling the Ledger.
    #[arg(long, value_name = "NETWORK")]
    pub restore: Option<String>,

    /// Explicitly allow recovery-critical Ledger secret export in non-interactive flows.
    #[arg(long = "allow-ledger-secret-export")]
    pub allow_ledger_secret_export: bool,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

#[derive(Debug, Args)]
pub struct LedgerSyncArgs {
    /// Label of the Ledger key source to sync.
    #[arg(value_name = "LABEL")]
    pub label: Option<String>,

    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub network: Option<String>,

    /// Identity provider selection. Repeat for multiple providers, or use `all`.
    #[arg(long = "provider", value_name = "VALUE")]
    pub providers: Vec<String>,

    /// Explicitly allow recovery-critical Ledger secret export in non-interactive flows.
    #[arg(long = "allow-ledger-secret-export")]
    pub allow_ledger_secret_export: bool,

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
    /// Key-source label to use. Defaults to the active key source.
    #[arg(long = "seed", alias = "key-source", value_name = "LABEL")]
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

    /// Key-source label to use. Defaults to the active key source.
    #[arg(long = "seed", alias = "key-source", value_name = "LABEL")]
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

    /// Explicitly allow Ledger identity issuance secrets to be exported temporarily in non-interactive mode.
    #[arg(long = "allow-ledger-secret-export")]
    pub allow_ledger_secret_export: bool,

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
    /// Show on-chain state for a local account label or account address.
    Show(Box<AccountShowArgs>),
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
    /// Key-source label to use. Defaults to the active key source.
    #[arg(long = "seed", alias = "key-source", value_name = "LABEL")]
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
pub struct AccountShowArgs {
    /// Local account label or Concordium account address to inspect.
    #[arg(value_name = "ACCOUNT")]
    pub account: String,

    /// Registered network name to resolve from the config store.
    #[arg(long = "network", value_name = "NAME")]
    pub network: Option<String>,

    /// Concordium node gRPC endpoint.
    #[arg(long = "node", env = config::NODE_ENDPOINT_ENV, value_name = "ENDPOINT")]
    pub node: Option<v2::Endpoint>,

    /// Block selector to query. Defaults to the last finalized block.
    #[arg(long = "block", value_name = "BLOCK")]
    pub block: Option<String>,

    /// Emit machine-readable JSON output.
    #[arg(long = "json")]
    pub json: bool,

    /// Include low-level protocol details such as nonce and account index.
    #[arg(long = "verbose")]
    pub verbose: bool,

    /// Disable silent use of active network defaults and force explicit selection.
    #[arg(long = "no-defaults")]
    pub no_defaults: bool,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

#[derive(Debug, Args)]
pub struct AccountNewArgs {
    /// Local label for the account. Use letters, digits, dash, or underscore only.
    #[arg(value_name = "LABEL")]
    pub label: Option<String>,

    /// Identity label to use. Defaults to an interactive identity selector.
    #[arg(long, value_name = "LABEL")]
    pub identity: Option<String>,

    /// Key-source label to use. Defaults to the active key source.
    #[arg(long = "seed", alias = "key-source", value_name = "LABEL")]
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

    /// Key-source label to use when showing addresses in the selector.
    #[arg(long = "seed", alias = "key-source", value_name = "LABEL")]
    pub seed: Option<String>,

    /// Reveal decrypted account addresses in the selector.
    #[arg(long = "show-addresses")]
    pub show_addresses: bool,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

#[derive(Debug, Args)]
pub struct TokenCommand {
    #[command(subcommand)]
    pub command: TokenSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum TokenSubcommand {
    /// Show protocol-level token information.
    Show(Box<TokenShowArgs>),
    /// Transfer protocol-level tokens.
    Transfer(Box<TokenTransferArgs>),
    /// Mint protocol-level tokens.
    Mint(Box<TokenAmountArgs>),
    /// Burn protocol-level tokens.
    Burn(Box<TokenAmountArgs>),
    /// Manage token allow-list entries.
    AllowList(Box<TokenListCommand>),
    /// Manage token deny-list entries.
    DenyList(Box<TokenListCommand>),
    /// Pause balance-changing token operations.
    Pause(Box<TokenPauseArgs>),
    /// Resume balance-changing token operations.
    Unpause(Box<TokenPauseArgs>),
    /// Manage token admin roles.
    AdminRoles(Box<TokenAdminRolesCommand>),
    /// Update token metadata.
    Metadata(Box<TokenMetadataCommand>),
    /// Manage token locks.
    Lock(Box<TokenLockCommand>),
}

#[derive(Debug, Args)]
pub struct TokenShowArgs {
    /// Token identifier to inspect.
    #[arg(value_name = "TOKEN_ID")]
    pub token_id: concordium_rust_sdk::protocol_level_tokens::TokenId,

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

    /// Disable silent use of active network defaults and force explicit selection.
    #[arg(long = "no-defaults")]
    pub no_defaults: bool,
}

#[derive(Debug, Args)]
pub struct TokenTransferArgs {
    /// Token identifier to transfer. If omitted interactively, the CLI prompts.
    #[arg(value_name = "TOKEN_ID")]
    pub token_id: Option<concordium_rust_sdk::protocol_level_tokens::TokenId>,

    /// Recipient account address or finalized local account label. If omitted interactively, the CLI prompts.
    #[arg(long = "recipient", value_name = "ADDRESS_OR_LABEL")]
    pub recipient: Option<String>,

    /// Token amount as a decimal value using the token's configured decimals. If omitted interactively, the CLI prompts with the available balance.
    #[arg(long = "amount", value_name = "AMOUNT")]
    pub amount: Option<String>,

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
pub struct TokenAmountArgs {
    /// Token identifier to operate on. If omitted interactively, the CLI prompts.
    #[arg(value_name = "TOKEN_ID")]
    pub token_id: Option<concordium_rust_sdk::protocol_level_tokens::TokenId>,

    /// Token amount as a decimal value using the token's configured decimals. If omitted interactively, the CLI prompts.
    #[arg(long = "amount", value_name = "AMOUNT")]
    pub amount: Option<String>,

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
pub struct TokenListCommand {
    #[command(subcommand)]
    pub command: TokenListSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum TokenListSubcommand {
    /// Add accounts to the token list.
    Add(Box<TokenListMutationArgs>),
    /// Remove accounts from the token list.
    Remove(Box<TokenListMutationArgs>),
}

#[derive(Debug, Args)]
pub struct TokenListMutationArgs {
    /// Token identifier to update. If omitted interactively, the CLI prompts.
    #[arg(value_name = "TOKEN_ID")]
    pub token_id: Option<concordium_rust_sdk::protocol_level_tokens::TokenId>,

    /// Target account addresses or finalized local account labels to add or remove. If omitted interactively, the CLI prompts.
    #[arg(long = "target", value_name = "ADDRESS_OR_LABEL")]
    pub targets: Vec<String>,

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
pub struct TokenPauseArgs {
    /// Token identifier to operate on. If omitted interactively, the CLI prompts.
    #[arg(value_name = "TOKEN_ID")]
    pub token_id: Option<concordium_rust_sdk::protocol_level_tokens::TokenId>,

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
pub struct TokenAdminRolesCommand {
    #[command(subcommand)]
    pub command: TokenAdminRolesSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum TokenAdminRolesSubcommand {
    /// Assign token admin roles to an account.
    Assign(Box<TokenAdminRolesArgs>),
    /// Revoke token admin roles from an account.
    Revoke(Box<TokenAdminRolesArgs>),
}

#[derive(Debug, Args)]
pub struct TokenAdminRolesArgs {
    /// Token identifier to update. If omitted interactively, the CLI prompts.
    #[arg(value_name = "TOKEN_ID")]
    pub token_id: Option<concordium_rust_sdk::protocol_level_tokens::TokenId>,

    /// Target account address or finalized local account label. If omitted interactively, the CLI prompts.
    #[arg(long = "target", value_name = "ADDRESS_OR_LABEL")]
    pub target: Option<String>,

    /// Token admin roles to assign or revoke. If omitted interactively, the CLI presents a multi-select.
    #[arg(long = "role", value_name = "ROLE")]
    pub roles: Vec<String>,

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
pub struct TokenMetadataCommand {
    #[command(subcommand)]
    pub command: TokenMetadataSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum TokenMetadataSubcommand {
    /// Update token metadata.
    Update(Box<TokenMetadataUpdateArgs>),
}

#[derive(Debug, Args)]
pub struct TokenMetadataUpdateArgs {
    /// Token identifier to update. If omitted interactively, the CLI prompts.
    #[arg(value_name = "TOKEN_ID")]
    pub token_id: Option<concordium_rust_sdk::protocol_level_tokens::TokenId>,

    /// Metadata URL. If omitted interactively, the CLI prompts.
    #[arg(long = "url", value_name = "URL")]
    pub url: Option<String>,

    /// Optional SHA-256 checksum for the metadata payload as hex.
    #[arg(long = "checksum-sha256", value_name = "HEX")]
    pub checksum_sha_256: Option<String>,

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
pub struct TokenLockCommand {
    #[command(subcommand)]
    pub command: TokenLockSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum TokenLockSubcommand {
    /// Create a new protocol-level token lock.
    Create(Box<TokenLockCreateArgs>),
    /// Fund an existing lock.
    Fund(Box<TokenLockFundArgs>),
    /// Send locked funds to a configured recipient.
    Send(Box<TokenLockSendArgs>),
    /// Return locked funds to the source account.
    Return(Box<TokenLockReturnArgs>),
    /// Cancel an existing lock.
    Cancel(Box<TokenLockCancelArgs>),
    /// Show protocol-level lock information.
    Show(Box<TokenLockShowArgs>),
}

#[derive(Debug, Args)]
pub struct TokenLockCreateArgs {
    /// Accounts that can receive funds from the lock. Accepts raw addresses or finalized local account labels. Repeat to add multiple recipients.
    #[arg(long = "recipient", value_name = "ADDRESS_OR_LABEL", required = true)]
    pub recipients: Vec<String>,

    /// Lock expiry time as relative duration, RFC3339 timestamp, or unix seconds.
    #[arg(long = "expiry", value_name = "TIME")]
    pub expiry: String,

    /// Controller grant in the form `<ACCOUNT_OR_LABEL:ROLE[,ROLE...]>`. Repeat for multiple grants.
    #[arg(long = "grant", value_name = "GRANT", required = true)]
    pub grants: Vec<String>,

    /// Token identifiers governed by the lock controller. Repeat to add multiple tokens.
    #[arg(long = "token", value_name = "TOKEN_ID", required = true)]
    pub tokens: Vec<concordium_rust_sdk::protocol_level_tokens::TokenId>,

    /// Keep the lock alive after all funds have been returned.
    #[arg(long = "keep-alive")]
    pub keep_alive: bool,

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
pub struct TokenLockFundArgs {
    /// Lock identifier to fund. If omitted interactively, the CLI prompts for it.
    #[arg(value_name = "LOCK_ID")]
    pub lock_id: Option<concordium_rust_sdk::protocol_level_tokens::LockId>,

    /// Token identifier to fund the lock with. If omitted interactively, the CLI prompts from the lock's configured tokens.
    #[arg(long = "token", value_name = "TOKEN_ID")]
    pub token_id: Option<concordium_rust_sdk::protocol_level_tokens::TokenId>,

    /// Token amount as a decimal value using the token's configured decimals. If omitted interactively, the CLI prompts with the available balance.
    #[arg(long = "amount", value_name = "AMOUNT")]
    pub amount: Option<String>,

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
pub struct TokenLockSendArgs {
    /// Lock identifier to use. If omitted interactively, the CLI prompts for it.
    #[arg(value_name = "LOCK_ID")]
    pub lock_id: Option<concordium_rust_sdk::protocol_level_tokens::LockId>,

    /// Token identifier whose locked funds are being sent. If omitted interactively, the CLI prompts from the lock's configured tokens.
    #[arg(long = "token", value_name = "TOKEN_ID")]
    pub token_id: Option<concordium_rust_sdk::protocol_level_tokens::TokenId>,

    /// Source account address or finalized local account label whose funds are currently locked.
    #[arg(long = "source", value_name = "ADDRESS_OR_LABEL")]
    pub source: Option<String>,

    /// Recipient account address or finalized local account label that must be configured on the lock.
    #[arg(long = "recipient", value_name = "ADDRESS_OR_LABEL")]
    pub recipient: Option<String>,

    /// Token amount as a decimal value using the token's configured decimals. If omitted interactively, the CLI prompts with the locked balance.
    #[arg(long = "amount", value_name = "AMOUNT")]
    pub amount: Option<String>,

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
pub struct TokenLockReturnArgs {
    /// Lock identifier to use. If omitted interactively, the CLI prompts for it.
    #[arg(value_name = "LOCK_ID")]
    pub lock_id: Option<concordium_rust_sdk::protocol_level_tokens::LockId>,

    /// Token identifier whose locked funds are being returned. If omitted interactively, the CLI prompts from the lock's configured tokens.
    #[arg(long = "token", value_name = "TOKEN_ID")]
    pub token_id: Option<concordium_rust_sdk::protocol_level_tokens::TokenId>,

    /// Source account address or finalized local account label whose funds are currently locked.
    #[arg(long = "source", value_name = "ADDRESS_OR_LABEL")]
    pub source: Option<String>,

    /// Token amount as a decimal value using the token's configured decimals. If omitted interactively, the CLI prompts with the locked balance.
    #[arg(long = "amount", value_name = "AMOUNT")]
    pub amount: Option<String>,

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
pub struct TokenLockCancelArgs {
    /// Lock identifier to cancel. If omitted interactively, the CLI prompts for it.
    #[arg(value_name = "LOCK_ID")]
    pub lock_id: Option<concordium_rust_sdk::protocol_level_tokens::LockId>,

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
pub struct TokenLockShowArgs {
    /// Lock identifier to inspect.
    #[arg(value_name = "LOCK_ID")]
    pub lock_id: concordium_rust_sdk::protocol_level_tokens::LockId,

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

    /// Disable silent use of active network defaults and force explicit selection.
    #[arg(long = "no-defaults")]
    pub no_defaults: bool,
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
            "--show-payload",
        ]);

        match cli.command {
            Command::Transaction(command) => match command.command {
                TransactionSubcommand::Show(args) => {
                    assert_eq!(args.network.as_deref(), Some("testnet"));
                    assert!(args.node.is_none());
                    assert!(args.show_payload);
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

    #[test]
    fn parses_account_show_command() {
        let cli = Cli::parse_from([
            "ccd-wallet",
            "account",
            "show",
            "alice",
            "--network",
            "testnet",
            "--block",
            "last-final",
            "--json",
            "--verbose",
            "--non-interactive",
        ]);

        match cli.command {
            Command::Account(command) => match command.command {
                AccountSubcommand::Show(args) => {
                    assert_eq!(args.account, "alice");
                    assert_eq!(args.network.as_deref(), Some("testnet"));
                    assert_eq!(args.block.as_deref(), Some("last-final"));
                    assert!(args.json);
                    assert!(args.verbose);
                    assert!(args.non_interactive);
                }
                other => panic!("expected account show command, got {other:?}"),
            },
            other => panic!("expected account command, got {other:?}"),
        }
    }

    #[test]
    fn parses_identity_new_with_key_source_alias() {
        let cli = Cli::parse_from([
            "ccd-wallet",
            "identity",
            "new",
            "alice-id",
            "--provider",
            "7",
            "--key-source",
            "ledger-main",
            "--network",
            "testnet",
            "--allow-ledger-secret-export",
        ]);

        match cli.command {
            Command::Identity(command) => match command.command {
                IdentitySubcommand::New(args) => {
                    assert_eq!(args.label.as_deref(), Some("alice-id"));
                    assert_eq!(args.provider, Some(7));
                    assert_eq!(args.seed.as_deref(), Some("ledger-main"));
                    assert_eq!(args.network.as_deref(), Some("testnet"));
                    assert!(args.allow_ledger_secret_export);
                }
                other => panic!("expected identity new command, got {other:?}"),
            },
            other => panic!("expected identity command, got {other:?}"),
        }
    }

    #[test]
    fn parses_account_new_with_key_source_alias() {
        let cli = Cli::parse_from([
            "ccd-wallet",
            "account",
            "new",
            "alice",
            "--identity",
            "alice-id",
            "--key-source",
            "ledger-main",
            "--network",
            "testnet",
        ]);

        match cli.command {
            Command::Account(command) => match command.command {
                AccountSubcommand::New(args) => {
                    assert_eq!(args.label.as_deref(), Some("alice"));
                    assert_eq!(args.identity.as_deref(), Some("alice-id"));
                    assert_eq!(args.seed.as_deref(), Some("ledger-main"));
                    assert_eq!(args.network.as_deref(), Some("testnet"));
                }
                other => panic!("expected account new command, got {other:?}"),
            },
            other => panic!("expected account command, got {other:?}"),
        }
    }

    #[test]
    fn parses_ledger_setup_command() {
        let cli = Cli::parse_from(["ccd-wallet", "ledger", "setup", "ledger-main"]);

        match cli.command {
            Command::Ledger(command) => match command.command {
                LedgerSubcommand::Setup(args) => {
                    assert_eq!(args.label.as_deref(), Some("ledger-main"));
                    assert_eq!(args.restore.as_deref(), None);
                }
                other => panic!("expected ledger setup command, got {other:?}"),
            },
            other => panic!("expected ledger setup command, got {other:?}"),
        }
    }

    #[test]
    fn parses_ledger_setup_restore_command() {
        let cli = Cli::parse_from([
            "ccd-wallet",
            "ledger",
            "setup",
            "ledger-main",
            "--restore",
            "testnet",
            "--allow-ledger-secret-export",
        ]);

        match cli.command {
            Command::Ledger(command) => match command.command {
                LedgerSubcommand::Setup(args) => {
                    assert_eq!(args.label.as_deref(), Some("ledger-main"));
                    assert_eq!(args.restore.as_deref(), Some("testnet"));
                    assert!(args.allow_ledger_secret_export);
                }
                other => panic!("expected ledger setup command, got {other:?}"),
            },
            other => panic!("expected ledger setup command, got {other:?}"),
        }
    }

    #[test]
    fn parses_ledger_sync_command() {
        let cli = Cli::parse_from([
            "ccd-wallet",
            "ledger",
            "sync",
            "ledger-main",
            "--network",
            "testnet",
            "--provider",
            "0",
            "--provider",
            "all",
            "--allow-ledger-secret-export",
            "--non-interactive",
            "--no-defaults",
        ]);

        match cli.command {
            Command::Ledger(command) => match command.command {
                LedgerSubcommand::Sync(args) => {
                    assert_eq!(args.label.as_deref(), Some("ledger-main"));
                    assert_eq!(args.network.as_deref(), Some("testnet"));
                    assert_eq!(args.providers, vec!["0".to_owned(), "all".to_owned()]);
                    assert!(args.allow_ledger_secret_export);
                    assert!(args.non_interactive);
                    assert!(args.no_defaults);
                }
                other => panic!("expected ledger sync command, got {other:?}"),
            },
            other => panic!("expected ledger sync command, got {other:?}"),
        }
    }

    #[test]
    fn parses_ledger_show_command() {
        let cli = Cli::parse_from(["ccd-wallet", "ledger", "show"]);

        match cli.command {
            Command::Ledger(command) => match command.command {
                LedgerSubcommand::Show => {}
                other => panic!("expected ledger show command, got {other:?}"),
            },
            other => panic!("expected ledger command, got {other:?}"),
        }
    }

    #[test]
    fn parses_token_show_command() {
        let cli = Cli::parse_from(["ccd-wallet", "token", "show", "CCD", "--network", "testnet"]);

        match cli.command {
            Command::Token(command) => match command.command {
                TokenSubcommand::Show(args) => {
                    assert_eq!(args.token_id.to_string(), "CCD");
                    assert_eq!(args.network.as_deref(), Some("testnet"));
                }
                other => panic!("expected token show command, got {other:?}"),
            },
            other => panic!("expected token command, got {other:?}"),
        }
    }

    #[test]
    fn parses_token_admin_roles_assign_command() {
        let cli = Cli::parse_from([
            "ccd-wallet",
            "token",
            "admin-roles",
            "assign",
            "CCD",
            "--target",
            "4UC8o4m8AgTxt5VBFMdLwMCwwJQVJwjesNzW7RPXkACynrULmd",
            "--role",
            "mint",
            "--role",
            "update-metadata",
            "--account",
            "alice",
        ]);

        match cli.command {
            Command::Token(command) => match command.command {
                TokenSubcommand::AdminRoles(command) => match command.command {
                    TokenAdminRolesSubcommand::Assign(args) => {
                        assert_eq!(
                            args.token_id.as_ref().map(|t| t.to_string()).as_deref(),
                            Some("CCD")
                        );
                        assert_eq!(args.roles, vec!["mint", "update-metadata"]);
                        assert_eq!(args.account.as_deref(), Some("alice"));
                    }
                    other => panic!("expected token admin-roles assign command, got {other:?}"),
                },
                other => panic!("expected token admin-roles command, got {other:?}"),
            },
            other => panic!("expected token command, got {other:?}"),
        }
    }

    #[test]
    fn parses_token_lock_create_command() {
        let cli = Cli::parse_from([
            "ccd-wallet",
            "token",
            "lock",
            "create",
            "--recipient",
            "4UC8o4m8AgTxt5VBFMdLwMCwwJQVJwjesNzW7RPXkACynrULmd",
            "--expiry",
            "1h",
            "--grant",
            "4UC8o4m8AgTxt5VBFMdLwMCwwJQVJwjesNzW7RPXkACynrULmd:fund,send,cancel",
            "--token",
            "CCD",
            "--account",
            "alice",
        ]);

        match cli.command {
            Command::Token(command) => match command.command {
                TokenSubcommand::Lock(command) => match command.command {
                    TokenLockSubcommand::Create(args) => {
                        assert_eq!(args.expiry, "1h");
                        assert_eq!(args.tokens.len(), 1);
                        assert_eq!(args.tokens[0].to_string(), "CCD");
                        assert_eq!(args.account.as_deref(), Some("alice"));
                    }
                    other => panic!("expected token lock create command, got {other:?}"),
                },
                other => panic!("expected token lock command, got {other:?}"),
            },
            other => panic!("expected token command, got {other:?}"),
        }
    }

    #[test]
    fn parses_token_lock_show_command() {
        let cli = Cli::parse_from([
            "ccd-wallet",
            "token",
            "lock",
            "show",
            "W9EXVYXZJq",
            "--network",
            "testnet",
        ]);

        match cli.command {
            Command::Token(command) => match command.command {
                TokenSubcommand::Lock(command) => match command.command {
                    TokenLockSubcommand::Show(args) => {
                        assert_eq!(args.lock_id.to_string(), "W9EXVYXZJq");
                        assert_eq!(args.network.as_deref(), Some("testnet"));
                    }
                    other => panic!("expected token lock show command, got {other:?}"),
                },
                other => panic!("expected token lock command, got {other:?}"),
            },
            other => panic!("expected token command, got {other:?}"),
        }
    }

    #[test]
    fn parses_token_lock_send_without_optional_promptable_details() {
        let cli = Cli::parse_from(["ccd-wallet", "token", "lock", "send", "--account", "alice"]);

        match cli.command {
            Command::Token(command) => match command.command {
                TokenSubcommand::Lock(command) => match command.command {
                    TokenLockSubcommand::Send(args) => {
                        assert!(args.lock_id.is_none());
                        assert!(args.token_id.is_none());
                        assert!(args.amount.is_none());
                        assert!(args.source.is_none());
                        assert!(args.recipient.is_none());
                        assert_eq!(args.account.as_deref(), Some("alice"));
                    }
                    other => panic!("expected token lock send command, got {other:?}"),
                },
                other => panic!("expected token lock command, got {other:?}"),
            },
            other => panic!("expected token command, got {other:?}"),
        }
    }

    #[test]
    fn parses_token_lock_fund_without_optional_promptable_details() {
        let cli = Cli::parse_from(["ccd-wallet", "token", "lock", "fund"]);

        match cli.command {
            Command::Token(command) => match command.command {
                TokenSubcommand::Lock(command) => match command.command {
                    TokenLockSubcommand::Fund(args) => {
                        assert!(args.lock_id.is_none());
                        assert!(args.token_id.is_none());
                        assert!(args.amount.is_none());
                    }
                    other => panic!("expected token lock fund command, got {other:?}"),
                },
                other => panic!("expected token lock command, got {other:?}"),
            },
            other => panic!("expected token command, got {other:?}"),
        }
    }

    #[test]
    fn parses_token_transfer_with_recipient_label() {
        let cli = Cli::parse_from([
            "ccd-wallet",
            "token",
            "transfer",
            "CCD",
            "--recipient",
            "treasury",
            "--amount",
            "1",
            "--account",
            "alice",
        ]);

        match cli.command {
            Command::Token(command) => match command.command {
                TokenSubcommand::Transfer(args) => {
                    assert_eq!(args.recipient.as_deref(), Some("treasury"));
                    assert_eq!(args.account.as_deref(), Some("alice"));
                }
                other => panic!("expected token transfer command, got {other:?}"),
            },
            other => panic!("expected token command, got {other:?}"),
        }
    }

    #[test]
    fn parses_token_list_update_with_mixed_targets() {
        let cli = Cli::parse_from([
            "ccd-wallet",
            "token",
            "allow-list",
            "add",
            "CCD",
            "--target",
            "treasury",
            "--target",
            "4UC8o4m8AgTxt5VBFMdLwMCwwJQVJwjesNzW7RPXkACynrULmd",
            "--account",
            "alice",
        ]);

        match cli.command {
            Command::Token(command) => match command.command {
                TokenSubcommand::AllowList(command) => match command.command {
                    TokenListSubcommand::Add(args) => {
                        assert_eq!(
                            args.targets,
                            vec![
                                "treasury".to_owned(),
                                "4UC8o4m8AgTxt5VBFMdLwMCwwJQVJwjesNzW7RPXkACynrULmd".to_owned(),
                            ]
                        );
                    }
                    other => panic!("expected allow-list add command, got {other:?}"),
                },
                other => panic!("expected token allow-list command, got {other:?}"),
            },
            other => panic!("expected token command, got {other:?}"),
        }
    }

    #[test]
    fn parses_token_lock_create_with_label_references() {
        let cli = Cli::parse_from([
            "ccd-wallet",
            "token",
            "lock",
            "create",
            "--recipient",
            "treasury",
            "--expiry",
            "1h",
            "--grant",
            "operator:fund,send,cancel",
            "--token",
            "CCD",
            "--account",
            "alice",
        ]);

        match cli.command {
            Command::Token(command) => match command.command {
                TokenSubcommand::Lock(command) => match command.command {
                    TokenLockSubcommand::Create(args) => {
                        assert_eq!(args.recipients, vec!["treasury"]);
                        assert_eq!(args.grants, vec!["operator:fund,send,cancel"]);
                    }
                    other => panic!("expected token lock create command, got {other:?}"),
                },
                other => panic!("expected token lock command, got {other:?}"),
            },
            other => panic!("expected token command, got {other:?}"),
        }
    }

    #[test]
    fn parses_token_lock_send_and_return_with_label_references() {
        let send = Cli::parse_from([
            "ccd-wallet",
            "token",
            "lock",
            "send",
            "--source",
            "alice",
            "--recipient",
            "treasury",
        ]);
        match send.command {
            Command::Token(command) => match command.command {
                TokenSubcommand::Lock(command) => match command.command {
                    TokenLockSubcommand::Send(args) => {
                        assert_eq!(args.source.as_deref(), Some("alice"));
                        assert_eq!(args.recipient.as_deref(), Some("treasury"));
                    }
                    other => panic!("expected token lock send command, got {other:?}"),
                },
                other => panic!("expected token lock command, got {other:?}"),
            },
            other => panic!("expected token command, got {other:?}"),
        }

        let return_funds =
            Cli::parse_from(["ccd-wallet", "token", "lock", "return", "--source", "alice"]);
        match return_funds.command {
            Command::Token(command) => match command.command {
                TokenSubcommand::Lock(command) => match command.command {
                    TokenLockSubcommand::Return(args) => {
                        assert_eq!(args.source.as_deref(), Some("alice"));
                    }
                    other => panic!("expected token lock return command, got {other:?}"),
                },
                other => panic!("expected token lock command, got {other:?}"),
            },
            other => panic!("expected token command, got {other:?}"),
        }
    }

    #[test]
    fn parses_contract_invoke_with_invoker_label() {
        let cli = Cli::parse_from([
            "ccd-wallet",
            "contract",
            "invoke",
            "--contract",
            "42,0",
            "--receive",
            "counter.view",
            "--invoker",
            "alice",
        ]);

        match cli.command {
            Command::Contract(command) => match command.command {
                ContractSubcommand::Invoke(args) => {
                    assert_eq!(args.invoker.as_deref(), Some("alice"));
                }
                other => panic!("expected contract invoke command, got {other:?}"),
            },
            other => panic!("expected contract command, got {other:?}"),
        }
    }
}
