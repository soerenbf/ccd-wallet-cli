//! Shared stake command helpers for context resolution, staking queries, and rendering.

use crate::{
    commands::{
        account::{
            AccountReferenceUnlocks, build_export_wallet_account_with_unlocks,
            decrypt_local_account_address, resolve_account_network_context, resolve_export_account,
        },
        transaction::render::render_finalized_summary,
        ui::{ContextLine, log_resolved_context},
    },
    smart_contracts::shared::parse_decimal_ccd_amount,
};
use anyhow::{Context, Result, bail};
use ccd_wallet_core::{config as node_config, store::accounts};
use chrono::SecondsFormat;
use cliclack::spinner;
use concordium_rust_sdk::{
    base::{
        base::{CommissionRates, DelegationTarget, OpenStatus},
        common::types::{AccountAddress, Amount, TransactionTime},
        transactions::{BlockItem, ConfigureBakerPayload, ConfigureDelegationPayload},
    },
    types::{
        AccountInfo, AccountStakingInfo, BakerId, StakePendingChange, WalletAccount,
        transactions::send,
    },
    v2::{self, AccountIdentifier, BlockIdentifier},
};
use futures_util::TryStreamExt;
use rusqlite::Connection;
use serde::Serialize;
use std::str::FromStr;

/// Resolved context for a read-only stake query.
pub(crate) struct StakeQueryContext {
    pub(crate) network_name: String,
    pub(crate) network_genesis_hash: String,
    pub(crate) client: v2::Client,
}

/// Resolved context for a stake mutation command.
pub(crate) struct StakeMutationContext {
    pub(crate) network_name: String,
    pub(crate) endpoint_label: String,
    pub(crate) client: v2::Client,
    pub(crate) wallet: WalletAccount,
    pub(crate) account_label: String,
}

/// Serializable staking details used by account and stake inspection views.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StakeDetailsView {
    pub(crate) mode: String,
    pub(crate) staked_amount: Option<String>,
    pub(crate) restake_earnings: Option<bool>,
    pub(crate) delegation_target: Option<String>,
    pub(crate) validator_id: Option<String>,
    pub(crate) pending_change: Option<StakePendingChangeView>,
    pub(crate) validator_pool: Option<ValidatorPoolView>,
    pub(crate) is_suspended: Option<bool>,
}

/// Serializable pending stake-change details.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StakePendingChangeView {
    pub(crate) kind: String,
    pub(crate) effective_time: String,
    pub(crate) new_stake: Option<String>,
}

/// Serializable validator-pool details.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ValidatorPoolView {
    pub(crate) open_status: String,
    pub(crate) metadata_url: String,
    pub(crate) commission_rates: CommissionRatesView,
}

/// Serializable commission-rate details.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommissionRatesView {
    pub(crate) finalization: String,
    pub(crate) baking: String,
    pub(crate) transaction: String,
}

/// Resolve a client for stake inspection.
pub(crate) async fn resolve_query_context(
    conn: &Connection,
    network: Option<&str>,
    node: Option<v2::Endpoint>,
    no_defaults: bool,
) -> Result<StakeQueryContext> {
    let (network_name, network_entry, endpoint, endpoint_label, network_source) =
        resolve_account_network_context(conn, network, node, false, no_defaults).await?;
    log_resolved_context(&[ContextLine {
        label: "network:",
        value: format!("{network_name} @ {endpoint_label}"),
        source: network_source,
    }])?;
    let client = node_config::connect_v2_client(endpoint.clone())
        .await
        .with_context(|| format!("failed to connect to Concordium node at {endpoint_label}"))?;
    Ok(StakeQueryContext {
        network_name,
        network_genesis_hash: network_entry.genesis_hash,
        client,
    })
}

/// Resolve the signer wallet and client for stake mutation.
pub(crate) async fn resolve_mutation_context(
    conn: &Connection,
    account: Option<&str>,
    network: Option<&str>,
    node: Option<v2::Endpoint>,
    non_interactive: bool,
    no_defaults: bool,
) -> Result<StakeMutationContext> {
    let (network_name, network_entry, endpoint, endpoint_label, network_source) =
        resolve_account_network_context(conn, network, node, non_interactive, no_defaults).await?;
    log_resolved_context(&[ContextLine {
        label: "network:",
        value: format!("{network_name} @ {endpoint_label}"),
        source: network_source,
    }])?;
    let account = resolve_export_account(
        conn,
        &network_name,
        &network_entry.genesis_hash,
        account,
        non_interactive,
        false,
    )?;
    let mut unlocks = AccountReferenceUnlocks::new();
    let wallet = build_export_wallet_account_with_unlocks(
        conn,
        &network_name,
        &network_entry,
        &account,
        &mut unlocks,
    )?;
    let client = node_config::connect_v2_client(endpoint.clone())
        .await
        .with_context(|| format!("failed to connect to Concordium node at {endpoint_label}"))?;
    Ok(StakeMutationContext {
        network_name,
        endpoint_label,
        client,
        wallet,
        account_label: account.label,
    })
}

