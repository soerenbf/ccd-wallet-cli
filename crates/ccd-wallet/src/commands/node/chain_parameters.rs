//! Query and render normalized Concordium chain parameters.

use anyhow::{Context, Result};
use concordium_rust_sdk::{
    base::updates::{AccessStructure, HigherLevelAccessStructure},
    types::chain_parameters::{
        ChainParameters, FinalizationCommitteeParameters, GasRewards, Level2Keys, MintDistribution,
        StakingParameters, TimeoutParameters, TransactionFeeDistribution, UpdateKeys,
    },
};
use serde::Serialize;
use serde_json::Value;

/// Stable output for a chain-parameter query.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ChainParametersOutput {
    /// Node endpoint used for the query.
    pub(super) endpoint: String,
    /// Block hash resolved by the node.
    pub(super) block_hash: String,
    /// Normalized chain parameters at the resolved block.
    pub(super) parameters: ChainParametersView,
}

/// Stable JSON view of all normalized SDK chain parameters.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ChainParametersView {
    timeout_parameters: TimeoutParametersView,
    election_difficulty: Option<Value>,
    min_block_time: Option<Value>,
    block_energy_limit: Option<Value>,
    euro_per_energy: Option<Value>,
    micro_ccd_per_euro: Option<Value>,
    cooldown_parameters: CooldownParametersView,
    reward_period_length: Option<Value>,
    mint_per_payday: Option<Value>,
    mint_per_slot: Option<Value>,
    account_creation_limit: Option<Value>,
    mint_distribution: MintDistributionView,
    transaction_fee_distribution: TransactionFeeDistributionView,
    gas_rewards: GasRewardsView,
    foundation_account: Option<String>,
    staking_parameters: StakingParametersView,
    finalization_committee_parameters: FinalizationCommitteeParametersView,
    validator_max_missed_rounds: Option<u64>,
    max_lock_duration: Option<Value>,
    keys: UpdateKeysView,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimeoutParametersView {
    base: Option<Value>,
    increase: Option<Value>,
    decrease: Option<Value>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CooldownParametersView {
    baker_cooldown_epochs: Option<Value>,
    pool_owner_cooldown: Option<Value>,
    delegator_cooldown: Option<Value>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MintDistributionView {
    baking_reward: Option<Value>,
    finalization_reward: Option<Value>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TransactionFeeDistributionView {
    baker: Option<Value>,
    gas_account: Option<Value>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GasRewardsView {
    baker: Option<Value>,
    finalization_proof: Option<Value>,
    account_creation: Option<Value>,
    chain_update: Option<Value>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StakingParametersView {
    passive_finalization_commission: Option<Value>,
    passive_baking_commission: Option<Value>,
    passive_transaction_commission: Option<Value>,
    finalization_commission_range: Option<Value>,
    baking_commission_range: Option<Value>,
    transaction_commission_range: Option<Value>,
    minimum_equity_capital: Option<Value>,
    capital_bound: Option<Value>,
    leverage_bound: Option<Value>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FinalizationCommitteeParametersView {
    min_finalizers: Option<u32>,
    max_finalizers: Option<u32>,
    finalizers_relative_stake_threshold: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateKeysView {
    root_keys: Option<HigherLevelKeysView>,
    level1_keys: Option<HigherLevelKeysView>,
    level2_keys: Option<Level2KeysView>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HigherLevelKeysView {
    keys: Vec<Value>,
    threshold: u16,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccessStructureView {
    authorized_keys: Vec<u16>,
    threshold: u16,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Level2KeysView {
    keys: Vec<Value>,
    emergency: Option<AccessStructureView>,
    protocol: Option<AccessStructureView>,
    consensus: Option<AccessStructureView>,
    euro_per_energy: Option<AccessStructureView>,
    micro_ccd_per_euro: Option<AccessStructureView>,
    foundation_account: Option<AccessStructureView>,
    mint_distribution: Option<AccessStructureView>,
    transaction_fee_distribution: Option<AccessStructureView>,
    param_gas_rewards: Option<AccessStructureView>,
    pool_parameters: Option<AccessStructureView>,
    add_anonymity_revoker: Option<AccessStructureView>,
    add_identity_provider: Option<AccessStructureView>,
    cooldown_parameters: Option<AccessStructureView>,
    time_parameters: Option<AccessStructureView>,
    create_plt: Option<AccessStructureView>,
    token_parameters: Option<AccessStructureView>,
}

impl ChainParametersOutput {
    /// Convert a normalized SDK response into stable command output.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Display label for the queried node endpoint.
    /// * `block_hash` - Block hash resolved by the node.
    /// * `parameters` - Normalized SDK chain parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if a Concordium domain value cannot be represented as JSON.
    pub(super) fn try_new(
        endpoint: String,
        block_hash: String,
        parameters: &ChainParameters,
    ) -> Result<Self> {
        Ok(Self {
            endpoint,
            block_hash,
            parameters: ChainParametersView::try_from(parameters)?,
        })
    }
}

impl TryFrom<&ChainParameters> for ChainParametersView {
    type Error = anyhow::Error;

    fn try_from(value: &ChainParameters) -> Result<Self> {
        Ok(Self {
            timeout_parameters: TimeoutParametersView::try_from(&value.timeout_parameters)?,
            election_difficulty: optional_value(value.election_difficulty.as_ref())?,
            min_block_time: optional_value(value.min_block_time.as_ref())?,
            block_energy_limit: optional_value(value.block_energy_limit.as_ref())?,
            euro_per_energy: optional_value(value.euro_per_energy.as_ref())?,
            micro_ccd_per_euro: optional_value(value.micro_ccd_per_euro.as_ref())?,
            cooldown_parameters: CooldownParametersView {
                baker_cooldown_epochs: optional_value(
                    value.cooldown_parameters.baker_cooldown_epochs.as_ref(),
                )?,
                pool_owner_cooldown: optional_value(
                    value.cooldown_parameters.pool_owner_cooldown.as_ref(),
                )?,
                delegator_cooldown: optional_value(
                    value.cooldown_parameters.delegator_cooldown.as_ref(),
                )?,
            },
            reward_period_length: optional_value(value.reward_period_length.as_ref())?,
            mint_per_payday: optional_value(value.mint_per_payday.as_ref())?,
            mint_per_slot: optional_value(value.mint_per_slot.as_ref())?,
            account_creation_limit: optional_value(value.account_creation_limit.as_ref())?,
            mint_distribution: MintDistributionView::try_from(&value.mint_distribution)?,
            transaction_fee_distribution: TransactionFeeDistributionView::try_from(
                &value.transaction_fee_distribution,
            )?,
            gas_rewards: GasRewardsView::try_from(&value.gas_rewards)?,
            foundation_account: value.foundation_account.map(|address| address.to_string()),
            staking_parameters: StakingParametersView::try_from(&value.staking_parameters)?,
            finalization_committee_parameters: FinalizationCommitteeParametersView::from(
                &value.finalization_committee_parameters,
            ),
            validator_max_missed_rounds: value.validator_max_missed_rounds,
            max_lock_duration: optional_value(value.max_lock_duration.as_ref())?,
            keys: UpdateKeysView::try_from(&value.keys)?,
        })
    }
}

impl TryFrom<&TimeoutParameters> for TimeoutParametersView {
    type Error = anyhow::Error;

    fn try_from(value: &TimeoutParameters) -> Result<Self> {
        Ok(Self {
            base: optional_value(value.base.as_ref())?,
            increase: optional_value(value.increase.as_ref())?,
            decrease: optional_value(value.decrease.as_ref())?,
        })
    }
}

impl TryFrom<&MintDistribution> for MintDistributionView {
    type Error = anyhow::Error;

    fn try_from(value: &MintDistribution) -> Result<Self> {
        Ok(Self {
            baking_reward: optional_value(value.baking_reward.as_ref())?,
            finalization_reward: optional_value(value.finalization_reward.as_ref())?,
        })
    }
}

impl TryFrom<&TransactionFeeDistribution> for TransactionFeeDistributionView {
    type Error = anyhow::Error;

    fn try_from(value: &TransactionFeeDistribution) -> Result<Self> {
        Ok(Self {
            baker: optional_value(value.baker.as_ref())?,
            gas_account: optional_value(value.gas_account.as_ref())?,
        })
    }
}

impl TryFrom<&GasRewards> for GasRewardsView {
    type Error = anyhow::Error;

    fn try_from(value: &GasRewards) -> Result<Self> {
        Ok(Self {
            baker: optional_value(value.baker.as_ref())?,
            finalization_proof: optional_value(value.finalization_proof.as_ref())?,
            account_creation: optional_value(value.account_creation.as_ref())?,
            chain_update: optional_value(value.chain_update.as_ref())?,
        })
    }
}

impl TryFrom<&StakingParameters> for StakingParametersView {
    type Error = anyhow::Error;

    fn try_from(value: &StakingParameters) -> Result<Self> {
        Ok(Self {
            passive_finalization_commission: optional_value(
                value.passive_finalization_commission.as_ref(),
            )?,
            passive_baking_commission: optional_value(value.passive_baking_commission.as_ref())?,
            passive_transaction_commission: optional_value(
                value.passive_transaction_commission.as_ref(),
            )?,
            finalization_commission_range: optional_value(
                value.finalization_commission_range.as_ref(),
            )?,
            baking_commission_range: optional_value(value.baking_commission_range.as_ref())?,
            transaction_commission_range: optional_value(
                value.transaction_commission_range.as_ref(),
            )?,
            minimum_equity_capital: optional_value(value.minimum_equity_capital.as_ref())?,
            capital_bound: optional_value(value.capital_bound.as_ref())?,
            leverage_bound: optional_value(value.leverage_bound.as_ref())?,
        })
    }
}

impl From<&FinalizationCommitteeParameters> for FinalizationCommitteeParametersView {
    fn from(value: &FinalizationCommitteeParameters) -> Self {
        Self {
            min_finalizers: value.min_finalizers,
            max_finalizers: value.max_finalizers,
            finalizers_relative_stake_threshold: value
                .finalizers_relative_stake_threshold
                .map(|threshold| threshold.to_string()),
        }
    }
}

impl TryFrom<&UpdateKeys> for UpdateKeysView {
    type Error = anyhow::Error;

    fn try_from(value: &UpdateKeys) -> Result<Self> {
        Ok(Self {
            root_keys: value
                .root_keys
                .as_ref()
                .map(higher_level_keys)
                .transpose()?,
            level1_keys: value
                .level_1_keys
                .as_ref()
                .map(higher_level_keys)
                .transpose()?,
            level2_keys: value
                .level_2_keys
                .as_ref()
                .map(Level2KeysView::try_from)
                .transpose()?,
        })
    }
}

fn higher_level_keys<Kind>(
    value: &HigherLevelAccessStructure<Kind>,
) -> Result<HigherLevelKeysView> {
    Ok(HigherLevelKeysView {
        keys: value
            .keys
            .iter()
            .map(json_value)
            .collect::<Result<Vec<_>>>()?,
        threshold: u16::from(value.threshold),
    })
}

impl TryFrom<&Level2Keys> for Level2KeysView {
    type Error = anyhow::Error;

    fn try_from(value: &Level2Keys) -> Result<Self> {
        Ok(Self {
            keys: value
                .keys
                .iter()
                .map(json_value)
                .collect::<Result<Vec<_>>>()?,
            emergency: optional_access(value.emergency.as_ref()),
            protocol: optional_access(value.protocol.as_ref()),
            consensus: optional_access(value.consensus.as_ref()),
            euro_per_energy: optional_access(value.euro_per_energy.as_ref()),
            micro_ccd_per_euro: optional_access(value.micro_ccd_per_euro.as_ref()),
            foundation_account: optional_access(value.foundation_account.as_ref()),
            mint_distribution: optional_access(value.mint_distribution.as_ref()),
            transaction_fee_distribution: optional_access(
                value.transaction_fee_distribution.as_ref(),
            ),
            param_gas_rewards: optional_access(value.param_gas_rewards.as_ref()),
            pool_parameters: optional_access(value.pool_parameters.as_ref()),
            add_anonymity_revoker: optional_access(value.add_anonymity_revoker.as_ref()),
            add_identity_provider: optional_access(value.add_identity_provider.as_ref()),
            cooldown_parameters: optional_access(value.cooldown_parameters.as_ref()),
            time_parameters: optional_access(value.time_parameters.as_ref()),
            create_plt: optional_access(value.create_plt.as_ref()),
            token_parameters: optional_access(value.token_parameters.as_ref()),
        })
    }
}

fn optional_access(value: Option<&AccessStructure>) -> Option<AccessStructureView> {
    value.map(|access| AccessStructureView {
        authorized_keys: access
            .authorized_keys
            .iter()
            .map(|index| index.index)
            .collect(),
        threshold: u16::from(access.threshold),
    })
}

fn json_value<T: Serialize>(value: &T) -> Result<Value> {
    serde_json::to_value(value).context("failed to serialize Concordium chain parameter value")
}

fn optional_value<T: Serialize>(value: Option<&T>) -> Result<Option<Value>> {
    value.map(json_value).transpose()
}

/// Render chain parameters in a sectioned human-readable form.
pub(super) fn render_human(output: &ChainParametersOutput) -> Result<String> {
    let parameters = serde_json::to_value(&output.parameters)
        .context("failed to render normalized chain parameters")?;
    let Value::Object(parameters) = parameters else {
        return Ok(format!(
            "Node endpoint: {}\nBlock hash: {}",
            output.endpoint, output.block_hash
        ));
    };
    let mut lines = vec![
        format!("Node endpoint: {}", output.endpoint),
        format!("Block hash: {}", output.block_hash),
        String::new(),
        "Chain parameters:".to_owned(),
    ];
    render_human_value(&mut lines, &Value::Object(parameters), 2)?;
    Ok(lines.join("\n"))
}

fn render_human_value(lines: &mut Vec<String>, value: &Value, indent: usize) -> Result<()> {
    let padding = " ".repeat(indent);
    match value {
        Value::Object(fields) => {
            for (name, value) in fields {
                lines.push(format!("{padding}{}:", humanize(name)));
                render_human_value(lines, value, indent + 2)?;
            }
        }
        Value::Array(values) if values.is_empty() => lines.push(format!("{padding}[]")),
        Value::Array(values) => {
            for value in values {
                lines.push(format!("{padding}-"));
                render_human_value(lines, value, indent + 2)?;
            }
        }
        _ => lines.push(format!(
            "{padding}{}",
            serde_json::to_string(value).context("failed to render chain parameter value")?
        )),
    }
    Ok(())
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
        base::updates::AccessStructure, smart_contracts::common::Duration,
        types::chain_parameters::Level2Keys,
    };

    #[test]
    fn json_keeps_unavailable_fields_null() {
        let output = ChainParametersOutput::try_new(
            "https://node.test".to_owned(),
            "00".repeat(32),
            &ChainParameters::default(),
        )
        .unwrap();
        let json = serde_json::to_value(output).unwrap();

        assert!(json["parameters"]["maxLockDuration"].is_null());
        assert!(json["parameters"]["keys"]["level2Keys"].is_null());
    }

    #[test]
    fn p11_fields_include_duration_and_token_authorization() {
        let access = AccessStructure {
            authorized_keys: [1u16.into()].into_iter().collect(),
            threshold: 1u16.try_into().unwrap(),
        };
        let parameters = ChainParameters {
            max_lock_duration: Some(Duration::from_days(30)),
            keys: UpdateKeys {
                level_2_keys: Some(Level2Keys {
                    keys: Vec::new(),
                    emergency: None,
                    protocol: None,
                    consensus: None,
                    euro_per_energy: None,
                    micro_ccd_per_euro: None,
                    foundation_account: None,
                    mint_distribution: None,
                    transaction_fee_distribution: None,
                    param_gas_rewards: None,
                    pool_parameters: None,
                    add_anonymity_revoker: None,
                    add_identity_provider: None,
                    cooldown_parameters: None,
                    time_parameters: None,
                    create_plt: None,
                    token_parameters: Some(access),
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        let output = ChainParametersOutput::try_new(
            "https://node.test".to_owned(),
            "11".repeat(32),
            &parameters,
        )
        .unwrap();
        let json = serde_json::to_value(&output).unwrap();

        assert_eq!(json["parameters"]["maxLockDuration"], "30d 0h 0m 0s 0ms");
        assert_eq!(
            json["parameters"]["keys"]["level2Keys"]["tokenParameters"]["authorizedKeys"],
            serde_json::json!([1])
        );
        assert_eq!(
            json["parameters"]["keys"]["level2Keys"]["tokenParameters"]["threshold"],
            1
        );
        let human = render_human(&output).unwrap();
        assert!(human.contains("Max lock duration"));
        assert!(human.contains("30d 0h 0m 0s 0ms"));
        assert!(human.contains("Token parameters"));
    }
}
