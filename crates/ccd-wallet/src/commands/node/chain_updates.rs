//! Query and render pending chain updates and next update sequence numbers.

use anyhow::{Context, Result};
use concordium_rust_sdk::{
    types::queries::{NextUpdateSequenceNumbers, PendingUpdate},
    v2::{BlockIdentifier, Upward},
};
use serde::Serialize;

/// Stable output for chain-update inspection.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ChainUpdatesOutput {
    /// Node endpoint used for both queries.
    pub(super) endpoint: String,
    /// Concrete block hash shared by both query results.
    pub(super) block_hash: String,
    /// Next sequence number for every governance update queue.
    pub(super) next_sequence_numbers: NextUpdateSequenceNumbersView,
    /// Pending updates in node stream order.
    pub(super) pending_updates: Vec<PendingUpdateView>,
}

/// Stable output for all governance update sequence queues.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NextUpdateSequenceNumbersView {
    root_keys: u64,
    level1_keys: u64,
    level2_keys: u64,
    protocol: u64,
    election_difficulty: u64,
    euro_per_energy: u64,
    micro_ccd_per_euro: u64,
    foundation_account: u64,
    mint_distribution: u64,
    transaction_fee_distribution: u64,
    gas_rewards: u64,
    pool_parameters: u64,
    add_anonymity_revoker: u64,
    add_identity_provider: u64,
    cooldown_parameters: u64,
    time_parameters: u64,
    timeout_parameters: u64,
    min_block_time: u64,
    block_energy_limit: u64,
    finalization_committee_parameters: u64,
    validator_score_parameters: u64,
    protocol_level_tokens: u64,
    max_lock_duration: u64,
}

/// Stable output for one scheduled governance update.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PendingUpdateView {
    /// Unix timestamp when the update becomes effective.
    effective_time: u64,
    /// Typed update effect, or an explicit unknown marker.
    effect: serde_json::Value,
}