/// Resolve either a raw account address or a finalized local account label.
pub(crate) fn resolve_stake_query_address(
    conn: &Connection,
    network_name: &str,
    network_genesis_hash: &str,
    value: &str,
) -> Result<(AccountAddress, Option<String>)> {
    if let Ok(address) = AccountAddress::from_str(value) {
        return Ok((address, None));
    }
    let record = accounts::find_by_network_and_label(conn, network_genesis_hash, value)?
        .with_context(|| {
            format!(
                "account '{value}' is not a valid address or configured local account label for the selected network"
            )
        })?;
    if record.status != accounts::AccountStatus::Finalized {
        bail!("account '{value}' is not finalized yet and has no live staking state to inspect");
    }
    Ok((
        decrypt_local_account_address(conn, network_name, &record)?,
        Some(record.label),
    ))
}

/// Query live account information.
pub(crate) async fn query_account_info(
    client: &mut v2::Client,
    address: AccountAddress,
    block: BlockIdentifier,
) -> Result<AccountInfo> {
    Ok(client
        .get_account_info(&AccountIdentifier::from(address), block)
        .await
        .context("failed to query account information")?
        .response)
}

/// Query the current validator ids on the selected network.
pub(crate) async fn query_validator_ids(client: &mut v2::Client) -> Result<Vec<BakerId>> {
    client
        .get_baker_list(BlockIdentifier::LastFinal)
        .await
        .context("failed to query validator list")?
        .response
        .try_collect::<Vec<_>>()
        .await
        .context("failed to stream validator list")
}

/// Validate that a validator exists on the selected network.
pub(crate) async fn validate_validator_id(
    client: &mut v2::Client,
    validator_id: BakerId,
) -> Result<()> {
    let bakers = query_validator_ids(client).await?;
    if validator_id_exists(&bakers, validator_id) {
        return Ok(());
    }
    bail!(
        "validator id {} is not valid on the selected network",
        validator_id
    )
}

/// Build a stake-details view from live account information.
pub(crate) fn stake_details_view_from_account_info(info: &AccountInfo) -> Option<StakeDetailsView> {
    stake_details_view(
        info.account_stake
            .as_ref()
            .and_then(|staking| staking.as_ref().known()),
    )
}

/// Build a stake-details view from optional staking information.
pub(crate) fn stake_details_view(staking: Option<&AccountStakingInfo>) -> Option<StakeDetailsView> {
    let staking = staking?;
    Some(match staking {
        AccountStakingInfo::Delegated {
            staked_amount,
            restake_earnings,
            delegation_target,
            pending_change,
        } => StakeDetailsView {
            mode: "delegated".to_owned(),
            staked_amount: Some(format!("{} CCD", staked_amount)),
            restake_earnings: Some(*restake_earnings),
            delegation_target: Some(format_delegation_target(delegation_target)),
            validator_id: delegation_target_validator_id(delegation_target),
            pending_change: pending_change.map(pending_change_view),
            validator_pool: None,
            is_suspended: None,
        },
        AccountStakingInfo::Baker {
            staked_amount,
            restake_earnings,
            baker_info,
            pending_change,
            pool_info,
            is_suspended,
        } => StakeDetailsView {
            mode: "validator".to_owned(),
            staked_amount: Some(format!("{} CCD", staked_amount)),
            restake_earnings: Some(*restake_earnings),
            delegation_target: None,
            validator_id: Some(baker_info.baker_id.to_string()),
            pending_change: pending_change.map(pending_change_view),
            validator_pool: pool_info.as_ref().map(pool_view),
            is_suspended: Some(*is_suspended),
        },
    })
}

