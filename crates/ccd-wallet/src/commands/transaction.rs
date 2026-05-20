use crate::{
    cli::{TransactionShowArgs, TransactionSubcommand},
    commands::node::{ResolvedEndpoint, resolve_endpoint_context},
};
use anyhow::{Context, Result, anyhow, bail};
use ccd_wallet_core::config;
use chrono::{DateTime, SecondsFormat, Utc};
use concordium_rust_sdk::types::{
    AccountCreationDetails, AccountTransactionDetails, BlockItemSummary, BlockItemSummaryDetails,
    TokenCreationDetails, TransactionStatus, UpdateDetails,
    hashes::{BlockHash, TransactionHash},
};
use rusqlite::Connection;
use std::collections::BTreeMap;

pub async fn run(conn: &Connection, command: TransactionSubcommand) -> Result<()> {
    match command {
        TransactionSubcommand::Show(args) => show(conn, *args).await,
    }
}

async fn show(conn: &Connection, args: TransactionShowArgs) -> Result<()> {
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

fn render_transaction_status(
    hash: &TransactionHash,
    query_context: &str,
    status: Option<&TransactionStatus>,
    block_times: &BTreeMap<BlockHash, String>,
) -> Result<String> {
    let mut lines = vec![
        "Metadata:".to_owned(),
        format!("  Transaction: {hash}"),
        format!("  Queried via: {query_context}"),
    ];

    match status {
        None => {
            lines.push("  Status: absent".to_owned());
            lines.push(String::new());
            lines.push("The node does not know this transaction hash.".to_owned());
            lines.push("Note: this can also mean you queried the wrong network/node.".to_owned());
        }
        Some(TransactionStatus::Received) => {
            lines.push("  Status: received".to_owned());
        }
        Some(TransactionStatus::Finalized(blocks)) => {
            lines.push("  Status: finalized".to_owned());
            let (block_hash, summary) = single_block_summary(blocks, "finalized")?;
            lines.extend(render_block_summary_sections(
                block_hash,
                summary,
                block_times.get(block_hash),
                false,
            )?);
        }
        Some(TransactionStatus::Committed(blocks)) => {
            lines.push("  Status: committed".to_owned());
            lines.push(format!("  Blocks: {}", blocks.len()));
            if !blocks.is_empty() {
                for (index, (block_hash, summary)) in blocks.iter().enumerate() {
                    lines.push(String::new());
                    lines.push(format!("Block {}:", index + 1));
                    lines.extend(render_block_summary_sections(
                        block_hash,
                        summary,
                        block_times.get(block_hash),
                        true,
                    )?);
                }
            }
        }
    }

    Ok(lines.join("\n"))
}

fn single_block_summary<'a>(
    blocks: &'a BTreeMap<BlockHash, BlockItemSummary>,
    label: &str,
) -> Result<(&'a BlockHash, &'a BlockItemSummary)> {
    let mut iter = blocks.iter();
    let Some(first) = iter.next() else {
        bail!("{label} transaction summary was empty")
    };
    if iter.next().is_some() {
        bail!("{label} transaction unexpectedly included multiple block summaries")
    }
    Ok(first)
}

fn render_block_summary_sections(
    block_hash: &BlockHash,
    summary: &BlockItemSummary,
    block_time: Option<&String>,
    include_metadata_header: bool,
) -> Result<Vec<String>> {
    let mut lines = Vec::new();

    if include_metadata_header {
        lines.push("Metadata:".to_owned());
    }
    lines.push(format!("  Block: {block_hash}"));
    if let Some(block_time) = block_time {
        lines.push(format!("  Block time: {block_time}"));
    }

    let mut details = vec![
        format!(
            "Outcome: {}",
            if summary.is_success().known().unwrap_or(false) {
                "success"
            } else {
                "rejected"
            }
        ),
        format!("Type: {}", summary_type_label(summary)),
        format!("Energy: {} NRG", summary.energy_cost),
    ];

    let variant = match summary.details.as_known() {
        Some(BlockItemSummaryDetails::AccountTransaction(details)) => {
            render_account_transaction(summary, details)?
        }
        Some(BlockItemSummaryDetails::AccountCreation(details)) => {
            VariantRender::details_only(render_account_creation(details)?)
        }
        Some(BlockItemSummaryDetails::Update(details)) => render_update(details)?,
        Some(BlockItemSummaryDetails::TokenCreationDetails(details)) => {
            render_token_creation(details)?
        }
        None => VariantRender::details_only(vec!["unknown".to_owned()]),
    };

    details.extend(variant.details);

    lines.push(String::new());
    lines.push("Details:".to_owned());
    for detail in details {
        lines.push(format!("  {detail}"));
    }

    if let Some((heading, body)) = variant.payload_section {
        lines.push(String::new());
        lines.push(heading);
        lines.push(body);
    }

    Ok(lines)
}

