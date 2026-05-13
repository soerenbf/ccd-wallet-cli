use crate::{
    cli::{NodeInfoArgs, NodeSubcommand},
    config,
};
use anyhow::{Context, Result};
use concordium_rust_sdk::v2;
use serde::Serialize;

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

async fn info(args: NodeInfoArgs) -> Result<()> {
    let endpoint = config::endpoint_label(&args.node);

    let mut client = v2::Client::new(args.node)
        .await
        .with_context(|| format!("failed to connect to Concordium node at {endpoint}"))?;

    let node_info = client
        .get_node_info()
        .await
        .with_context(|| format!("failed to query node information from {endpoint}"))?;

    let output = NodeInfoOutput {
        endpoint,
        node_info: format!("{node_info:#?}"),
    };

    println!("Node endpoint: {}", output.endpoint);
    println!("Node info:\n{}", output.node_info);

    Ok(())
}