/// Render human-readable staking lines.
pub(crate) fn render_stake_details_lines(view: Option<&StakeDetailsView>) -> Vec<String> {
    let Some(view) = view else {
        return vec!["mode: none".to_owned(), "no staking configured.".to_owned()];
    };

    let mut lines = vec![format!("mode: {}", view.mode)];
    if let Some(amount) = &view.staked_amount {
        lines.push(format!("staked amount: {amount}"));
    }
    if let Some(target) = &view.delegation_target {
        lines.push(format!("delegation target: {target}"));
    }
    if view.mode == "validator"
        && let Some(validator_id) = &view.validator_id
    {
        lines.push(format!("validator id: {validator_id}"));
    }
    if let Some(restake_earnings) = view.restake_earnings {
        lines.push(format!(
            "restake earnings: {}",
            if restake_earnings {
                "enabled"
            } else {
                "disabled"
            }
        ));
    }
    if let Some(is_suspended) = view.is_suspended {
        lines.push(format!(
            "suspended: {}",
            if is_suspended { "yes" } else { "no" }
        ));
    }
    if let Some(pool) = &view.validator_pool {
        lines.push("validator pool:".to_owned());
        lines.push(format!("  open status: {}", pool.open_status));
        lines.push(format!("  metadata url: {}", pool.metadata_url));
        lines.push(format!(
            "  commission rates: finalization {}, baking {}, transaction {}",
            pool.commission_rates.finalization,
            pool.commission_rates.baking,
            pool.commission_rates.transaction
        ));
    }
    if let Some(pending_change) = &view.pending_change {
        lines.push("pending change:".to_owned());
        lines.push(format!("  kind: {}", pending_change.kind));
        if let Some(new_stake) = &pending_change.new_stake {
            lines.push(format!("  new stake: {new_stake}"));
        }
        lines.push(format!(
            "  effective time: {}",
            pending_change.effective_time
        ));
    }
    lines
}

/// Return a short staking mode label for confirmations.
pub(crate) fn staking_mode_label(staking: Option<&AccountStakingInfo>) -> &'static str {
    match staking {
        Some(AccountStakingInfo::Delegated { .. }) => "delegated",
        Some(AccountStakingInfo::Baker { .. }) => "validator",
        None => "none",
    }
}

/// Query active validator-pool delegation limits.
pub(crate) async fn query_validator_pool_delegation_capacity(
    client: &mut v2::Client,
    validator_id: BakerId,
) -> Result<Option<(Amount, Amount)>> {
    let pool = client
        .get_pool_info(BlockIdentifier::LastFinal, validator_id)
        .await
        .with_context(|| format!("failed to query validator pool {validator_id}"))?
        .response;
    Ok(pool
        .active_baker_pool_status
        .map(|status| (status.delegated_capital, status.delegated_capital_cap)))
}

/// Build a configure-delegation transaction.
pub(crate) fn build_configure_delegation_transaction(
    wallet: &WalletAccount,
    nonce: concordium_rust_sdk::types::Nonce,
    payload: ConfigureDelegationPayload,
) -> BlockItem<concordium_rust_sdk::base::transactions::EncodedPayload> {
    let expiry = TransactionTime::seconds_after(300);
    BlockItem::AccountTransaction(send::configure_delegation(
        wallet,
        wallet.address,
        nonce,
        expiry,
        payload,
    ))
}

/// Build a validator-removal transaction.
pub(crate) fn build_remove_validator_transaction(
    wallet: &WalletAccount,
    nonce: concordium_rust_sdk::types::Nonce,
) -> BlockItem<concordium_rust_sdk::base::transactions::EncodedPayload> {
    let expiry = TransactionTime::seconds_after(300);
    let payload = ConfigureBakerPayload::new_remove_baker();
    BlockItem::AccountTransaction(send::configure_baker(
        wallet,
        wallet.address,
        nonce,
        expiry,
        payload,
    ))
}

/// Parse a CCD capital input.
pub(crate) fn parse_capital(value: Option<&str>) -> Result<Option<Amount>> {
    value
        .map(|value| parse_decimal_ccd_amount(Some(value)))
        .transpose()
}

/// Wait for transaction finalization and print the finalized summary.
pub(crate) async fn wait_for_finalization(
    client: &mut v2::Client,
    transaction_hash: &concordium_rust_sdk::base::hashes::TransactionHash,
    network_name: &str,
    endpoint_label: &str,
) -> Result<()> {
    let spin = spinner();
    spin.start("Waiting for transaction finalization...");
    let (block_hash, summary) = client
        .wait_until_finalized(transaction_hash)
        .await
        .context("failed while waiting for transaction finalization")?;
    spin.clear();
    let block_time = client.get_block_info(block_hash).await.ok().map(|info| {
        info.response
            .block_slot_time
            .to_rfc3339_opts(SecondsFormat::Secs, true)
    });
    println!(
        "{}",
        render_finalized_summary(
            transaction_hash,
            &format!("{network_name} @ {endpoint_label}"),
            &block_hash,
            &summary,
            block_time.as_ref(),
        )?
    );
    Ok(())
}

