//! Transaction inspection command.

use super::render::render_transaction_status;
use crate::{
    cli::TransactionShowArgs,
    commands::node::{ResolvedEndpoint, resolve_endpoint_context},
};
use anyhow::{Context, Result};
use ccd_wallet_core::config;
use chrono::SecondsFormat;
use concordium_rust_sdk::types::{TransactionStatus, hashes::BlockHash};
use rusqlite::Connection;
use std::collections::BTreeMap;

pub(super) async fn show(conn: &Connection, args: TransactionShowArgs) -> Result<()> {
    let resolved = resolve_endpoint_context(conn, args.network, args.node, args.no_defaults)?;
    let endpoint_label = resolved.endpoint_label.clone();
    let query_context = query_context_label(&resolved);

    let mut client = config::connect_v2_client(resolved.endpoint)
        .await
        .with_context(|| format!("failed to connect to Concordium node at {endpoint_label}"))?;

    let status = match client.get_block_item_status(&args.hash).await {
        Ok(status) => Some(status),
        Err(err) if err.is_not_found() => None,
        Err(err) => {
            return Err(err).with_context(|| {
                format!("failed to query transaction status from {endpoint_label}")
            });
        }
    };

    let block_times = fetch_block_times(&mut client, status.as_ref(), &endpoint_label).await?;

    println!(
        "{}",
        render_transaction_status(&args.hash, &query_context, status.as_ref(), &block_times)?
    );
    Ok(())
}

async fn fetch_block_times(
    client: &mut concordium_rust_sdk::v2::Client,
    status: Option<&TransactionStatus>,
    endpoint_label: &str,
) -> Result<BTreeMap<BlockHash, String>> {
    let mut block_times = BTreeMap::new();
    let blocks = match status {
        Some(TransactionStatus::Finalized(blocks)) | Some(TransactionStatus::Committed(blocks)) => {
            Some(blocks)
        }
        _ => None,
    };

    if let Some(blocks) = blocks {
        for block_hash in blocks.keys() {
            let block_info = client.get_block_info(*block_hash).await.with_context(|| {
                format!("failed to query block information for {block_hash} from {endpoint_label}")
            })?;
            block_times.insert(
                *block_hash,
                block_info
                    .response
                    .block_slot_time
                    .to_rfc3339_opts(SecondsFormat::Secs, true),
            );
        }
    }

    Ok(block_times)
}

fn query_context_label(resolved: &ResolvedEndpoint) -> String {
    match &resolved.network_name {
        Some(network_name) => format!("{network_name} @ {}", resolved.endpoint_label),
        None => resolved.endpoint_label.clone(),
    }
}
