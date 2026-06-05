use crate::commands::ui::{SelectItem, select_or_single};
use anyhow::{Context, Result, bail};
use ccd_wallet_core::{
    config,
    store::{
        accounts,
        config::{NetworkEntry, list_networks, load, rename_network, save},
        governance, identities, wallet_state,
    },
};
use clap::{Args, Subcommand};
use cliclack::{input, multiselect};
use concordium_rust_sdk::v2;
use rusqlite::Connection;
use std::{collections::BTreeSet, str::FromStr};

// ---------------------------------------------------------------------------
// CLI types
// ---------------------------------------------------------------------------

#[derive(Debug, Args)]
pub struct NetworkCommand {
    #[command(subcommand)]
    pub command: NetworkSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum NetworkSubcommand {
    /// Register a named Concordium network by connecting to a node and
    /// deriving its genesis hash.
    Add(Box<NetworkAddArgs>),
    /// Delete one or more configured networks.
    Delete(NetworkDeleteArgs),
    /// List configured networks.
    List,
    /// Rename a configured network.
    Rename(NetworkRenameArgs),
    /// Reset wallet-local data for a network.
    Reset(NetworkResetArgs),
    /// Show configured network details or matches for a node endpoint.
    Show(Box<NetworkShowArgs>),
    /// Set the active network by name.
    Use(NetworkUseArgs),
}

#[derive(Debug, Args)]
pub struct NetworkAddArgs {
    /// Local name to identify this network.
    #[arg(long, value_name = "NAME")]
    pub name: Option<String>,

    /// Concordium node gRPC endpoint to connect to.
    #[arg(long = "node", value_name = "ENDPOINT")]
    pub node: Option<v2::Endpoint>,

    /// Optional wallet proxy base URL used to resolve wallet-facing identity provider metadata.
    #[arg(long = "wallet-proxy", value_name = "URL")]
    pub wallet_proxy: Option<String>,

    /// Disable prompt fallback and require all values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

#[derive(Debug, Args)]
pub struct NetworkUseArgs {
    /// Name of a registered network to set as active.
    #[arg(value_name = "NAME")]
    pub name: Option<String>,

    /// Disable prompt fallback and require values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

#[derive(Debug, Args)]
pub struct NetworkDeleteArgs {
    /// One or more configured network names to delete.
    #[arg(value_name = "NAME")]
    pub names: Vec<String>,

    /// Disable prompt fallback and require values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

#[derive(Debug, Args)]
pub struct NetworkResetArgs {
    /// Configured network name to reset.
    #[arg(value_name = "NAME")]
    pub name: Option<String>,

    /// Explicit network genesis hash to reset.
    #[arg(long = "genesis-hash", value_name = "HASH")]
    pub genesis_hash: Option<String>,

    /// Disable prompt fallback and require values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

#[derive(Debug, Args)]
pub struct NetworkShowArgs {
    /// Configured network name to show.
    #[arg(value_name = "NAME")]
    pub name: Option<String>,

    /// Explicit Concordium node gRPC endpoint to query.
    #[arg(long = "node", value_name = "ENDPOINT")]
    pub node: Option<v2::Endpoint>,

    /// Disable silent use of active defaults and force explicit selection.
    #[arg(long = "no-defaults")]
    pub no_defaults: bool,

    /// Disable prompt fallback and require values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

#[derive(Debug, Args)]
pub struct NetworkRenameArgs {
    /// Existing network name.
    #[arg(value_name = "OLD_NAME")]
    pub old_name: Option<String>,

    /// New network name.
    #[arg(value_name = "NEW_NAME")]
    pub new_name: Option<String>,

    /// Disable prompt fallback and require values on the command line.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn add(args: NetworkAddArgs) -> Result<()> {
    let name = resolve_required_input(
        args.name,
        args.non_interactive,
        "Network name:",
        "network name must be provided in --non-interactive mode",
    )?;
    let node = match args.node {
        Some(node) => node,
        None if args.non_interactive => {
            bail!("network node endpoint must be provided in --non-interactive mode")
        }
        None => {
            let node_input: String = input("Node endpoint:")
                .validate(|value: &String| {
                    if value.is_empty() {
                        Err("Node endpoint is required.")
                    } else {
                        value
                            .parse::<v2::Endpoint>()
                            .map(|_| ())
                            .map_err(|_| "Enter a valid node endpoint.")
                    }
                })
                .interact()?;
            node_input.parse().context("invalid node endpoint")?
        }
    };
    let wallet_proxy_input = if args.non_interactive {
        args.wallet_proxy
    } else {
        match args.wallet_proxy {
            Some(value) => Some(value),
            None => {
                let value: String = input("Wallet proxy URL (optional):")
                    .required(false)
                    .validate(|value: &String| {
                        if value.is_empty() {
                            Ok(())
                        } else {
                            reqwest::Url::parse(value)
                                .map(|_| ())
                                .map_err(|_| "Enter a valid wallet proxy URL.")
                        }
                    })
                    .interact()?;
                if value.is_empty() { None } else { Some(value) }
            }
        }
    };
    let endpoint_label = config::endpoint_label(&node);