fn validator_id_exists(bakers: &[BakerId], validator_id: BakerId) -> bool {
    bakers
        .iter()
        .copied()
        .any(|candidate| candidate == validator_id)
}

fn format_delegation_target(target: &DelegationTarget) -> String {
    match target {
        DelegationTarget::Passive => "passive".to_owned(),
        DelegationTarget::Baker { baker_id } => format!("validator {baker_id}"),
    }
}

fn delegation_target_validator_id(target: &DelegationTarget) -> Option<String> {
    match target {
        DelegationTarget::Passive => None,
        DelegationTarget::Baker { baker_id } => Some(baker_id.to_string()),
    }
}

fn pending_change_view(change: StakePendingChange) -> StakePendingChangeView {
    match change {
        StakePendingChange::ReduceStake {
            new_stake,
            effective_time,
        } => StakePendingChangeView {
            kind: "reduce".to_owned(),
            effective_time: effective_time.to_rfc3339_opts(SecondsFormat::Secs, true),
            new_stake: Some(format!("{} CCD", new_stake)),
        },
        StakePendingChange::RemoveStake { effective_time } => StakePendingChangeView {
            kind: "remove".to_owned(),
            effective_time: effective_time.to_rfc3339_opts(SecondsFormat::Secs, true),
            new_stake: None,
        },
    }
}

fn pool_view(pool: &concordium_rust_sdk::types::BakerPoolInfo) -> ValidatorPoolView {
    ValidatorPoolView {
        open_status: format_open_status(pool.open_status.as_ref().known()),
        metadata_url: pool.metadata_url.to_string(),
        commission_rates: commission_rates_view(pool.commission_rates),
    }
}

fn commission_rates_view(rates: CommissionRates) -> CommissionRatesView {
    CommissionRatesView {
        finalization: format!("{:?}", rates.finalization),
        baking: format!("{:?}", rates.baking),
        transaction: format!("{:?}", rates.transaction),
    }
}

fn format_open_status(status: Option<&OpenStatus>) -> String {
    match status {
        Some(OpenStatus::OpenForAll) => "open-for-all".to_owned(),
        Some(OpenStatus::ClosedForNew) => "closed-for-new".to_owned(),
        Some(OpenStatus::ClosedForAll) => "closed-for-all".to_owned(),
        None => "unknown".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_absent_stake() {
        let lines = render_stake_details_lines(None);
        assert_eq!(lines[0], "mode: none");
        assert!(lines[1].contains("no staking configured"));
    }

    #[test]
    fn validator_validation_helper_matches_known_ids() {
        let bakers = vec![BakerId { id: 7u64.into() }, BakerId { id: 42u64.into() }];
        assert!(validator_id_exists(&bakers, BakerId { id: 42u64.into() }));
        assert!(!validator_id_exists(&bakers, BakerId { id: 9u64.into() }));
    }

    #[test]
    fn renders_validator_stake_details() {
        let lines = render_stake_details_lines(Some(&StakeDetailsView {
            mode: "validator".to_owned(),
            staked_amount: Some("100 CCD".to_owned()),
            restake_earnings: Some(true),
            delegation_target: None,
            validator_id: Some("42".to_owned()),
            pending_change: Some(StakePendingChangeView {
                kind: "remove".to_owned(),
                effective_time: "2026-01-01T00:00:00Z".to_owned(),
                new_stake: None,
            }),
            validator_pool: Some(ValidatorPoolView {
                open_status: "open-for-all".to_owned(),
                metadata_url: "https://example.invalid".to_owned(),
                commission_rates: CommissionRatesView {
                    finalization: "0%".to_owned(),
                    baking: "10%".to_owned(),
                    transaction: "20%".to_owned(),
                },
            }),
            is_suspended: Some(false),
        }));
        assert!(lines.iter().any(|line| line == "mode: validator"));
        assert!(lines.iter().any(|line| line == "validator id: 42"));
        assert!(lines.iter().any(|line| line == "pending change:"));
    }
}
