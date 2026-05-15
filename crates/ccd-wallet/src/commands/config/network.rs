use crate::commands::ui::{SelectItem, select_or_single};
use anyhow::{Context, Result, bail};
use ccd_wallet_core::{
    config,
    store::{
        accounts,
        config::{NetworkEntry, list_networks, load, rename_network, save},
        identities, wallet_state,
    },
};
use clap::{Args, Subcommand};
use cliclack::input;
use concordium_rust_sdk::v2;
use rusqlite::Connection;

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
    /// List configured networks.
    List,
    /// Rename a configured network.
    Rename(NetworkRenameArgs),
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

    /// Wallet proxy base URL used to resolve wallet-facing identity provider metadata.
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
    let wallet_proxy_input = resolve_required_input(
        args.wallet_proxy,
        args.non_interactive,
        "Wallet proxy URL:",
        "wallet proxy URL must be provided in --non-interactive mode",
    )?;
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

    let wallet_proxy = config::normalize_url_string(
        reqwest::Url::parse(&wallet_proxy_input)
            .with_context(|| format!("invalid wallet proxy URL: {wallet_proxy_input}"))?
            .as_ref(),
    );

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
    println!("  wallet proxy:  {wallet_proxy}");

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
    use ccd_wallet_core::store::{config::AppConfig, migrations};
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
                        wallet_proxy: format!("https://wallet-proxy.{name}.example.com"),
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
            wallet_proxy: "https://wallet-proxy.testnet.concordium.com".to_owned(),
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
    fn format_count_handles_singular_and_plural() {
        assert_eq!(format_count(1, "identity", "identities"), "1 identity");
        assert_eq!(format_count(2, "identity", "identities"), "2 identities");
    }
}
