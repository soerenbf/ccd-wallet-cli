use crate::{
    cli::{NodeInfoArgs, NodeSubcommand},
    config,
    store::config::load,
};
use anyhow::{Context, Result, bail};
use concordium_rust_sdk::v2;
use serde::Serialize;
use std::str::FromStr;

#[derive(Debug, Serialize)]
pub struct NodeInfoOutput {
    pub endpoint: String,
    pub node_info: String,
}

pub async fn run(command: NodeSubcommand) -> Result<()> {
    match command {
        NodeSubcommand::Info(args) => info(args).await,
    }
}

fn resolve_endpoint(
    network: Option<String>,
    node: Option<v2::Endpoint>,
) -> Result<(v2::Endpoint, String)> {
    match (network, node) {
        (Some(network_name), None) => {
            let app_config = load()?;
            let entry = app_config.networks.get(&network_name).with_context(|| {
                format!(
                    "network '{}' is not registered; run `ccd-wallet config network add --name {} --node <ENDPOINT>` first",
                    network_name, network_name
                )
            })?;

            let endpoint = v2::Endpoint::from_str(&entry.node_endpoint).with_context(|| {
                format!(
                    "network '{}' has an invalid stored endpoint: {}",
                    network_name, entry.node_endpoint
                )
            })?;

            Ok((endpoint, entry.node_endpoint.clone()))
        }
        (None, Some(node)) => {
            let label = config::endpoint_label(&node);
            Ok((node, label))
        }
        (Some(_), Some(_)) => bail!("--network and --node are mutually exclusive"),
        (None, None) => bail!("one of `--network` or `--node` is required"),
    }
}

async fn info(args: NodeInfoArgs) -> Result<()> {
    let (endpoint, endpoint_label) = resolve_endpoint(args.network, args.node)?;

    let mut client = v2::Client::new(endpoint)
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
