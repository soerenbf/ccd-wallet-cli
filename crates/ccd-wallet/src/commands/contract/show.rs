//! Contract instance information command.

use crate::{
    cli::ContractShowArgs,
    commands::account::resolve_account_network_context,
    smart_contracts::{query as query_core, shared},
};
use anyhow::{Context, Result};
use ccd_wallet_core::config as node_config;
use rusqlite::Connection;

pub(super) async fn show(conn: &Connection, args: ContractShowArgs) -> Result<()> {
    let (_network_name, _network_entry, endpoint, endpoint_label, _network_source) =
        resolve_account_network_context(
            conn,
            args.network.as_deref(),
            args.node,
            false,
            args.no_defaults,
        )
        .await?;
    let mut client = node_config::connect_v2_client(endpoint.clone())
        .await
        .with_context(|| format!("failed to connect to Concordium node at {endpoint_label}"))?;
    let block = shared::parse_block_identifier(args.block.as_deref())?;
    let contract = shared::parse_contract_address(&args.contract)?;
    let info = query_core::get_instance_info(&mut client, block, contract).await?;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&query_core::instance_info_json(contract, &info))?
        );
    } else {
        println!("{}", query_core::render_instance_info(contract, &info));
    }
    Ok(())
}