impl ChainUpdatesOutput {
    /// Construct stable chain-update output from normalized SDK values.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Display label for the queried node endpoint.
    /// * `block_hash` - Concrete block hash shared by both queries.
    /// * `sequence_numbers` - Next sequence numbers at the concrete block.
    /// * `pending_updates` - Complete pending-update stream at the same block.
    ///
    /// # Errors
    ///
    /// Returns an error if a known pending update effect cannot be serialized.
    pub(super) fn try_new(
        endpoint: String,
        block_hash: String,
        sequence_numbers: &NextUpdateSequenceNumbers,
        pending_updates: &[PendingUpdate],
    ) -> Result<Self> {
        Ok(Self {
            endpoint,
            block_hash,
            next_sequence_numbers: sequence_numbers.into(),
            pending_updates: pending_updates
                .iter()
                .map(PendingUpdateView::try_from)
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

impl From<&NextUpdateSequenceNumbers> for NextUpdateSequenceNumbersView {
    fn from(value: &NextUpdateSequenceNumbers) -> Self {
        Self {
            root_keys: value.root_keys.number,
            level1_keys: value.level_1_keys.number,
            level2_keys: value.level_2_keys.number,
            protocol: value.protocol.number,
            election_difficulty: value.election_difficulty.number,
            euro_per_energy: value.euro_per_energy.number,
            micro_ccd_per_euro: value.micro_ccd_per_euro.number,
            foundation_account: value.foundation_account.number,
            mint_distribution: value.mint_distribution.number,
            transaction_fee_distribution: value.transaction_fee_distribution.number,
            gas_rewards: value.gas_rewards.number,
            pool_parameters: value.pool_parameters.number,
            add_anonymity_revoker: value.add_anonymity_revoker.number,
            add_identity_provider: value.add_identity_provider.number,
            cooldown_parameters: value.cooldown_parameters.number,
            time_parameters: value.time_parameters.number,
            timeout_parameters: value.timeout_parameters.number,
            min_block_time: value.min_block_time.number,
            block_energy_limit: value.block_energy_limit.number,
            finalization_committee_parameters: value.finalization_committee_parameters.number,
            validator_score_parameters: value.validator_score_parameters.number,
            protocol_level_tokens: value.protocol_level_tokens.number,
            max_lock_duration: value.max_lock_duration.number,
        }
    }
}

impl TryFrom<&PendingUpdate> for PendingUpdateView {
    type Error = anyhow::Error;

    fn try_from(value: &PendingUpdate) -> Result<Self> {
        let effect = match &value.effect {
            Upward::Known(effect) => serde_json::to_value(effect)
                .context("failed to serialize pending chain update effect")?,
            Upward::Unknown(()) => serde_json::json!({ "updateType": "unknown" }),
        };
        Ok(Self {
            effective_time: value.effective_time.seconds,
            effect,
        })
    }
}

/// Return the concrete identifier used for the pending-update snapshot query.
pub(super) fn snapshot_block_identifier(
    block_hash: concordium_rust_sdk::base::hashes::BlockHash,
) -> BlockIdentifier {
    BlockIdentifier::Given(block_hash)
}

/// Render chain-update state in a sectioned human-readable form.
pub(super) fn render_human(output: &ChainUpdatesOutput) -> Result<String> {
    let mut lines = vec![
        format!("Node endpoint: {}", output.endpoint),
        format!("Block hash: {}", output.block_hash),
        String::new(),
        "Next update sequence numbers:".to_owned(),
    ];
    let sequences = serde_json::to_value(output.next_sequence_numbers)
        .context("failed to render next update sequence numbers")?;
    let serde_json::Value::Object(sequences) = sequences else {
        unreachable!("sequence-number view serializes as an object");
    };
    for (queue, sequence) in sequences {
        lines.push(format!("  {}: {sequence}", humanize(&queue)));
    }

    lines.push(String::new());
    lines.push("Pending updates:".to_owned());
    if output.pending_updates.is_empty() {
        lines.push("  none".to_owned());
    } else {
        for (index, update) in output.pending_updates.iter().enumerate() {
            lines.push(format!(
                "  {}. effective time: {}",
                index + 1,
                update.effective_time
            ));
            let rendered = serde_json::to_string_pretty(&update.effect)
                .context("failed to render pending update effect")?;
            lines.push("     effect:".to_owned());
            lines.extend(rendered.lines().map(|line| format!("       {line}")));
        }
    }
    Ok(lines.join("\n"))
}

fn humanize(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for (index, ch) in value.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            output.push(' ');
            output.push(ch.to_ascii_lowercase());
        } else if index == 0 {
            output.push(ch.to_ascii_uppercase());
        } else {
            output.push(ch);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use concordium_rust_sdk::{
        base::{base::UpdateSequenceNumber, hashes::BlockHash},
        common::types::TransactionTime,
        smart_contracts::common::Duration,
        types::queries::PendingUpdateEffect,
    };

    fn sequences(number: u64) -> NextUpdateSequenceNumbers {
        let sequence = UpdateSequenceNumber { number };
        NextUpdateSequenceNumbers {
            root_keys: sequence,
            level_1_keys: sequence,
            level_2_keys: sequence,
            protocol: sequence,
            election_difficulty: sequence,
            euro_per_energy: sequence,
            micro_ccd_per_euro: sequence,
            foundation_account: sequence,
            mint_distribution: sequence,
            transaction_fee_distribution: sequence,
            gas_rewards: sequence,
            pool_parameters: sequence,
            add_anonymity_revoker: sequence,
            add_identity_provider: sequence,
            cooldown_parameters: sequence,
            time_parameters: sequence,
            timeout_parameters: sequence,
            min_block_time: sequence,
            block_energy_limit: sequence,
            finalization_committee_parameters: sequence,
            validator_score_parameters: sequence,
            protocol_level_tokens: sequence,
            max_lock_duration: sequence,
        }
    }

    #[test]
    fn output_includes_all_sequences_and_p11_pending_update() {
        let pending = PendingUpdate {
            effective_time: TransactionTime::from_seconds(1_800_000_000),
            effect: Upward::Known(PendingUpdateEffect::MaxLockDuration(Duration::from_days(
                30,
            ))),
        };
        let output = ChainUpdatesOutput::try_new(
            "https://node.test".to_owned(),
            "11".repeat(32),
            &sequences(7),
            &[pending],
        )
        .unwrap();
        let json = serde_json::to_value(&output).unwrap();

        assert_eq!(json["nextSequenceNumbers"]["maxLockDuration"], 7);
        assert_eq!(
            json["pendingUpdates"][0]["effect"]["updateType"],
            "maxLockDuration"
        );
        assert_eq!(
            json["pendingUpdates"][0]["effect"]["update"],
            "30d 0h 0m 0s 0ms"
        );
        let human = render_human(&output).unwrap();
        assert!(human.contains("Max lock duration: 7"));
        assert!(human.contains("maxLockDuration"));
        assert!(human.contains("1800000000"));
    }

    #[test]
    fn empty_and_unknown_pending_updates_are_explicit() {
        let empty = ChainUpdatesOutput::try_new(
            "https://node.test".to_owned(),
            "00".repeat(32),
            &sequences(0),
            &[],
        )
        .unwrap();
        assert!(
            render_human(&empty)
                .unwrap()
                .contains("Pending updates:\n  none")
        );
        assert_eq!(
            serde_json::to_value(&empty).unwrap()["pendingUpdates"],
            serde_json::json!([])
        );

        let unknown = PendingUpdate {
            effective_time: TransactionTime::from_seconds(42),
            effect: Upward::Unknown(()),
        };
        let output = ChainUpdatesOutput::try_new(
            "https://node.test".to_owned(),
            "00".repeat(32),
            &sequences(0),
            &[unknown],
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(output).unwrap()["pendingUpdates"][0]["effect"]["updateType"],
            "unknown"
        );
    }

    #[test]
    fn snapshot_identifier_uses_resolved_block_hash() {
        let hash: BlockHash = [7u8; 32].into();
        assert_eq!(
            snapshot_block_identifier(hash),
            BlockIdentifier::Given(hash)
        );
    }
}
