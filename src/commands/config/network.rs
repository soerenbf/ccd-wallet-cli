use crate::{
    config,
    store::{
        config::{NetworkEntry, load, save},
        wallet_state,
    },
};
use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
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
    pub name: String,

    /// Concordium node gRPC endpoint to connect to.
    #[arg(long = "node", value_name = "ENDPOINT")]
    pub node: v2::Endpoint,
}

#[derive(Debug, Args)]
pub struct NetworkUseArgs {
    /// Name of a registered network to set as active.
    #[arg(value_name = "NAME")]
    pub name: String,
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn add(args: NetworkAddArgs) -> Result<()> {
    let endpoint_label = config::endpoint_label(&args.node);

    // Load existing config (initialises file if absent).
    let mut app_config = load()?;

    // Reject duplicate names before touching the node.
    if app_config.networks.contains_key(&args.name) {
        bail!(
            "network '{}' is already registered in the config; \
             use a different name or remove the existing entry first",
            args.name
        );
    }

    // Connect to the node.
    let mut client = v2::Client::new(args.node)
        .await
        .with_context(|| format!("failed to connect to Concordium node at {endpoint_label}"))?;

    // Derive the genesis hash from consensus info.
    let consensus_info = client
        .get_consensus_info()
        .await
        .with_context(|| format!("failed to query consensus info from node at {endpoint_label}"))?;

    let genesis_hash = format!("{}", consensus_info.genesis_block);

    // Persist — only after all fallible operations have succeeded.
    app_config.networks.insert(
        args.name.clone(),
        NetworkEntry {
            node_endpoint: endpoint_label.clone(),
            genesis_hash: genesis_hash.clone(),
        },
    );

    save(&app_config)?;

    println!("Network '{}' registered successfully.", args.name);
    println!("  endpoint:     {endpoint_label}");
    println!("  genesis hash: {genesis_hash}");

    Ok(())
}

pub async fn use_network(conn: &Connection, args: NetworkUseArgs) -> Result<()> {
    let app_config = load()?;

    if !app_config.networks.contains_key(&args.name) {
        bail!(
            "network '{}' is not registered; run `ccd-wallet network add --name {} --node <ENDPOINT>` first",
            args.name,
            args.name
        );
    }

    wallet_state::set(conn, wallet_state::ACTIVE_NETWORK_KEY, &args.name)?;

    println!("Active network set to '{}'.", args.name);

    Ok(())
}