fn summary_type_label(summary: &BlockItemSummary) -> String {
    match summary.details.as_known() {
        Some(BlockItemSummaryDetails::AccountTransaction(details)) => details
            .transaction_type()
            .and_then(|kind| kind.known())
            .map(|kind| kind.to_string())
            .unwrap_or_else(|| "Account transaction".to_owned()),
        Some(BlockItemSummaryDetails::AccountCreation(_)) => "Credential deployment".to_owned(),
        Some(BlockItemSummaryDetails::Update(_)) => "Chain update".to_owned(),
        Some(BlockItemSummaryDetails::TokenCreationDetails(_)) => "Token creation".to_owned(),
        None => "unknown".to_owned(),
    }
}

struct VariantRender {
    details: Vec<String>,
    payload_section: Option<(String, String)>,
}

impl VariantRender {
    fn details_only(details: Vec<String>) -> Self {
        Self {
            details,
            payload_section: None,
        }
    }
}

fn render_account_transaction(
    summary: &BlockItemSummary,
    details: &AccountTransactionDetails,
) -> Result<VariantRender> {
    let mut stable = vec![
        format!("Sender: {}", details.sender),
        format!("Cost: {}", details.cost),
    ];

    if let Some(sponsor) = &details.sponsor {
        stable.push(format!(
            "Sponsor: {} (cost: {})",
            sponsor.sponsor, sponsor.cost
        ));
    }

    let Some(effects) = details.effects.as_known() else {
        stable.push("Effects: unknown".to_owned());
        return Ok(VariantRender::details_only(stable));
    };

    if let Some(reject_reason) = effects.is_rejected().and_then(|reason| reason.known()) {
        return Ok(VariantRender {
            details: stable,
            payload_section: Some((
                "Reject reason:".to_owned(),
                serde_json::to_string_pretty(reject_reason)
                    .context("failed to pretty-print transaction reject reason")?,
            )),
        });
    }

    let (count, json) = account_transaction_events_json(summary)?;
    Ok(VariantRender {
        details: stable,
        payload_section: Some((format!("Events: {count}"), json)),
    })
}

fn render_account_creation(details: &AccountCreationDetails) -> Result<Vec<String>> {
    Ok(vec![
        format!("Credential type: {:?}", details.credential_type),
        format!("Address: {}", details.address),
        format!("Registration ID: {}", details.reg_id),
    ])
}

fn render_update(details: &UpdateDetails) -> Result<VariantRender> {
    let effective_time = format_transaction_time(details.effective_time.seconds)
        .unwrap_or_else(|| details.effective_time.seconds.to_string());
    let update_type = details
        .update_type()
        .known()
        .map(|kind| format!("{kind:?}"))
        .unwrap_or_else(|| "unknown".to_owned());

    let payload_section = match details.payload.as_known() {
        Some(payload) => Some((
            "Payload:".to_owned(),
            serde_json::to_string_pretty(payload)
                .context("failed to pretty-print update payload")?,
        )),
        None => Some(("Payload:".to_owned(), "unknown".to_owned())),
    };

    Ok(VariantRender {
        details: vec![
            format!("Effective time: {effective_time}"),
            format!("Update type: {update_type}"),
        ],
        payload_section,
    })
}

fn render_token_creation(details: &TokenCreationDetails) -> Result<VariantRender> {
    Ok(VariantRender {
        details: vec![
            format!("Token ID: {:?}", details.create_plt.token_id),
            format!("Token module: {:?}", details.create_plt.token_module),
            format!("Decimals: {}", details.create_plt.decimals),
        ],
        payload_section: Some((
            format!("Events: {}", details.events.len()),
            serde_json::to_string_pretty(&details.events)
                .context("failed to pretty-print token-creation events")?,
        )),
    })
}