    let mut app_config = load()?;

    if app_config.networks.contains_key(&name) {
        bail!(
            "network '{}' is already registered in the config; \
             use a different name or remove the existing entry first",
            name
        );
    }

    let mut client = config::connect_v2_client(node)
        .await
        .with_context(|| format!("failed to connect to Concordium node at {endpoint_label}"))?;

    let consensus_info = client
        .get_consensus_info()
        .await
        .with_context(|| format!("failed to query consensus info from node at {endpoint_label}"))?;

    let genesis_hash = format!("{}", consensus_info.genesis_block);

    let wallet_proxy = wallet_proxy_input
        .map(|wallet_proxy_input| {
            reqwest::Url::parse(&wallet_proxy_input)
                .with_context(|| format!("invalid wallet proxy URL: {wallet_proxy_input}"))
                .map(|url| config::normalize_url_string(url.as_ref()))
        })
        .transpose()?;

    app_config.networks.insert(
        name.clone(),
        NetworkEntry {
            node_endpoint: endpoint_label.clone(),
            genesis_hash: genesis_hash.clone(),
            wallet_proxy: wallet_proxy.clone(),
        },
    );

    save(&app_config)?;

    println!("Network '{}' registered successfully.", name);
    println!("  endpoint:      {endpoint_label}");
    println!("  genesis hash:  {genesis_hash}");
    if let Some(wallet_proxy) = wallet_proxy {
        println!("  wallet proxy:  {wallet_proxy}");
    }

