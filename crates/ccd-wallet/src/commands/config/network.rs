use crate::commands::ui::{SelectItem, select_or_single};
use anyhow::{Context, Result, bail};
use ccd_wallet_core::{
    config,
    store::{
        config::{NetworkEntry, load, save},
        wallet_state,
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

    println!("Active network set to '{}'.", name);

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

    let items = app_config
        .networks
        .iter()
        .map(|(name, entry)| SelectItem {
            value: name.clone(),
            label: name.clone(),
            hint: entry.node_endpoint.clone(),
        })
        .collect::<Vec<_>>();
    let active = wallet_state::get(conn, wallet_state::ACTIVE_NETWORK_KEY)?;
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
}