fn account_transaction_events_json(summary: &BlockItemSummary) -> Result<(usize, String)> {
    let summary_json = serde_json::to_value(summary)
        .context("failed to serialize successful transaction summary to JSON")?;
    let events = summary_json
        .get("result")
        .and_then(|result| result.get("events"))
        .cloned()
        .ok_or_else(|| anyhow!("successful transaction result missing events"))?;
    let count = events
        .as_array()
        .map(Vec::len)
        .ok_or_else(|| anyhow!("successful transaction events payload was not an array"))?;
    let json = serde_json::to_string_pretty(&events)
        .context("failed to pretty-print transaction events")?;
    Ok((count, json))
}

fn format_transaction_time(seconds: u64) -> Option<String> {
    let seconds = i64::try_from(seconds).ok()?;
    let dt: DateTime<Utc> = DateTime::from_timestamp(seconds, 0)?;
    Some(dt.to_rfc3339_opts(SecondsFormat::Secs, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use concordium_rust_sdk::{
        common::types::Amount,
        id::types::AccountAddress,
        protocol_level_tokens::{TokenId, TokenModuleRef},
        types::{
            AccountCreationDetails, AccountTransactionDetails, AccountTransactionEffects,
            BlockItemSummaryDetails, CredentialRegistrationID, RejectReason, TokenCreationDetails,
            TransactionIndex, TransactionType, UpdateDetails, UpdatePayload,
        },
        v2::Upward,
    };
    use std::str::FromStr;

    fn transaction_hash(hex: &str) -> TransactionHash {
        TransactionHash::from_str(hex).unwrap()
    }

    fn block_hash(hex: &str) -> BlockHash {
        BlockHash::from_str(hex).unwrap()
    }

    fn account_address() -> AccountAddress {
        AccountAddress::from_str("47b6Qe2XtZANHetanWKP1PbApLKtS3AyiCtcXaqLMbypKjCaRw").unwrap()
    }

    fn rejected_summary(hash: &str) -> BlockItemSummary {
        BlockItemSummary {
            index: TransactionIndex { index: 0 },
            energy_cost: 112u64.into(),
            hash: transaction_hash(hash),
            details: Upward::Known(BlockItemSummaryDetails::AccountTransaction(
                AccountTransactionDetails {
                    cost: Amount::from_micro_ccd(11_200),
                    sender: account_address(),
                    sponsor: None,
                    effects: Upward::Known(AccountTransactionEffects::None {
                        transaction_type: Some(TransactionType::Transfer),
                        reject_reason: Upward::Known(RejectReason::SerializationFailure),
                    }),
                },
            )),
        }
    }

    fn success_summary(hash: &str) -> BlockItemSummary {
        BlockItemSummary {
            index: TransactionIndex { index: 1 },
            energy_cost: 113u64.into(),
            hash: transaction_hash(hash),
            details: Upward::Known(BlockItemSummaryDetails::AccountTransaction(
                AccountTransactionDetails {
                    cost: Amount::from_micro_ccd(11_300),
                    sender: account_address(),
                    sponsor: None,
                    effects: Upward::Known(AccountTransactionEffects::AccountTransfer {
                        amount: Amount::from_micro_ccd(42),
                        to: account_address(),
                    }),
                },
            )),
        }
    }

    fn account_creation_summary(hash: &str) -> BlockItemSummary {
        BlockItemSummary {
            index: TransactionIndex { index: 2 },
            energy_cost: 114u64.into(),
            hash: transaction_hash(hash),
            details: Upward::Known(BlockItemSummaryDetails::AccountCreation(
                AccountCreationDetails {
                    credential_type: concordium_rust_sdk::types::CredentialType::Initial,
                    address: account_address(),
                    reg_id: CredentialRegistrationID::from_str(
                        "8a3a87f3f38a7a507d1e85dc02a92b8bcaa859f5cf56accb3c1bc7c40e1789b4933875a38dd4c0646ca3e940a02c42d8",
                    )
                    .unwrap(),
                },
            )),
        }
    }

    fn update_summary(hash: &str) -> BlockItemSummary {
        BlockItemSummary {
            index: TransactionIndex { index: 3 },
            energy_cost: 0u64.into(),
            hash: transaction_hash(hash),
            details: Upward::Known(BlockItemSummaryDetails::Update(UpdateDetails {
                effective_time: 1_716_000_000u64.into(),
                payload: Upward::Known(UpdatePayload::FoundationAccount(account_address())),
            })),
        }
    }

    fn token_creation_summary(hash: &str) -> BlockItemSummary {
        BlockItemSummary {
            index: TransactionIndex { index: 4 },
            energy_cost: 115u64.into(),
            hash: transaction_hash(hash),
            details: Upward::Known(BlockItemSummaryDetails::TokenCreationDetails(
                TokenCreationDetails {
                    create_plt: concordium_rust_sdk::types::CreatePlt {
                        token_id: TokenId::from_str("TEST").unwrap(),
                        token_module: TokenModuleRef::from([7u8; 32]),
                        decimals: 6,
                        initialization_parameters: vec![1, 2, 3].into(),
                    },
                    events: Vec::new(),
                },
            )),
        }
    }

    #[test]
    fn received_output_has_no_detail_sections() {
        let hash =
            transaction_hash("0fda6e284f9cd4429c6f76fd1bf6179aad4fa1bb218fe5ec8ad33916bf84a833");
        let rendered = render_transaction_status(
            &hash,
            "testnet @ https://grpc.testnet.concordium.com:20000",
            Some(&TransactionStatus::Received),
            &BTreeMap::new(),
        )
        .unwrap();

        assert!(rendered.contains("Metadata:"));
        assert!(rendered.contains("Status: received"));
        assert!(!rendered.contains("Details:"));
        assert!(!rendered.contains("Events:"));
        assert!(!rendered.contains("Reject reason:"));
    }

    #[test]
    fn absent_output_includes_network_guidance() {
        let hash =
            transaction_hash("0fda6e284f9cd4429c6f76fd1bf6179aad4fa1bb218fe5ec8ad33916bf84a833");
        let rendered = render_transaction_status(
            &hash,
            "testnet @ https://grpc.testnet.concordium.com:20000",
            None,
            &BTreeMap::new(),
        )
        .unwrap();

        assert!(rendered.contains("Metadata:"));
        assert!(rendered.contains("Status: absent"));
        assert!(rendered.contains("wrong network/node"));
    }

    #[test]
    fn finalized_account_transaction_shows_static_fields_and_reject_reason() {
        let hash =
            transaction_hash("0fda6e284f9cd4429c6f76fd1bf6179aad4fa1bb218fe5ec8ad33916bf84a833");
        let block = block_hash("e2a12d06273f5641ea8157e04367eae49a72706aa831aa58b60ee5c062cdd6e2");
        let status = TransactionStatus::Finalized(BTreeMap::from([(
            block,
            rejected_summary(&hash.to_string()),
        )]));
        let block_times = BTreeMap::from([(block, "2026-05-19T14:23:11Z".to_owned())]);

        let rendered = render_transaction_status(
            &hash,
            "testnet @ https://grpc.testnet.concordium.com:20000",
            Some(&status),
            &block_times,
        )
        .unwrap();

        assert!(rendered.contains("Metadata:"));
        assert!(rendered.contains("Status: finalized"));
        assert!(
            rendered.contains(
                "Block: e2a12d06273f5641ea8157e04367eae49a72706aa831aa58b60ee5c062cdd6e2"
            )
        );
        assert!(rendered.contains("Block time: 2026-05-19T14:23:11Z"));
        assert!(rendered.contains("Details:"));
        assert!(rendered.contains("Type: Transfer"));
        assert!(rendered.contains("Sender:"));
        assert!(rendered.contains("Cost:"));
        assert!(rendered.contains("Reject reason:"));
        assert!(rendered.contains("\"tag\": \"SerializationFailure\""));
        assert!(!rendered.contains("Block metadata:"));
    }

    #[test]
    fn finalized_account_transaction_shows_events_count_for_success() {
        let hash =
            transaction_hash("0fda6e284f9cd4429c6f76fd1bf6179aad4fa1bb218fe5ec8ad33916bf84a833");
        let block = block_hash("e2a12d06273f5641ea8157e04367eae49a72706aa831aa58b60ee5c062cdd6e2");
        let status = TransactionStatus::Finalized(BTreeMap::from([(
            block,
            success_summary(&hash.to_string()),
        )]));
        let block_times = BTreeMap::from([(block, "2026-05-19T14:23:11Z".to_owned())]);

        let rendered = render_transaction_status(
            &hash,
            "testnet @ https://grpc.testnet.concordium.com:20000",
            Some(&status),
            &block_times,
        )
        .unwrap();

        assert!(rendered.contains("Details:"));
        assert!(rendered.contains("Outcome: success"));
        assert!(rendered.contains("Events: 1"));
        assert!(rendered.contains("\"tag\": \"Transferred\""));
    }

    #[test]
    fn finalized_account_creation_shows_static_fields_only() {
        let hash =
            transaction_hash("0fda6e284f9cd4429c6f76fd1bf6179aad4fa1bb218fe5ec8ad33916bf84a833");
        let block = block_hash("e2a12d06273f5641ea8157e04367eae49a72706aa831aa58b60ee5c062cdd6e2");
        let status = TransactionStatus::Finalized(BTreeMap::from([(
            block,
            account_creation_summary(&hash.to_string()),
        )]));
        let rendered = render_transaction_status(
            &hash,
            "testnet @ https://grpc.testnet.concordium.com:20000",
            Some(&status),
            &BTreeMap::from([(block, "2026-05-19T14:23:11Z".to_owned())]),
        )
        .unwrap();

        assert!(rendered.contains("Details:"));
        assert!(rendered.contains("Outcome: success"));
        assert!(rendered.contains("Type: Credential deployment"));
        assert!(rendered.contains("Credential type:"));
        assert!(rendered.contains("Registration ID:"));
        assert!(!rendered.contains("Payload:"));
        assert!(!rendered.contains("Events:"));
    }

    #[test]
    fn finalized_update_shows_payload_json() {
        let hash =
            transaction_hash("0fda6e284f9cd4429c6f76fd1bf6179aad4fa1bb218fe5ec8ad33916bf84a833");
        let block = block_hash("e2a12d06273f5641ea8157e04367eae49a72706aa831aa58b60ee5c062cdd6e2");
        let status = TransactionStatus::Finalized(BTreeMap::from([(
            block,
            update_summary(&hash.to_string()),
        )]));
        let rendered = render_transaction_status(
            &hash,
            "testnet @ https://grpc.testnet.concordium.com:20000",
            Some(&status),
            &BTreeMap::from([(block, "2026-05-19T14:23:11Z".to_owned())]),
        )
        .unwrap();

        assert!(rendered.contains("Details:"));
        assert!(rendered.contains("Outcome: success"));
        assert!(rendered.contains("Type: Chain update"));
        assert!(rendered.contains("Effective time:"));
        assert!(rendered.contains("Update type:"));
        assert!(rendered.contains("Payload:"));
    }

    #[test]
    fn finalized_token_creation_shows_static_fields_and_events() {
        let hash =
            transaction_hash("0fda6e284f9cd4429c6f76fd1bf6179aad4fa1bb218fe5ec8ad33916bf84a833");
        let block = block_hash("e2a12d06273f5641ea8157e04367eae49a72706aa831aa58b60ee5c062cdd6e2");
        let status = TransactionStatus::Finalized(BTreeMap::from([(
            block,
            token_creation_summary(&hash.to_string()),
        )]));
        let rendered = render_transaction_status(
            &hash,
            "testnet @ https://grpc.testnet.concordium.com:20000",
            Some(&status),
            &BTreeMap::from([(block, "2026-05-19T14:23:11Z".to_owned())]),
        )
        .unwrap();

        assert!(rendered.contains("Details:"));
        assert!(rendered.contains("Outcome: success"));
        assert!(rendered.contains("Type: Token creation"));
        assert!(rendered.contains("Token ID:"));
        assert!(rendered.contains("Decimals: 6"));
        assert!(rendered.contains("Events: 0"));
    }
}
