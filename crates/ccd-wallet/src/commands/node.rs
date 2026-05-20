use crate::{
    cli::{NodeInfoArgs, NodeSubcommand},
    commands::ui::{SelectItem, select_or_single},
};
use anyhow::{Context, Result, bail};
use ccd_wallet_core::{
    config,
    store::{config::load, wallet_state},
};
use concordium_rust_sdk::v2;
use rusqlite::Connection;
use serde::Serialize;
use std::str::FromStr;

#[derive(Debug, Serialize)]
pub struct NodeInfoOutput {
    pub endpoint: String,
    pub node_info: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedEndpoint {
    pub endpoint: v2::Endpoint,
    pub endpoint_label: String,
    pub network_name: Option<String>,
}

pub async fn run(conn: &Connection, command: NodeSubcommand) -> Result<()> {
    match command {
        NodeSubcommand::Info(args) => info(conn, args).await,
    }
}

fn resolve_registered_network(network_name: &str) -> Result<(v2::Endpoint, String)> {
    let app_config = load()?;
    let entry = app_config.networks.get(network_name).with_context(|| {
        format!(
            "network '{}' is not registered; run `ccd-wallet network add --name {} --node <ENDPOINT>` first",
            network_name, network_name
        )
    })?;

    let endpoint = v2::Endpoint::from_str(&config::normalize_url_string(&entry.node_endpoint))
        .with_context(|| {
            format!(
                "network '{}' has an invalid stored endpoint: {}",
                network_name, entry.node_endpoint
            )
        })?;

    Ok((endpoint, entry.node_endpoint.clone()))
}

pub(crate) fn resolve_endpoint_context(
    conn: &Connection,
    network: Option<String>,
    node: Option<v2::Endpoint>,
    no_defaults: bool,
) -> Result<ResolvedEndpoint> {
    match (network, node) {
        (Some(network_name), None) => {
            let (endpoint, endpoint_label) = resolve_registered_network(&network_name)?;
            Ok(ResolvedEndpoint {
                endpoint,
                endpoint_label,
                network_name: Some(network_name),
            })
        }
        (None, Some(node)) => {
            let endpoint_label = config::endpoint_label(&node);
            Ok(ResolvedEndpoint {
                endpoint: node,
                endpoint_label,
                network_name: None,
            })
        }
        (Some(_), Some(_)) => bail!("--network and --node are mutually exclusive"),
        (None, None) => {
            let app_config = load()?;
            let active_network = wallet_state::get(conn, wallet_state::ACTIVE_NETWORK_KEY)?;
            if no_defaults {
                let selected_network =
                    prompt_for_network_name(&app_config, active_network.as_deref())?;
                let (endpoint, endpoint_label) = resolve_registered_network(&selected_network)?;
                Ok(ResolvedEndpoint {
                    endpoint,
                    endpoint_label,
                    network_name: Some(selected_network),
                })
            } else {
                let active_network = active_network.with_context(|| {
                    "no active network is set; provide `--network` or `--node`, or run `ccd-wallet network use <NAME>`"
                })?;

                let (endpoint, endpoint_label) = resolve_registered_network(&active_network)
                    .with_context(|| {
                        format!(
                            "active network '{}' is no longer registered; update it with `ccd-wallet network use <NAME>` or provide `--network` / `--node` explicitly",
                            active_network
                        )
                    })?;
                Ok(ResolvedEndpoint {
                    endpoint,
                    endpoint_label,
                    network_name: Some(active_network),
                })
            }
        }
    }
}

fn prompt_for_network_name(
    app_config: &ccd_wallet_core::store::config::AppConfig,
    active: Option<&str>,
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
            hint: entry.node_endpoint.to_string(),
        })
        .collect::<Vec<_>>();
    let initial = active.map(str::to_owned);
    select_or_single("Select network", &items, initial.as_ref())
}

async fn info(conn: &Connection, args: NodeInfoArgs) -> Result<()> {
    let ResolvedEndpoint {
        endpoint,
        endpoint_label,
        ..
    } = resolve_endpoint_context(conn, args.network, args.node, args.no_defaults)?;

    let mut client = config::connect_v2_client(endpoint)
        .await
        .with_context(|| format!("failed to connect to Concordium node at {endpoint_label}"))?;

    let node_info = client
        .get_node_info()
        .await
        .with_context(|| format!("failed to query node information from {endpoint_label}"))?;

    let output = NodeInfoOutput {
        endpoint: endpoint_label,
        node_info: format!("{node_info:#?}"),
    };

    println!("Node endpoint: {}", output.endpoint);
    println!("Node info:\n{}", output.node_info);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccd_wallet_core::store::{config as store_config, migrations, wallet_state};
    use rusqlite::Connection;
    use std::{
        fs,
        path::PathBuf,
        str::FromStr,
        sync::{Mutex, OnceLock},
        time::{SystemTime, UNIX_EPOCH},
    };

    fn home_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn temp_home_dir() -> PathBuf {
        std::env::temp_dir().join(format!(
            "ccd-wallet-node-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn resolve_endpoint_context_uses_explicit_node() {
        let conn = Connection::open_in_memory().unwrap();
        let endpoint = v2::Endpoint::from_str("https://grpc.testnet.concordium.com:20000").unwrap();

        let resolved =
            resolve_endpoint_context(&conn, None, Some(endpoint.clone()), false).unwrap();

        assert!(resolved.network_name.is_none());
        assert_eq!(
            resolved.endpoint_label,
            "https://grpc.testnet.concordium.com:20000"
        );
    }

    #[test]
    fn resolve_endpoint_context_uses_active_network_by_default() {
        let _guard = home_lock().lock().unwrap();
        let previous_home = std::env::var_os("HOME");
        let home = temp_home_dir();
        fs::create_dir_all(&home).unwrap();
        unsafe { std::env::set_var("HOME", &home) };

        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        let mut app_config = store_config::AppConfig::default();
        app_config.networks.insert(
            "testnet".to_owned(),
            store_config::NetworkEntry {
                node_endpoint: "https://grpc.testnet.concordium.com:20000".to_owned(),
                genesis_hash: "abc".to_owned(),
                wallet_proxy: Some("https://wallet-proxy.testnet.concordium.com".to_owned()),
            },
        );
        store_config::save(&app_config).unwrap();
        wallet_state::set(&conn, wallet_state::ACTIVE_NETWORK_KEY, "testnet").unwrap();

        let resolved = resolve_endpoint_context(&conn, None, None, false).unwrap();

        assert_eq!(resolved.network_name.as_deref(), Some("testnet"));
        assert_eq!(
            resolved.endpoint_label,
            "https://grpc.testnet.concordium.com:20000"
        );

        if let Some(previous_home) = previous_home {
            unsafe { std::env::set_var("HOME", previous_home) };
        } else {
            unsafe { std::env::remove_var("HOME") };
        }
        fs::remove_dir_all(&home).unwrap();
    }
}
