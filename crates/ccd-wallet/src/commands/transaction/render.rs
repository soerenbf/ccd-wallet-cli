//! Shared human-oriented transaction summary rendering.

use anyhow::{Context, Result, anyhow, bail};
use chrono::{DateTime, SecondsFormat, Utc};
use concordium_rust_sdk::{
    base::transactions::{BlockItem, EncodedPayload},
    types::{
        AccountCreationDetails, AccountTransactionDetails, BlockItemSummary,
        BlockItemSummaryDetails, TokenCreationDetails, TransactionStatus, UpdateDetails,
        hashes::{BlockHash, TransactionHash},
    },
};
use std::collections::BTreeMap;

#[derive(Debug)]
pub(crate) enum SubmittedPayloadDisplay {
    Absent,
    Unavailable(String),
    ByBlock(BTreeMap<BlockHash, SubmittedPayload>),
}

impl SubmittedPayloadDisplay {
    fn for_block(&self, block_hash: &BlockHash) -> Option<&SubmittedPayload> {
        match self {
            Self::ByBlock(payloads) => payloads.get(block_hash),
            Self::Absent | Self::Unavailable(_) => None,
        }
    }
}

#[derive(Debug)]
pub(crate) enum SubmittedPayload {
    Known(Box<BlockItem<EncodedPayload>>),
    Unavailable(String),
}

#[cfg(test)]
pub(crate) fn render_transaction_status(
    hash: &TransactionHash,
    query_context: &str,
    status: Option<&TransactionStatus>,
    block_times: &BTreeMap<BlockHash, String>,
) -> Result<String> {
    render_transaction_status_with_payloads(hash, query_context, status, block_times, None)
}

