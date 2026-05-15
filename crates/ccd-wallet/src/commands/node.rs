use crate::cli::{NodeInfoArgs, NodeSubcommand};
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

pub(crate) fn resolve_endpoint(
    conn: &Connection,
    network: Option<String>,
    node: Option<v2::Endpoint>,
) -> Result<(v2::Endpoint, String)> {
    match (network, node) {
        (Some(network_name), None) => resolve_registered_network(&network_name),
        (None, Some(node)) => {
            let label = config::endpoint_label(&node);
            Ok((node, label))
        }
        (Some(_), Some(_)) => bail!("--network and --node are mutually exclusive"),
        (None, None) => {
            let active_network = wallet_state::get(conn, wallet_state::ACTIVE_NETWORK_KEY)?.with_context(|| {
                "no active network is set; provide `--network` or `--node`, or run `ccd-wallet network use <NAME>`"
            })?;

            resolve_registered_network(&active_network).with_context(|| {
                format!(
                    "active network '{}' is no longer registered; update it with `ccd-wallet network use <NAME>` or provide `--network` / `--node` explicitly",
                    active_network
                )
            })
        }
    }
}

async fn info(conn: &Connection, args: NodeInfoArgs) -> Result<()> {
    let (endpoint, endpoint_label) = resolve_endpoint(conn, args.network, args.node)?;

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
