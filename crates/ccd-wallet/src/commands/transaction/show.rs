//! Transaction inspection command.

use super::render::{
    SubmittedPayload, SubmittedPayloadDisplay, render_transaction_status_with_payloads,
};
use crate::{
    cli::TransactionShowArgs,
    commands::node::{ResolvedEndpoint, resolve_endpoint_context},
};
use anyhow::{Context, Result};
use ccd_wallet_core::config;
use chrono::SecondsFormat;
use concordium_rust_sdk::{
    types::{TransactionStatus, hashes::BlockHash},
    v2::BlockIdentifier,
};
use futures_util::TryStreamExt;
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
    let submitted_payloads = if args.show_payload {
        Some(fetch_submitted_payloads(&mut client, status.as_ref(), &endpoint_label).await?)
    } else {
        None
    };

    println!(
        "{}",
        render_transaction_status_with_payloads(
            &args.hash,
            &query_context,
            status.as_ref(),
            &block_times,
            submitted_payloads.as_ref(),
        )?
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

async fn fetch_submitted_payloads(
    client: &mut concordium_rust_sdk::v2::Client,
    status: Option<&TransactionStatus>,
    endpoint_label: &str,
) -> Result<SubmittedPayloadDisplay> {
    let blocks = match status {
        Some(TransactionStatus::Finalized(blocks)) | Some(TransactionStatus::Committed(blocks)) => {
            blocks
        }
        Some(TransactionStatus::Received) => {
            return Ok(SubmittedPayloadDisplay::Unavailable(
                "Original submitted transaction is not available until the transaction is included in a block."
                    .to_owned(),
            ));
        }
        None => return Ok(SubmittedPayloadDisplay::Absent),
    };

    let mut by_block = BTreeMap::new();
    for (block_hash, summary) in blocks {
        let response = client
            .get_block_items(BlockIdentifier::Given(*block_hash))
            .await
            .with_context(|| {
                format!("failed to query block items for {block_hash} from {endpoint_label}")
            })?;
        let mut stream = response.response;
        let mut payload = None;
        while let Some(block_item) = stream.try_next().await.with_context(|| {
            format!("failed while streaming block items for {block_hash} from {endpoint_label}")
        })? {
            if let Some(matching_payload) = submitted_payload_for_summary(block_item, summary.hash)
            {
                payload = Some(matching_payload);
                break;
            }
        }
        by_block.insert(
            *block_hash,
            payload.unwrap_or_else(|| {
                SubmittedPayload::Unavailable(
                    "Original submitted transaction was not found in the reported block contents."
                        .to_owned(),
                )
            }),
        );
    }

    Ok(SubmittedPayloadDisplay::ByBlock(by_block))
}

fn submitted_payload_for_summary(
    block_item: concordium_rust_sdk::v2::Upward<
        concordium_rust_sdk::base::transactions::BlockItem<
            concordium_rust_sdk::base::transactions::EncodedPayload,
        >,
    >,
    transaction_hash: concordium_rust_sdk::types::hashes::TransactionHash,
) -> Option<SubmittedPayload> {
    let block_item = block_item.known()?;
    (block_item.hash() == transaction_hash).then(|| SubmittedPayload::Known(Box::new(block_item)))
}

fn query_context_label(resolved: &ResolvedEndpoint) -> String {
    match &resolved.network_name {
        Some(network_name) => format!("{network_name} @ {}", resolved.endpoint_label),
        None => resolved.endpoint_label.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use concordium_rust_sdk::{
        common::types::{Amount, TransactionSignature, TransactionTime},
        id::types::AccountAddress,
        types::{
            Nonce,
            transactions::{
                AccountTransaction, BlockItem, Payload, PayloadLike, PayloadSize, TransactionHeader,
            },
        },
        v2::Upward,
    };
    use std::{collections::BTreeMap, str::FromStr};

    fn account_address() -> AccountAddress {
        AccountAddress::from_str("47b6Qe2XtZANHetanWKP1PbApLKtS3AyiCtcXaqLMbypKjCaRw").unwrap()
    }

    fn account_transfer_block_item()
    -> BlockItem<concordium_rust_sdk::base::transactions::EncodedPayload> {
        let payload = Payload::Transfer {
            to_address: account_address(),
            amount: Amount::from_micro_ccd(42),
        };
        let encoded = payload.encode();
        BlockItem::AccountTransaction(AccountTransaction {
            signature: TransactionSignature {
                signatures: BTreeMap::new(),
            },
            header: TransactionHeader {
                sender: account_address(),
                nonce: Nonce { nonce: 1 },
                energy_amount: 501u64.into(),
                payload_size: PayloadSize::from(encoded.as_ref().len() as u32),
                expiry: TransactionTime {
                    seconds: 1_716_000_100,
                },
            },
            payload: encoded,
        })
    }

    #[test]
    fn submitted_payload_for_summary_matches_by_transaction_hash() {
        let block_item = account_transfer_block_item();
        let transaction_hash = block_item.hash();

        let matched = submitted_payload_for_summary(Upward::Known(block_item), transaction_hash);

        assert!(matches!(matched, Some(SubmittedPayload::Known(_))));
    }

    #[test]
    fn submitted_payload_for_summary_ignores_unknown_and_non_matching_items() {
        let block_item = account_transfer_block_item();
        let other_hash = concordium_rust_sdk::types::hashes::TransactionHash::from([9u8; 32]);

        assert!(submitted_payload_for_summary(Upward::Known(block_item), other_hash).is_none());
        assert!(submitted_payload_for_summary(Upward::Unknown(()), other_hash).is_none());
    }
}
