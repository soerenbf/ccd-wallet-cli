//! Contract module download command.

use crate::{
    cli::ContractDownloadModuleArgs,
    commands::account::resolve_account_network_context,
    smart_contracts::{query as query_core, shared},
};
use anyhow::{Context, Result, bail};
use ccd_wallet_core::config as node_config;
use rusqlite::Connection;
use std::fs;

pub(super) async fn download_module(
    conn: &Connection,
    args: ContractDownloadModuleArgs,
) -> Result<()> {
    if args.out.exists() && !args.force {
        bail!(
            "output file {} already exists; pass --force to overwrite it",
            args.out.display()
        );
    }
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
    let module_ref = args
        .module_ref
        .as_deref()
        .map(shared::parse_module_reference)
        .transpose()?;
    let contract = args
        .contract
        .as_deref()
        .map(shared::parse_contract_address)
        .transpose()?;
    let module_ref =
        query_core::resolve_module_ref(&mut client, block, module_ref, contract).await?;
    let module = query_core::get_module_source(&mut client, block, &module_ref).await?;
    let bytes = query_core::module_bytes(&module);
    fs::write(&args.out, bytes)
        .with_context(|| format!("failed to write module source to {}", args.out.display()))?;
    cliclack::log::success(format!(
        "Downloaded module {module_ref} to {}",
        args.out.display()
    ))?;
    Ok(())
}