pub(crate) fn render_transaction_status_with_payloads(
    hash: &TransactionHash,
    query_context: &str,
    status: Option<&TransactionStatus>,
    block_times: &BTreeMap<BlockHash, String>,
    submitted_payloads: Option<&SubmittedPayloadDisplay>,
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
            if let Some(SubmittedPayloadDisplay::Unavailable(message)) = submitted_payloads {
                lines.push(String::new());
                lines.push("Submitted transaction:".to_owned());
                lines.push(format!("  {message}"));
            }
        }
        Some(TransactionStatus::Finalized(blocks)) => {
            lines.push("  Status: finalized".to_owned());
            let (block_hash, summary) = single_block_summary(blocks, "finalized")?;
            lines.extend(render_block_summary_sections_with_payload(
                block_hash,
                summary,
                block_times.get(block_hash),
                false,
                submitted_payloads.and_then(|payloads| payloads.for_block(block_hash)),
            )?);
        }
        Some(TransactionStatus::Committed(blocks)) => {
            lines.push("  Status: committed".to_owned());
            lines.push(format!("  Blocks: {}", blocks.len()));
            if !blocks.is_empty() {
                for (index, (block_hash, summary)) in blocks.iter().enumerate() {
                    lines.push(String::new());
                    lines.push(format!("Block {}:", index + 1));
                    lines.extend(render_block_summary_sections_with_payload(
                        block_hash,
                        summary,
                        block_times.get(block_hash),
                        true,
                        submitted_payloads.and_then(|payloads| payloads.for_block(block_hash)),
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

pub(crate) fn render_finalized_summary(
    transaction_hash: &TransactionHash,
    query_context: &str,
    block_hash: &BlockHash,
    summary: &BlockItemSummary,
    block_time: Option<&String>,
) -> Result<String> {
    let mut lines = vec![
        "Metadata:".to_owned(),
        format!("  Transaction: {transaction_hash}"),
        format!("  Queried via: {query_context}"),
        "  Status: finalized".to_owned(),
    ];
    lines.extend(render_block_summary_sections(
        block_hash, summary, block_time, false,
    )?);
    Ok(lines.join("\n"))
}

pub(crate) fn render_block_summary_sections(
    block_hash: &BlockHash,
    summary: &BlockItemSummary,
    block_time: Option<&String>,
    include_metadata_header: bool,
) -> Result<Vec<String>> {
    render_block_summary_sections_with_payload(
        block_hash,
        summary,
        block_time,
        include_metadata_header,
        None,
    )
}

fn render_block_summary_sections_with_payload(
    block_hash: &BlockHash,
    summary: &BlockItemSummary,
    block_time: Option<&String>,
    include_metadata_header: bool,
    submitted_payload: Option<&SubmittedPayload>,
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

    if let Some(submitted_payload) = submitted_payload {
        lines.push(String::new());
        lines.push("Submitted transaction:".to_owned());
        lines.push(render_submitted_payload(submitted_payload)?);
    }

    Ok(lines)
}

fn render_submitted_payload(payload: &SubmittedPayload) -> Result<String> {
    match payload {
        SubmittedPayload::Known(block_item) => render_submitted_block_item(block_item),
        SubmittedPayload::Unavailable(message) => Ok(format!("  {message}")),
    }
}

fn render_submitted_block_item(block_item: &BlockItem<EncodedPayload>) -> Result<String> {
    let value = match block_item {
        BlockItem::AccountTransaction(transaction) => serde_json::json!({
            "kind": "accountTransaction",
            "header": transaction.header,
            "payload": render_encoded_payload_json(&transaction.payload),
        }),
        BlockItem::AccountTransactionV1(transaction) => serde_json::json!({
            "kind": "accountTransactionV1",
            "header": transaction.header,
            "payload": render_encoded_payload_json(&transaction.payload),
        }),
        BlockItem::CredentialDeployment(deployment) => serde_json::json!({
            "kind": "credentialDeployment",
            "payload": deployment,
        }),
        BlockItem::UpdateInstruction(instruction) => serde_json::json!({
            "kind": "updateInstruction",
            "payload": format!("{instruction:?}"),
        }),
    };
    serde_json::to_string_pretty(&value).context("failed to pretty-print submitted transaction")
}

fn render_encoded_payload_json(payload: &EncodedPayload) -> serde_json::Value {
    let raw_hex = hex::encode(payload.as_ref());
    match payload.decode() {
        Ok(decoded) => match serde_json::to_value(&decoded) {
            Ok(decoded) => serde_json::json!({
                "format": "decoded",
                "decoded": decoded,
                "rawHex": raw_hex,
            }),
            Err(err) => serde_json::json!({
                "format": "decodedDebug",
                "decoded": format!("{decoded:?}"),
                "rawHex": raw_hex,
                "note": format!("Decoded payload could not be serialized as JSON: {err}"),
            }),
        },
        Err(err) => serde_json::json!({
            "format": "rawHex",
            "rawHex": raw_hex,
            "decodeError": err.to_string(),
        }),
    }
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

    let event_lines = account_transaction_event_lines(summary)?;
    Ok(VariantRender {
        details: stable,
        payload_section: Some((
            format!("Events: {}", event_lines.len()),
            event_lines.join("\n"),
        )),
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

fn account_transaction_event_lines(summary: &BlockItemSummary) -> Result<Vec<String>> {
    let summary_json = serde_json::to_value(summary)
        .context("failed to serialize successful transaction summary to JSON")?;
    let events = summary_json
        .get("result")
        .and_then(|result| result.get("events"))
        .ok_or_else(|| anyhow!("successful transaction result missing events"))?;
    let events = events
        .as_array()
        .ok_or_else(|| anyhow!("successful transaction events payload was not an array"))?;
    events.iter().map(render_event_line).collect()
}

fn render_event_line(event: &serde_json::Value) -> Result<String> {
    let tag = event
        .get("tag")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Unknown");
    match tag {
        "TokenTransfer" => render_token_transfer_event_line(event),
        "LockCreated" => Ok(format!(
            "- Lock created: {}",
            render_json_lock_id(event.get("lockId"))?
        )),
        "LockDestroyed" => Ok(format!(
            "- Lock destroyed: {}",
            render_json_lock_id(event.get("lockId"))?
        )),
        "TokenMint" => Ok(format!(
            "- Mint {}: {} ({})",
            event_string_field(event, "tokenId")?,
            render_holder(event.get("target")),
            render_json_token_amount(event.get("amount"))?
        )),
        "TokenBurn" => Ok(format!(
            "- Burn {}: {} ({})",
            event_string_field(event, "tokenId")?,
            render_holder(event.get("target")),
            render_json_token_amount(event.get("amount"))?
        )),
        "TokenModuleEvent" => Ok(format!(
            "- Token module event {} on {}: {}",
            event_string_field(event, "type")?,
            event_string_field(event, "tokenId")?,
            compact_json(event.get("details"))?
        )),
        _ => Ok(format!("- {}", compact_json(event)?)),
    }
}

fn render_token_transfer_event_line(event: &serde_json::Value) -> Result<String> {
    let token_id = event_string_field(event, "tokenId")?;
    let from = render_holder_with_optional_lock(event.get("from"), event.get("fromLock"))?;
    let to = render_holder_with_optional_lock(event.get("to"), event.get("toLock"))?;
    let amount = render_json_token_amount(event.get("amount"))?;
    Ok(format!("- Transfer {amount} {token_id}: {from} -> {to}"))
}

fn render_holder(holder: Option<&serde_json::Value>) -> String {
    holder
        .and_then(|holder| holder.get("address"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
        .to_owned()
}

fn render_holder_with_optional_lock(
    holder: Option<&serde_json::Value>,
    lock: Option<&serde_json::Value>,
) -> Result<String> {
    let mut rendered = render_holder(holder);
    if let Some(lock) = lock {
        rendered.push_str(" (locked @ ");
        rendered.push_str(&render_json_lock_id(Some(lock))?);
        rendered.push(')');
    }
    Ok(rendered)
}

fn render_json_lock_id(value: Option<&serde_json::Value>) -> Result<String> {
    let Some(value) = value else {
        return Ok("unknown".to_owned());
    };
    if let Some(text) = value.as_str() {
        return Ok(text.to_owned());
    }
    // Accept both camelCase (accountIndex) and snake_case (account_index) — the
    // exact field names depend on the SDK version and which serde rename rule
    // applies to LockId.
    let account_index = value
        .get("accountIndex")
        .or_else(|| value.get("account_index"))
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow!("lock id missing accountIndex / account_index"))?;
    let sequence_number = value
        .get("sequenceNumber")
        .or_else(|| value.get("sequence_number"))
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow!("lock id missing sequenceNumber / sequence_number"))?;
    let creation_order = value
        .get("creationOrder")
        .or_else(|| value.get("creation_order"))
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow!("lock id missing creationOrder / creation_order"))?;
    Ok(
        concordium_rust_sdk::base::protocol_level_locks::LockId::new(
            account_index,
            sequence_number,
            creation_order,
        )
        .to_string(),
    )
}

fn render_json_token_amount(value: Option<&serde_json::Value>) -> Result<String> {
    let Some(value) = value else {
        return Ok("unknown".to_owned());
    };
    let decimals = value
        .get("decimals")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow!("token amount missing decimals"))? as u8;
    let raw = value
        .get("value")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("token amount missing value"))?
        .parse::<u64>()
        .context("token amount value was not an unsigned integer")?;
    Ok(
        concordium_rust_sdk::base::protocol_level_tokens::TokenAmount::from_raw(raw, decimals)
            .to_string(),
    )
}

fn event_string_field(event: &serde_json::Value, field: &str) -> Result<String> {
    event
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("event missing string field '{field}'"))
}

fn compact_json(value: impl serde::Serialize) -> Result<String> {
    serde_json::to_string(&value).context("failed to render event JSON")
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
        common::types::{Amount, TransactionSignature, TransactionTime},
        id::types::AccountAddress,
        protocol_level_tokens::{
            LockCreateEvent, LockDestroyEvent, MetaEvent, TokenEvent, TokenEventDetails,
            TokenHolder, TokenId, TokenModuleRef, TokenTransferEvent,
        },
        types::{
            AccountCreationDetails, AccountTransactionDetails, AccountTransactionEffects,
            BlockItemSummaryDetails, CredentialRegistrationID, Nonce, RejectReason,
            TokenCreationDetails, TransactionIndex, TransactionType, UpdateDetails, UpdatePayload,
            transactions::{
                AccountTransaction, Payload, PayloadLike, PayloadSize, TransactionHeader,
            },
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

    fn submitted_account_transfer_payload() -> SubmittedPayload {
        let payload = Payload::Transfer {
            to_address: account_address(),
            amount: Amount::from_micro_ccd(42),
        };
        let encoded = payload.encode();
        SubmittedPayload::Known(Box::new(BlockItem::AccountTransaction(
            AccountTransaction {
                signature: TransactionSignature {
                    signatures: BTreeMap::new(),
                },
                header: TransactionHeader {
                    sender: account_address(),
                    nonce: Nonce { nonce: 7 },
                    energy_amount: 501u64.into(),
                    payload_size: PayloadSize::from(encoded.as_ref().len() as u32),
                    expiry: TransactionTime {
                        seconds: 1_716_000_100,
                    },
                },
                payload: encoded,
            },
        )))
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
    fn received_output_with_requested_payload_explains_payload_unavailable() {
        let hash =
            transaction_hash("0fda6e284f9cd4429c6f76fd1bf6179aad4fa1bb218fe5ec8ad33916bf84a833");
        let rendered = render_transaction_status_with_payloads(
            &hash,
            "testnet @ https://grpc.testnet.concordium.com:20000",
            Some(&TransactionStatus::Received),
            &BTreeMap::new(),
            Some(&SubmittedPayloadDisplay::Unavailable(
                "Original submitted transaction is not available until the transaction is included in a block."
                    .to_owned(),
            )),
        )
        .unwrap();

        assert!(rendered.contains("Status: received"));
        assert!(rendered.contains("Submitted transaction:"));
        assert!(rendered.contains("not available until the transaction is included in a block"));
    }

    #[test]
    fn absent_output_with_requested_payload_does_not_show_payload_section() {
        let hash =
            transaction_hash("0fda6e284f9cd4429c6f76fd1bf6179aad4fa1bb218fe5ec8ad33916bf84a833");
        let rendered = render_transaction_status_with_payloads(
            &hash,
            "testnet @ https://grpc.testnet.concordium.com:20000",
            None,
            &BTreeMap::new(),
            Some(&SubmittedPayloadDisplay::Absent),
        )
        .unwrap();

        assert!(rendered.contains("Status: absent"));
        assert!(!rendered.contains("Submitted transaction:"));
    }

    #[test]
    fn finalized_account_transaction_payload_shows_header_and_payload_json() {
        let hash =
            transaction_hash("0fda6e284f9cd4429c6f76fd1bf6179aad4fa1bb218fe5ec8ad33916bf84a833");
        let block = block_hash("e2a12d06273f5641ea8157e04367eae49a72706aa831aa58b60ee5c062cdd6e2");
        let status = TransactionStatus::Finalized(BTreeMap::from([(
            block,
            success_summary(&hash.to_string()),
        )]));
        let submitted_payloads = SubmittedPayloadDisplay::ByBlock(BTreeMap::from([(
            block,
            submitted_account_transfer_payload(),
        )]));

        let rendered = render_transaction_status_with_payloads(
            &hash,
            "testnet @ https://grpc.testnet.concordium.com:20000",
            Some(&status),
            &BTreeMap::from([(block, "2026-05-19T14:23:11Z".to_owned())]),
            Some(&submitted_payloads),
        )
        .unwrap();

        assert!(rendered.contains("Submitted transaction:"));
        assert!(rendered.contains("\"kind\": \"accountTransaction\""));
        assert!(rendered.contains("\"header\""));
        assert!(rendered.contains("\"payload\""));
        assert!(rendered.contains("\"format\": \"decoded\""));
        assert!(rendered.contains("\"rawHex\""));
    }

    #[test]
    fn committed_account_transaction_payload_is_rendered_in_matching_block_section() {
        let hash =
            transaction_hash("0fda6e284f9cd4429c6f76fd1bf6179aad4fa1bb218fe5ec8ad33916bf84a833");
        let block = block_hash("e2a12d06273f5641ea8157e04367eae49a72706aa831aa58b60ee5c062cdd6e2");
        let status = TransactionStatus::Committed(BTreeMap::from([(
            block,
            success_summary(&hash.to_string()),
        )]));
        let submitted_payloads = SubmittedPayloadDisplay::ByBlock(BTreeMap::from([(
            block,
            SubmittedPayload::Unavailable("not found".to_owned()),
        )]));

        let rendered = render_transaction_status_with_payloads(
            &hash,
            "testnet @ https://grpc.testnet.concordium.com:20000",
            Some(&status),
            &BTreeMap::from([(block, "2026-05-19T14:23:11Z".to_owned())]),
            Some(&submitted_payloads),
        )
        .unwrap();

        assert!(rendered.contains("Block 1:"));
        assert!(rendered.contains("Submitted transaction:"));
        assert!(rendered.contains("not found"));
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
        assert!(rendered.contains("\"tag\":\"Transferred\""));
    }

    #[test]
    fn finalized_meta_update_renders_lock_events_as_single_lines() {
        let hash =
            transaction_hash("0fda6e284f9cd4429c6f76fd1bf6179aad4fa1bb218fe5ec8ad33916bf84a833");
        let block = block_hash("e2a12d06273f5641ea8157e04367eae49a72706aa831aa58b60ee5c062cdd6e2");
        let lock_id =
            concordium_rust_sdk::base::protocol_level_locks::LockId::new(1u64, 2u64, 3u64);
        let status = TransactionStatus::Finalized(BTreeMap::from([(
            block,
            BlockItemSummary {
                index: TransactionIndex { index: 5 },
                energy_cost: 116u64.into(),
                hash: transaction_hash(&hash.to_string()),
                details: Upward::Known(BlockItemSummaryDetails::AccountTransaction(
                    AccountTransactionDetails {
                        cost: Amount::from_micro_ccd(11_600),
                        sender: account_address(),
                        sponsor: None,
                        effects: Upward::Known(AccountTransactionEffects::MetaUpdate {
                            events: vec![
                                MetaEvent::LockCreate(LockCreateEvent {
                                    lock_id: lock_id.clone(),
                                    lock_config: Vec::<u8>::new().into(),
                                }),
                                MetaEvent::LockDestroy(LockDestroyEvent {
                                    lock_id: lock_id.clone(),
                                }),
                            ],
                        }),
                    },
                )),
            },
        )]));

        let rendered = render_transaction_status(
            &hash,
            "testnet @ https://grpc.testnet.concordium.com:20000",
            Some(&status),
            &BTreeMap::from([(block, "2026-05-19T14:23:11Z".to_owned())]),
        )
        .unwrap();

        assert!(rendered.contains(&format!("- Lock created: {lock_id}")));
        assert!(rendered.contains(&format!("- Lock destroyed: {lock_id}")));
    }

    #[test]
    fn finalized_meta_update_renders_token_transfer_as_single_line() {
        let hash =
            transaction_hash("0fda6e284f9cd4429c6f76fd1bf6179aad4fa1bb218fe5ec8ad33916bf84a833");
        let block = block_hash("e2a12d06273f5641ea8157e04367eae49a72706aa831aa58b60ee5c062cdd6e2");
        let status = TransactionStatus::Finalized(BTreeMap::from([(
            block,
            BlockItemSummary {
                index: TransactionIndex { index: 6 },
                energy_cost: 117u64.into(),
                hash: transaction_hash(&hash.to_string()),
                details: Upward::Known(BlockItemSummaryDetails::AccountTransaction(
                    AccountTransactionDetails {
                        cost: Amount::from_micro_ccd(11_700),
                        sender: account_address(),
                        sponsor: None,
                        effects: Upward::Known(AccountTransactionEffects::MetaUpdate {
                            events: vec![MetaEvent::Token(TokenEvent {
                                token_id: TokenId::from_str("TEST").unwrap(),
                                event: TokenEventDetails::Transfer(TokenTransferEvent {
                                    from: TokenHolder::Account {
                                        address: account_address(),
                                    },
                                    to: TokenHolder::Account {
                                        address: account_address(),
                                    },
                                    amount: concordium_rust_sdk::base::protocol_level_tokens::TokenAmount::from_raw(1000, 2),
                                    memo: None,
                                    from_lock: None,
                                    to_lock: None,
                                }),
                            })],
                        }),
                    },
                )),
            },
        )]));

        let rendered = render_transaction_status(
            &hash,
            "testnet @ https://grpc.testnet.concordium.com:20000",
            Some(&status),
            &BTreeMap::from([(block, "2026-05-19T14:23:11Z".to_owned())]),
        )
        .unwrap();

        assert!(rendered.contains("- Transfer 10.00 TEST:"));
        assert!(!rendered.contains("("));
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