    Ok(())
}

fn resolve_required_input(
    value: Option<String>,
    non_interactive: bool,
    prompt: &str,
    error: &str,
) -> Result<String> {
    match value {
        Some(value) => Ok(value),
        None if non_interactive => bail!("{error}"),
        None => Ok(input(prompt)
            .validate(|value: &String| {
                if value.is_empty() {
                    Err("Value is required.")
                } else {
                    Ok(())
                }
            })
            .interact()?),
    }
}

pub async fn show(conn: &Connection, args: NetworkShowArgs) -> Result<()> {
    let target = resolve_show_target(conn, &args)?;
    let summary = query_consensus_summary(target.endpoint.clone(), &target.endpoint_label).await?;

    match &target.mode {
        NetworkShowMode::Config(config_view) => {
            render_network_configuration(config_view);
            maybe_warn_genesis_mismatch(config_view, &summary)?;
        }
        NetworkShowMode::NodeOnly => {
            let app_config = load()?;
            render_network_matches(&app_config, &summary.genesis_hash);
        }
    }
    println!();
    render_consensus(&summary, &target.endpoint_label);
    Ok(())
}

pub async fn list(conn: &Connection) -> Result<()> {
    let app_config = load()?;
    let active = wallet_state::get(conn, wallet_state::ACTIVE_NETWORK_KEY)?;
    let identities = identities::list(conn)?;
    let accounts = accounts::list(conn)?;

    for (name, entry) in list_networks(&app_config) {
        let identity_count = identities
            .iter()
            .filter(|record| record.network_genesis_hash == entry.genesis_hash)
            .count();
        let account_count = accounts
            .iter()
            .filter(|record| record.network_genesis_hash == entry.genesis_hash)
            .count();
        println!(
            "{}",
            render_network_list_text(
                &name,
                &entry,
                active.as_deref() == Some(name.as_str()),
                identity_count,
                account_count,
            )
        );
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct NetworkConfigView {
    name: String,
    entry: NetworkEntry,
}

#[derive(Clone, Debug)]
enum NetworkShowMode {
    Config(NetworkConfigView),
    NodeOnly,
}

#[derive(Clone, Debug)]
struct NetworkShowTarget {
    endpoint: v2::Endpoint,
    endpoint_label: String,
    mode: NetworkShowMode,
}

#[derive(Clone, Debug)]
struct ConsensusSummary {
    genesis_hash: String,
    protocol_version: String,
    best_block: String,
    best_block_height: String,
    last_finalized_block: String,
    last_finalized_block_height: String,
}

fn render_network_list_text(
    name: &str,
    entry: &NetworkEntry,
    active: bool,
    identity_count: usize,
    account_count: usize,
) -> String {
    render_network_text(name, entry, active, identity_count, account_count, true)
}

fn render_network_selector_text(
    name: &str,
    entry: &NetworkEntry,
    identity_count: usize,
    account_count: usize,
) -> String {
    render_network_text(name, entry, false, identity_count, account_count, false)
}

fn render_network_text(
    name: &str,
    entry: &NetworkEntry,
    active: bool,
    identity_count: usize,
    account_count: usize,
    show_active: bool,
) -> String {
    let mut text = format!(
        "{name} — {} • {} • {}",
        entry.node_endpoint,
        format_count(identity_count, "identity", "identities"),
        format_count(account_count, "account", "accounts"),
    );
    if show_active && active {
        text.push_str(" • active");
    }
    text
}

fn format_count(count: usize, singular: &str, plural: &str) -> String {
    let noun = if count == 1 { singular } else { plural };
    format!("{count} {noun}")
}

fn network_config_view(
    app_config: &ccd_wallet_core::store::config::AppConfig,
    name: &str,
) -> Result<NetworkConfigView> {
    let entry = app_config
        .networks
        .get(name)
        .cloned()
        .with_context(|| format!("network '{name}' is not registered"))?;
    Ok(NetworkConfigView {
        name: name.to_owned(),
        entry,
    })
}

fn endpoint_from_string(endpoint: &str) -> Result<v2::Endpoint> {
    v2::Endpoint::from_str(&config::normalize_url_string(endpoint))
        .with_context(|| format!("invalid stored node endpoint: {endpoint}"))
}

fn resolve_show_target_with_config(
    conn: &Connection,
    app_config: &ccd_wallet_core::store::config::AppConfig,
    args: &NetworkShowArgs,
) -> Result<NetworkShowTarget> {
    match (args.name.as_deref(), args.node.clone()) {
        (Some(name), Some(node)) => Ok(NetworkShowTarget {
            endpoint_label: config::endpoint_label(&node),
            endpoint: node,
            mode: NetworkShowMode::Config(network_config_view(app_config, name)?),
        }),
        (Some(name), None) => {
            let config_view = network_config_view(app_config, name)?;
            let endpoint = endpoint_from_string(&config_view.entry.node_endpoint)?;
            Ok(NetworkShowTarget {
                endpoint,
                endpoint_label: config_view.entry.node_endpoint.clone(),
                mode: NetworkShowMode::Config(config_view),
            })
        }
        (None, Some(node)) => Ok(NetworkShowTarget {
            endpoint_label: config::endpoint_label(&node),
            endpoint: node,
            mode: NetworkShowMode::NodeOnly,
        }),
        (None, None) if args.no_defaults && args.non_interactive => {
            bail!("network name or --node must be provided in --non-interactive mode")
        }
        (None, None) if args.no_defaults => {
            let name = select_network_name(conn, app_config)?;
            let config_view = network_config_view(app_config, &name)?;
            let endpoint = endpoint_from_string(&config_view.entry.node_endpoint)?;
            Ok(NetworkShowTarget {
                endpoint,
                endpoint_label: config_view.entry.node_endpoint.clone(),
                mode: NetworkShowMode::Config(config_view),
            })
        }
        (None, None) => {
            let active = wallet_state::get(conn, wallet_state::ACTIVE_NETWORK_KEY)?.with_context(
                || {
                    "no active network is set; provide a network label, use `--node`, or run `ccd-wallet network use <NAME>`"
                },
            )?;
            let config_view = network_config_view(app_config, &active).with_context(|| {
                format!(
                    "active network '{}' is no longer registered; update it with `ccd-wallet network use <NAME>` or provide a network label / `--node` explicitly",
                    active
                )
            })?;
            let endpoint = endpoint_from_string(&config_view.entry.node_endpoint)?;
            Ok(NetworkShowTarget {
                endpoint,
                endpoint_label: config_view.entry.node_endpoint.clone(),
                mode: NetworkShowMode::Config(config_view),
            })
        }
    }
}

fn resolve_show_target(conn: &Connection, args: &NetworkShowArgs) -> Result<NetworkShowTarget> {
    let app_config = load()?;
    resolve_show_target_with_config(conn, &app_config, args)
}

async fn query_consensus_summary(
    endpoint: v2::Endpoint,
    endpoint_label: &str,
) -> Result<ConsensusSummary> {
    let mut client = config::connect_v2_client(endpoint)
        .await
        .with_context(|| format!("failed to connect to Concordium node at {endpoint_label}"))?;
    let consensus_info = client
        .get_consensus_info()
        .await
        .with_context(|| format!("failed to query consensus info from node at {endpoint_label}"))?;
    Ok(ConsensusSummary {
        genesis_hash: format!("{}", consensus_info.genesis_block),
        protocol_version: format!("{}", consensus_info.protocol_version),
        best_block: format!("{}", consensus_info.best_block),
        best_block_height: consensus_info.best_block_height.height.to_string(),
        last_finalized_block: format!("{}", consensus_info.last_finalized_block),
        last_finalized_block_height: consensus_info
            .last_finalized_block_height
            .height
            .to_string(),
    })
}

fn render_consensus(summary: &ConsensusSummary, endpoint_label: &str) {
    println!("Consensus ({endpoint_label})");
    println!("- observed genesis hash: {}", summary.genesis_hash);
    println!("- protocol version: {}", summary.protocol_version);
    println!("- best block: {}", summary.best_block);
    println!("- best block height: {}", summary.best_block_height);
    println!("- last finalized block: {}", summary.last_finalized_block);
    println!(
        "- last finalized block height: {}",
        summary.last_finalized_block_height
    );
}

fn render_network_configuration(config_view: &NetworkConfigView) {
    println!("Network configuration");
    println!("- name: {}", config_view.name);
    println!("- node: {}", config_view.entry.node_endpoint);
    if let Some(wallet_proxy) = &config_view.entry.wallet_proxy {
        println!("- wallet proxy: {}", wallet_proxy);
    }
    println!("- genesis hash: {}", config_view.entry.genesis_hash);
}

fn network_match_lines(
    app_config: &ccd_wallet_core::store::config::AppConfig,
    genesis_hash: &str,
) -> (String, Vec<String>) {
    let matches = app_config
        .networks
        .iter()
        .filter(|(_, entry)| entry.genesis_hash == genesis_hash)
        .collect::<Vec<_>>();
    let heading = if matches.len() == 1 {
        format!("Network match ({genesis_hash})")
    } else {
        format!("Network matches ({genesis_hash})")
    };
    let lines = if matches.is_empty() {
        vec!["- none".to_owned()]
    } else {
        matches
            .into_iter()
            .map(|(name, entry)| format!("- {} ({})", name, entry.node_endpoint))
            .collect()
    };
    (heading, lines)
}

fn render_network_matches(
    app_config: &ccd_wallet_core::store::config::AppConfig,
    genesis_hash: &str,
) {
    let (heading, lines) = network_match_lines(app_config, genesis_hash);
    println!("{heading}");
    for line in lines {
        println!("{line}");
    }
}

fn maybe_warn_genesis_mismatch(
    config_view: &NetworkConfigView,
    summary: &ConsensusSummary,
) -> Result<()> {
    if config_view.entry.genesis_hash != summary.genesis_hash {
        cliclack::log::warning(format!(
            "configured network '{}' expects genesis hash {}, but the queried node reported {}",
            config_view.name, config_view.entry.genesis_hash, summary.genesis_hash
        ))?;
    }
    Ok(())
}

fn abbreviate_hash(hash: &str) -> String {
    if hash.chars().count() <= 12 {
        return hash.to_owned();
    }
    let prefix = hash.chars().take(4).collect::<String>();
    let suffix = hash
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("{prefix}…{suffix}")
}

fn render_reset_partition_label(genesis_hash: &str, aliases: &[String]) -> String {
    let abbreviated = abbreviate_hash(genesis_hash);
    if aliases.is_empty() {
        format!("{abbreviated} (orphan)")
    } else {
        format!("{abbreviated} - {}", aliases.join(", "))
    }
}

fn render_partition_hint(identity_count: usize, account_count: usize) -> String {
    format!(
        "{} • {}",
        format_count(identity_count, "identity", "identities"),
        format_count(account_count, "account", "accounts")
    )
}

fn network_partition_counts(conn: &Connection, genesis_hash: &str) -> Result<(usize, usize)> {
    let identity_count = identities::list(conn)?
        .into_iter()
        .filter(|record| record.network_genesis_hash == genesis_hash)
        .count();
    let account_count = accounts::list(conn)?
        .into_iter()
        .filter(|record| record.network_genesis_hash == genesis_hash)
        .count();
    Ok((identity_count, account_count))
}

fn known_network_hashes(conn: &Connection) -> Result<Vec<String>> {
    let mut hashes = BTreeSet::new();
    hashes.extend(identities::distinct_network_genesis_hashes(conn)?);
    hashes.extend(accounts::distinct_network_genesis_hashes(conn)?);
    hashes.extend(governance::distinct_network_genesis_hashes(conn)?);
    Ok(hashes.into_iter().collect())
}

fn resolve_reset_genesis_hash(
    conn: &Connection,
    app_config: &ccd_wallet_core::store::config::AppConfig,
    name: Option<String>,
    genesis_hash: Option<String>,
    non_interactive: bool,
) -> Result<String> {
    match (name, genesis_hash) {
        (Some(_), Some(_)) => bail!("network name and --genesis-hash cannot be combined"),
        (Some(name), None) => app_config
            .networks
            .get(&name)
            .map(|entry| entry.genesis_hash.clone())
            .with_context(|| format!("network '{name}' is not registered")),
        (None, Some(genesis_hash)) => Ok(genesis_hash),
        (None, None) if non_interactive => {
            bail!("network name or --genesis-hash must be provided in --non-interactive mode")
        }
        (None, None) => select_reset_genesis_hash(conn, app_config),
    }
}

fn select_reset_genesis_hash(
    conn: &Connection,
    app_config: &ccd_wallet_core::store::config::AppConfig,
) -> Result<String> {
    let hashes = known_network_hashes(conn)?;
    if hashes.is_empty() {
        bail!("no network data is stored; nothing to reset")
    }

    let items = hashes
        .iter()
        .map(|hash| {
            let aliases = ccd_wallet_core::store::config::aliases_by_genesis_hash(app_config, hash);
            let (identity_count, account_count) = network_partition_counts(conn, hash)?;
            Ok(SelectItem {
                value: hash.clone(),
                label: render_reset_partition_label(hash, &aliases),
                hint: render_partition_hint(identity_count, account_count),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    select_or_single("Select network to reset", &items, None)
}

fn prune_network_partition(conn: &Connection, genesis_hash: &str) -> Result<(usize, usize)> {
    let identity_count = identities::prune_by_network(conn, genesis_hash)?;
    let account_count = accounts::prune_by_network(conn, genesis_hash)?;
    let _ = governance::prune_by_network(conn, genesis_hash)?;
    Ok((identity_count, account_count))
}

pub async fn reset(conn: &Connection, args: NetworkResetArgs) -> Result<()> {
    let app_config = load()?;
    let genesis_hash = resolve_reset_genesis_hash(
        conn,
        &app_config,
        args.name,
        args.genesis_hash,
        args.non_interactive,
    )?;
    let aliases =
        ccd_wallet_core::store::config::aliases_by_genesis_hash(&app_config, &genesis_hash);
    let (identity_count, account_count) = network_partition_counts(conn, &genesis_hash)?;
    cliclack::log::warning(format!(
        "This will reset network data for '{}' and remove {} and {}.",
        render_reset_partition_label(&genesis_hash, &aliases),
        format_count(identity_count, "identity", "identities"),
        format_count(account_count, "account", "accounts"),
    ))?;
    let confirmation: String = input(format!("Type '{}' to confirm:", genesis_hash))
        .validate(|value: &String| {
            if value.is_empty() {
                Err("Confirmation is required.")
            } else {
                Ok(())
            }
        })
        .interact()?;
    if confirmation != genesis_hash {
        bail!("network reset aborted: confirmation did not match '{genesis_hash}'");
    }

    prune_network_partition(conn, &genesis_hash)?;
    println!("Network data for '{genesis_hash}' reset successfully.");
    Ok(())
}

fn resolve_delete_names(
    app_config: &ccd_wallet_core::store::config::AppConfig,
    names: Vec<String>,
    non_interactive: bool,
) -> Result<Vec<String>> {
    if !names.is_empty() {
        let mut seen = BTreeSet::new();
        for name in &names {
            if !app_config.networks.contains_key(name) {
                bail!("network '{name}' is not registered");
            }
            if !seen.insert(name.clone()) {
                bail!("network '{name}' was provided more than once");
            }
        }
        return Ok(names);
    }
    if non_interactive {
        bail!("at least one network name must be provided in --non-interactive mode");
    }
    select_delete_names(app_config)
}

fn select_delete_names(
    app_config: &ccd_wallet_core::store::config::AppConfig,
) -> Result<Vec<String>> {
    if app_config.networks.is_empty() {
        bail!("no networks are configured; run `ccd-wallet network add` first")
    }
    let mut picker = multiselect("Select networks to delete").filter_mode();
    for (name, entry) in &app_config.networks {
        picker = picker.item(
            name.clone(),
            name.clone(),
            abbreviate_hash(&entry.genesis_hash),
        );
    }
    Ok(picker.interact()?)
}

fn network_delete_orphan_warnings(
    conn: &Connection,
    app_config: &ccd_wallet_core::store::config::AppConfig,
    names: &[String],
) -> Result<Vec<String>> {
    let selected = names.iter().cloned().collect::<BTreeSet<_>>();
    let mut warnings = Vec::new();
    let mut seen_hashes = BTreeSet::new();
    for name in names {
        let entry = app_config
            .networks
            .get(name)
            .with_context(|| format!("network '{name}' is not registered"))?;
        if !seen_hashes.insert(entry.genesis_hash.clone()) {
            continue;
        }
        let remaining_aliases = ccd_wallet_core::store::config::aliases_by_genesis_hash(
            app_config,
            &entry.genesis_hash,
        )
        .into_iter()
        .filter(|alias| !selected.contains(alias))
        .collect::<Vec<_>>();
        if remaining_aliases.is_empty() {
            let (identity_count, account_count) =
                network_partition_counts(conn, &entry.genesis_hash)?;
            if identity_count > 0 || account_count > 0 {
                warnings.push(format!(
                    "{} will become orphaned with {} and {} remaining local data",
                    entry.genesis_hash,
                    format_count(identity_count, "identity", "identities"),
                    format_count(account_count, "account", "accounts")
                ));
            }
        }
    }
    Ok(warnings)
}

fn cleanup_deleted_active_network_alias(conn: &Connection, names: &[String]) -> Result<()> {
    if let Some(active) = wallet_state::get(conn, wallet_state::ACTIVE_NETWORK_KEY)?
        && names.contains(&active)
    {
        wallet_state::remove(conn, wallet_state::ACTIVE_NETWORK_KEY)?;
    }
    Ok(())
}

fn apply_network_delete(
    conn: &Connection,
    app_config: &mut ccd_wallet_core::store::config::AppConfig,
    names: &[String],
) -> Result<()> {
    ccd_wallet_core::store::config::delete_networks(app_config, names)?;
    save(app_config)?;
    cleanup_deleted_active_network_alias(conn, names)?;
    Ok(())
}

pub async fn delete(conn: &Connection, args: NetworkDeleteArgs) -> Result<()> {
    let mut app_config = load()?;
    let names = resolve_delete_names(&app_config, args.names, args.non_interactive)?;
    let joined = names.join(" ");

    let orphan_warnings = network_delete_orphan_warnings(conn, &app_config, &names)?;
    let mut warning = format!(
        "This will delete {} network alias(es): {}.",
        names.len(),
        joined
    );
    if !orphan_warnings.is_empty() {
        warning.push_str(" Local wallet data will not be removed. ");
        warning.push_str(&orphan_warnings.join("; "));
        warning.push_str(". Use `ccd-wallet network reset` to prune orphaned data.");
    }
    cliclack::log::warning(warning)?;
    let confirmation: String = input(format!("Type '{}' to confirm:", joined))
        .validate(|value: &String| {
            if value.is_empty() {
                Err("Confirmation is required.")
            } else {
                Ok(())
            }
        })
        .interact()?;
    if confirmation != joined {
        bail!("network deletion aborted: confirmation did not match '{joined}'");
    }

    apply_network_delete(conn, &mut app_config, &names)?;
    println!("Deleted network aliases: {joined}.");
    Ok(())
}

pub async fn rename(conn: &Connection, args: NetworkRenameArgs) -> Result<()> {
    let mut app_config = load()?;
    let old_name = match args.old_name {
        Some(name) => name,
        None if args.non_interactive => {
            bail!("network name must be provided in --non-interactive mode")
        }
        None => select_network_name(conn, &app_config)?,
    };
    let new_name = match args.new_name {
        Some(name) => name,
        None if args.non_interactive => {
            bail!("new network name must be provided in --non-interactive mode")
        }
        None => input("New network name:")
            .placeholder(&old_name)
            .validate(|value: &String| {
                if value.is_empty() {
                    Err("Network name is required.")
                } else {
                    Ok(())
                }
            })
            .interact()?,
    };

    rename_network(&mut app_config, &old_name, &new_name)?;
    save(&app_config)?;
    if wallet_state::get(conn, wallet_state::ACTIVE_NETWORK_KEY)?.as_deref()
        == Some(old_name.as_str())
    {
        wallet_state::set(conn, wallet_state::ACTIVE_NETWORK_KEY, &new_name)?;
    }
    println!("Network '{old_name}' renamed to '{new_name}'.");
    Ok(())
}

pub async fn use_network(conn: &Connection, args: NetworkUseArgs) -> Result<()> {
    let app_config = load()?;
    let name = resolve_network_use_name(conn, &app_config, args.name, args.non_interactive)?;

    if !app_config.networks.contains_key(&name) {
        bail!(
            "network '{}' is not registered; run `ccd-wallet network add --name {} --node <ENDPOINT>` first",
            name,
            name
        );
    }

    wallet_state::set(conn, wallet_state::ACTIVE_NETWORK_KEY, &name)?;

    println!("Active network set to '{name}'.");

    Ok(())
}

fn resolve_network_use_name(
    conn: &Connection,
    app_config: &ccd_wallet_core::store::config::AppConfig,
    name: Option<String>,
    non_interactive: bool,
) -> Result<String> {
    match name {
        Some(name) => Ok(name),
        None if non_interactive => {
            bail!("network name must be provided in --non-interactive mode")
        }
        None => select_network_name(conn, app_config),
    }
}

fn select_network_name(
    conn: &Connection,
    app_config: &ccd_wallet_core::store::config::AppConfig,
) -> Result<String> {
    if app_config.networks.is_empty() {
        bail!("no networks are configured; run `ccd-wallet network add` first")
    }

    let identities = identities::list(conn)?;
    let accounts = accounts::list(conn)?;
    let active = wallet_state::get(conn, wallet_state::ACTIVE_NETWORK_KEY)?;
    let items = app_config
        .networks
        .iter()
        .map(|(name, entry)| {
            let identity_count = identities
                .iter()
                .filter(|record| record.network_genesis_hash == entry.genesis_hash)
                .count();
            let account_count = accounts
                .iter()
                .filter(|record| record.network_genesis_hash == entry.genesis_hash)
                .count();
            SelectItem {
                value: name.clone(),
                label: render_network_selector_text(name, entry, identity_count, account_count),
                hint: String::new(),
            }
        })
        .collect::<Vec<_>>();
    let initial = active.as_ref();
    select_or_single("Select network", &items, initial)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccd_wallet_core::store::{config::AppConfig, governance, migrations, seeds};
    use std::collections::BTreeMap;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    fn app_config_with_networks(names: &[&str]) -> AppConfig {
        let networks = names
            .iter()
            .map(|name| {
                (
                    (*name).to_owned(),
                    NetworkEntry {
                        node_endpoint: format!("https://{name}.example.com:20000"),
                        genesis_hash: format!("hash-{name}"),
                        wallet_proxy: Some(format!("https://wallet-proxy.{name}.example.com")),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        AppConfig {
            version: 1,
            networks,
        }
    }

    #[test]
    fn network_use_missing_name_errors_in_non_interactive_mode() {
        let conn = conn();
        let app_config = app_config_with_networks(&["testnet"]);

        let err = resolve_network_use_name(&conn, &app_config, None, true).unwrap_err();

        assert!(
            err.to_string()
                .contains("network name must be provided in --non-interactive mode")
        );
    }

    #[test]
    fn network_use_missing_name_skips_selector_when_only_one_network_exists() {
        let conn = conn();
        let app_config = app_config_with_networks(&["testnet"]);

        let selected = resolve_network_use_name(&conn, &app_config, None, false).unwrap();

        assert_eq!(selected, "testnet");
    }

    #[test]
    fn rename_active_network_updates_wallet_state() {
        let conn = conn();
        wallet_state::set(&conn, wallet_state::ACTIVE_NETWORK_KEY, "testnet").unwrap();

        let mut app_config = app_config_with_networks(&["testnet"]);
        rename_network(&mut app_config, "testnet", "staging").unwrap();
        if wallet_state::get(&conn, wallet_state::ACTIVE_NETWORK_KEY)
            .unwrap()
            .as_deref()
            == Some("testnet")
        {
            wallet_state::set(&conn, wallet_state::ACTIVE_NETWORK_KEY, "staging").unwrap();
        }

        assert_eq!(
            wallet_state::get(&conn, wallet_state::ACTIVE_NETWORK_KEY).unwrap(),
            Some("staging".to_owned())
        );
    }

    #[test]
    fn render_network_list_text_uses_single_line_format() {
        let entry = NetworkEntry {
            node_endpoint: "https://grpc.testnet.concordium.com:20000".to_owned(),
            genesis_hash: "hash-testnet".to_owned(),
            wallet_proxy: Some("https://wallet-proxy.testnet.concordium.com".to_owned()),
        };

        assert_eq!(
            render_network_list_text("testnet", &entry, true, 2, 3),
            "testnet — https://grpc.testnet.concordium.com:20000 • 2 identities • 3 accounts • active"
        );
        assert_eq!(
            render_network_selector_text("testnet", &entry, 2, 3),
            "testnet — https://grpc.testnet.concordium.com:20000 • 2 identities • 3 accounts"
        );
    }

    #[test]
    fn resolve_show_target_uses_active_network_in_config_mode() {
        let conn = conn();
        wallet_state::set(&conn, wallet_state::ACTIVE_NETWORK_KEY, "testnet").unwrap();
        let app_config = app_config_with_networks(&["testnet"]);

        let target = resolve_show_target_with_config(
            &conn,
            &app_config,
            &NetworkShowArgs {
                name: None,
                node: None,
                no_defaults: false,
                non_interactive: false,
            },
        )
        .unwrap();

        assert_eq!(target.endpoint_label, "https://testnet.example.com:20000");
        assert!(matches!(
            target.mode,
            NetworkShowMode::Config(NetworkConfigView { name, .. }) if name == "testnet"
        ));
    }

    #[test]
    fn resolve_show_target_node_only_does_not_use_active_network() {
        let conn = conn();
        wallet_state::set(&conn, wallet_state::ACTIVE_NETWORK_KEY, "testnet").unwrap();
        let app_config = app_config_with_networks(&["testnet"]);
        let endpoint = "https://override.example.com:20000".parse().unwrap();

        let target = resolve_show_target_with_config(
            &conn,
            &app_config,
            &NetworkShowArgs {
                name: None,
                node: Some(endpoint),
                no_defaults: false,
                non_interactive: false,
            },
        )
        .unwrap();

        assert_eq!(target.endpoint_label, "https://override.example.com:20000");
        assert!(matches!(target.mode, NetworkShowMode::NodeOnly));
    }

    #[test]
    fn render_reset_partition_label_uses_hash_and_aliases() {
        assert_eq!(
            render_reset_partition_label("1234567890abcdef", &["testnet".to_owned()]),
            "1234…cdef - testnet"
        );
        assert_eq!(
            render_reset_partition_label(
                "1234567890abcdef",
                &["testnet".to_owned(), "staging-testnet".to_owned()]
            ),
            "1234…cdef - testnet, staging-testnet"
        );
        assert_eq!(
            render_reset_partition_label("1234567890abcdef", &[]),
            "1234…cdef (orphan)"
        );
    }

    #[test]
    fn resolve_reset_genesis_hash_rejects_ambiguous_inputs() {
        let conn = conn();
        let app_config = app_config_with_networks(&["testnet"]);
        let err = resolve_reset_genesis_hash(
            &conn,
            &app_config,
            Some("testnet".to_owned()),
            Some("abc".to_owned()),
            false,
        )
        .unwrap_err();
        assert!(err.to_string().contains("cannot be combined"));
    }

    #[test]
    fn prune_network_partition_deletes_only_matching_rows() {
        let mut conn = conn();
        let seed = seeds::add(&conn, "main_seed", b"seed secret", "password").unwrap();
        accounts::insert_pending(
            &conn,
            accounts::PendingAccount {
                network_genesis_hash: "hash-testnet",
                signer_owner_id: &seed.id,
                ip_identity: 1,
                identity_index: 0,
                credential_counter: 0,
                label: "account-a",
            },
        )
        .unwrap();
        let unlocked = seeds::unlock_context(&conn, "main_seed", "password").unwrap();
        identities::insert_pending(
            &mut conn,
            &unlocked.dek,
            identities::PendingIdentity {
                network_genesis_hash: "hash-testnet",
                signer_owner_id: &seed.id,
                ip_identity: 1,
                identity_index: 0,
                label: "identity-a",
                code_uri: "code",
            },
        )
        .unwrap();
        identities::insert_pending(
            &mut conn,
            &unlocked.dek,
            identities::PendingIdentity {
                network_genesis_hash: "hash-mainnet",
                signer_owner_id: &seed.id,
                ip_identity: 1,
                identity_index: 1,
                label: "identity-b",
                code_uri: "code-2",
            },
        )
        .unwrap();

        assert_eq!(
            prune_network_partition(&conn, "hash-testnet").unwrap(),
            (1, 1)
        );
        assert_eq!(
            known_network_hashes(&conn).unwrap(),
            vec!["hash-mainnet".to_owned()]
        );
        assert!(seeds::find_by_label(&conn, "main_seed").unwrap().is_some());
        let vault_count: u32 = conn
            .query_row("SELECT COUNT(*) FROM signer_owner_vaults", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(vault_count, 1);
    }

    #[test]
    fn known_network_hashes_includes_governance_vault_networks() {
        let conn = conn();
        governance::create_or_unlock_vault(&conn, "hash-governance", "").unwrap();
        assert!(
            known_network_hashes(&conn)
                .unwrap()
                .contains(&"hash-governance".to_owned())
        );
    }

    #[test]
    fn network_match_lines_render_multiple_and_no_match_cases() {
        let mut app_config = app_config_with_networks(&["testnet"]);
        app_config.networks.insert(
            "other_testnet".to_owned(),
            NetworkEntry {
                node_endpoint: "https://other.example.com:20000".to_owned(),
                genesis_hash: "hash-testnet".to_owned(),
                wallet_proxy: Some("https://wallet-proxy.other.example.com".to_owned()),
            },
        );

        let (heading, lines) = network_match_lines(&app_config, "hash-testnet");
        assert_eq!(heading, "Network matches (hash-testnet)");
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().any(|line| line.contains("testnet (")));
        assert!(lines.iter().any(|line| line.contains("other_testnet")));

        let (heading, lines) = network_match_lines(&app_config, "missing-hash");
        assert_eq!(heading, "Network matches (missing-hash)");
        assert_eq!(lines, vec!["- none".to_owned()]);
    }

    #[test]
    fn orphan_warning_detects_last_alias_with_data() {
        let conn = conn();
        let seed = seeds::add(&conn, "main_seed", b"seed secret", "password").unwrap();
        accounts::insert_pending(
            &conn,
            accounts::PendingAccount {
                network_genesis_hash: "hash-testnet",
                signer_owner_id: &seed.id,
                ip_identity: 1,
                identity_index: 0,
                credential_counter: 0,
                label: "account-a",
            },
        )
        .unwrap();
        let app_config = app_config_with_networks(&["testnet"]);
        let warnings =
            network_delete_orphan_warnings(&conn, &app_config, &["testnet".to_owned()]).unwrap();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("hash-testnet"));
    }

    #[test]
    fn cleanup_deleted_active_network_alias_clears_only_matching_active() {
        let conn = conn();
        wallet_state::set(&conn, wallet_state::ACTIVE_NETWORK_KEY, "testnet").unwrap();
        cleanup_deleted_active_network_alias(&conn, &["testnet".to_owned()]).unwrap();
        assert_eq!(
            wallet_state::get(&conn, wallet_state::ACTIVE_NETWORK_KEY).unwrap(),
            None
        );

        wallet_state::set(&conn, wallet_state::ACTIVE_NETWORK_KEY, "testnet").unwrap();
        cleanup_deleted_active_network_alias(&conn, &["mainnet".to_owned()]).unwrap();
        assert_eq!(
            wallet_state::get(&conn, wallet_state::ACTIVE_NETWORK_KEY).unwrap(),
            Some("testnet".to_owned())
        );
    }

    #[test]
    fn format_count_handles_singular_and_plural() {
        assert_eq!(format_count(1, "identity", "identities"), "1 identity");
        assert_eq!(format_count(2, "identity", "identities"), "2 identities");
    }
}
