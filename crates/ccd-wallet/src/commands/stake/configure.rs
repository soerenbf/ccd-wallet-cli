//! Delegation-configuration command.

use crate::{
    cli::StakeConfigureDelegationArgs,
    commands::stake::shared::{
        build_configure_delegation_transaction, parse_capital, query_account_info,
        query_validator_ids, query_validator_pool_delegation_capacity, resolve_mutation_context,
        staking_mode_label, validate_validator_id, wait_for_finalization,
    },
};
use anyhow::{Context, Result, bail};
use cliclack::{confirm, input, select, spinner};
use concordium_rust_sdk::{
    base::{
        base::DelegationTarget, common::types::Amount, transactions::ConfigureDelegationPayload,
    },
    types::{AccountInfo, AccountStakingInfo, BakerId},
    v2::BlockIdentifier,
};
use rusqlite::Connection;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct DelegationState {
    capital: Option<Amount>,
    restake_earnings: Option<bool>,
    delegation_target: Option<DelegationTarget>,
}

/// Run `stake configure delegation`.
///
/// # Arguments
/// * `conn` - Open wallet store connection.
/// * `args` - Parsed command arguments.
///
/// # Errors
/// Returns an error if prompting, validation, transaction submission, or finalization fails.
pub(super) async fn configure_delegation(
    conn: &Connection,
    args: StakeConfigureDelegationArgs,
) -> Result<()> {
    let mut context = resolve_mutation_context(
        conn,
        args.account.as_deref(),
        args.network.as_deref(),
        args.node.clone(),
        args.non_interactive,
        args.no_defaults,
    )
    .await?;
    let current_info = query_account_info(
        &mut context.client,
        context.wallet.address,
        BlockIdentifier::LastFinal,
    )
    .await?;
    let current_staking = current_info
        .account_stake
        .as_ref()
        .and_then(|staking| staking.as_ref().known());
    let current_state = current_delegation_state(current_staking);
    let requested_state = resolve_requested_state(
        &mut context.client,
        &args,
        &current_info,
        current_staking,
        &current_state,
    )
    .await?;
    let changes = diff_requested_state(&current_state, &requested_state);
    if changes.capital.is_none()
        && changes.restake_earnings.is_none()
        && changes.delegation_target.is_none()
    {
        bail!("delegation settings are unchanged; nothing to submit");
    }

    let mut payload = ConfigureDelegationPayload::new();
    if let Some(capital) = changes.capital {
        payload.set_capital(capital);
    }
    if let Some(restake_earnings) = changes.restake_earnings {
        payload.set_restake_earnings(restake_earnings);
    }
    if let Some(target) = changes.delegation_target {
        payload.set_delegation_target(target);
    }

    let prompt = render_confirmation(
        &context.network_name,
        &context.endpoint_label,
        &context.account_label,
        current_staking,
        &requested_state,
    );
    let approved = confirm(prompt)
        .initial_value(false)
        .interact()
        .context("failed to read confirmation")?;
    if !approved {
        cliclack::log::warning("stake delegation update declined by user")?;
        return Ok(());
    }

    let nonce = context
        .client
        .get_next_account_sequence_number(&context.wallet.address)
        .await
        .context("failed to query next account sequence number")?
        .nonce;
    let transaction = build_configure_delegation_transaction(&context.wallet, nonce, payload);
    let spin = spinner();
    spin.start("Submitting delegation transaction...");
    let transaction_hash = context
        .client
        .send_block_item(&transaction)
        .await
        .context("failed to submit delegation transaction")?;
    spin.clear();
    cliclack::log::success(format!(
        "Submitted delegation transaction on {} ({}): {}",
        context.network_name, context.endpoint_label, transaction_hash
    ))?;
    if args.no_wait {
        return Ok(());
    }
    wait_for_finalization(
        &mut context.client,
        &transaction_hash,
        &context.network_name,
        &context.endpoint_label,
    )
    .await
}

fn current_delegation_state(current_staking: Option<&AccountStakingInfo>) -> DelegationState {
    match current_staking {
        Some(AccountStakingInfo::Delegated {
            staked_amount,
            restake_earnings,
            delegation_target,
            ..
        }) => DelegationState {
            capital: Some(*staked_amount),
            restake_earnings: Some(*restake_earnings),
            delegation_target: Some(delegation_target.clone()),
        },
        _ => DelegationState::default(),
    }
}

async fn resolve_requested_state(
    client: &mut concordium_rust_sdk::v2::Client,
    args: &StakeConfigureDelegationArgs,
    current_info: &AccountInfo,
    current_staking: Option<&AccountStakingInfo>,
    current_state: &DelegationState,
) -> Result<DelegationState> {
    let target = resolve_requested_target(client, args, current_staking, current_state).await?;
    let capital = resolve_requested_capital(
        client,
        args,
        current_info,
        current_staking,
        &target,
        current_state,
    )
    .await?;
    let restake_earnings = resolve_requested_restake(args, current_state)?;
    Ok(DelegationState {
        capital,
        restake_earnings,
        delegation_target: Some(target),
    })
}

async fn resolve_requested_target(
    client: &mut concordium_rust_sdk::v2::Client,
    args: &StakeConfigureDelegationArgs,
    current_staking: Option<&AccountStakingInfo>,
    current_state: &DelegationState,
) -> Result<DelegationTarget> {
    if args.passive {
        return Ok(DelegationTarget::Passive);
    }
    if let Some(validator) = args.validator {
        let target = validator_target(validator);
        if !args.no_validate {
            validate_requested_target(client, &target).await?;
        }
        return Ok(target);
    }
    if args.non_interactive {
        bail!(
            "delegation target must be provided in --non-interactive mode with either --passive or --validator"
        );
    }
    prompt_target(client, current_staking, current_state, args.no_validate).await
}

async fn resolve_requested_capital(
    client: &mut concordium_rust_sdk::v2::Client,
    args: &StakeConfigureDelegationArgs,
    current_info: &AccountInfo,
    current_staking: Option<&AccountStakingInfo>,
    target: &DelegationTarget,
    current_state: &DelegationState,
) -> Result<Option<Amount>> {
    if let Some(capital) = parse_capital(args.capital.as_deref())? {
        if !args.no_validate {
            validate_requested_capital(client, current_info, current_staking, target, capital)
                .await?;
        }
        return Ok(Some(capital));
    }
    if args.non_interactive {
        bail!("delegated capital must be provided in --non-interactive mode with --capital");
    }
    prompt_capital(
        client,
        current_info,
        current_staking,
        target,
        current_state,
        args.no_validate,
    )
    .await
}

fn resolve_requested_restake(
    args: &StakeConfigureDelegationArgs,
    current_state: &DelegationState,
) -> Result<Option<bool>> {
    if args.restake {
        return Ok(Some(true));
    }
    if args.no_restake {
        return Ok(Some(false));
    }
    if args.non_interactive {
        bail!(
            "restake preference must be provided in --non-interactive mode with --restake or --no-restake"
        );
    }
    prompt_restake(current_state)
}

fn diff_requested_state(
    current_state: &DelegationState,
    requested_state: &DelegationState,
) -> DelegationState {
    DelegationState {
        capital: (requested_state.capital != current_state.capital)
            .then_some(requested_state.capital)
            .flatten(),
        restake_earnings: (requested_state.restake_earnings != current_state.restake_earnings)
            .then_some(requested_state.restake_earnings)
            .flatten(),
        delegation_target: (requested_state.delegation_target != current_state.delegation_target)
            .then_some(requested_state.delegation_target.clone())
            .flatten(),
    }
}

async fn prompt_target(
    client: &mut concordium_rust_sdk::v2::Client,
    current_staking: Option<&AccountStakingInfo>,
    current_state: &DelegationState,
    no_validate: bool,
) -> Result<DelegationTarget> {
    let mode_prompt = if current_state.delegation_target.is_some() {
        "Delegation target:"
    } else {
        "Delegation target to configure:"
    };
    let mode = match &current_state.delegation_target {
        Some(DelegationTarget::Passive) => select(mode_prompt)
            .item("passive", "Passive delegation", "currently active")
            .item("validator", "Validator pool", "choose a validator id")
            .interact()?,
        Some(DelegationTarget::Baker { baker_id }) => select(mode_prompt)
            .item(
                "validator",
                "Validator pool",
                &format!("currently active: {baker_id}"),
            )
            .item("passive", "Passive delegation", "")
            .interact()?,
        None => select(mode_prompt)
            .item("passive", "Passive delegation", "")
            .item("validator", "Validator pool", "choose a validator id")
            .interact()?,
    };

    match mode {
        "passive" => Ok(DelegationTarget::Passive),
        "validator" => {
            let default_validator = match current_staking {
                Some(AccountStakingInfo::Delegated {
                    delegation_target: DelegationTarget::Baker { baker_id },
                    ..
                }) => Some(baker_id.to_string()),
                _ => None,
            };
            let mut prompt = input("Validator id:");
            if let Some(default_validator) = default_validator.as_deref() {
                prompt = prompt.default_input(default_validator);
            }
            if !no_validate {
                let validators = query_validator_ids(client).await?;
                prompt = prompt.validate(move |value: &String| {
                    validate_validator_input(value, &validators).map_err(|err| err.to_string())
                });
            }
            let value: String = prompt.interact()?;
            let validator = value
                .trim()
                .parse::<u64>()
                .context("validator id must be an unsigned integer")?;
            Ok(validator_target(validator))
        }
        _ => unreachable!("unexpected delegation target selection"),
    }
}

async fn prompt_capital(
    client: &mut concordium_rust_sdk::v2::Client,
    current_info: &AccountInfo,
    current_staking: Option<&AccountStakingInfo>,
    target: &DelegationTarget,
    current_state: &DelegationState,
    no_validate: bool,
) -> Result<Option<Amount>> {
    let mut prompt = input("Delegated capital in CCD:");
    if let Some(current_capital) = current_state.capital {
        let default = current_capital.to_string();
        prompt = prompt.default_input(&default);
    }
    if !no_validate {
        let total_balance = current_info.account_amount;
        let pool_limit = match target {
            DelegationTarget::Baker { baker_id } => {
                query_validator_pool_delegation_capacity(client, *baker_id)
                    .await?
                    .map(|(delegated_capital, delegated_capital_cap)| {
                        (
                            *baker_id,
                            delegated_capital,
                            delegated_capital_cap,
                            current_validator_contribution(current_staking, *baker_id),
                        )
                    })
            }
            DelegationTarget::Passive => None,
        };
        prompt = prompt.validate(move |value: &String| {
            validate_capital_input(value, total_balance, pool_limit).map_err(|err| err.to_string())
        });
    }
    let value: String = prompt.interact()?;
    parse_capital(Some(value.trim()))
}

fn prompt_restake(current_state: &DelegationState) -> Result<Option<bool>> {
    let current_restake = current_state.restake_earnings;
    if current_restake.is_some() {
        Ok(Some(
            confirm("Restake delegation earnings?")
                .initial_value(current_restake.unwrap_or(false))
                .interact()?,
        ))
    } else {
        Ok(Some(
            confirm("Restake delegation earnings?")
                .initial_value(false)
                .interact()?,
        ))
    }
}

async fn validate_requested_target(
    client: &mut concordium_rust_sdk::v2::Client,
    target: &DelegationTarget,
) -> Result<()> {
    if let DelegationTarget::Baker { baker_id } = target {
        validate_validator_id(client, *baker_id).await?;
    }
    Ok(())
}

async fn validate_requested_capital(
    client: &mut concordium_rust_sdk::v2::Client,
    current_info: &AccountInfo,
    current_staking: Option<&AccountStakingInfo>,
    target: &DelegationTarget,
    capital: Amount,
) -> Result<()> {
    validate_capital_amount(capital, current_info.account_amount, None)?;

    let DelegationTarget::Baker { baker_id } = target else {
        return Ok(());
    };
    let Some((current_delegated_capital, delegated_capital_cap)) =
        query_validator_pool_delegation_capacity(client, *baker_id).await?
    else {
        return Ok(());
    };
    validate_capital_amount(
        capital,
        current_info.account_amount,
        Some((
            *baker_id,
            current_delegated_capital,
            delegated_capital_cap,
            current_validator_contribution(current_staking, *baker_id),
        )),
    )
}

fn validate_validator_input(value: &str, validators: &[BakerId]) -> Result<()> {
    let validator = value
        .trim()
        .parse::<u64>()
        .context("validator id must be an unsigned integer")?;
    let target = validator_target(validator);
    let DelegationTarget::Baker { baker_id } = target else {
        unreachable!("validator target helper must return baker target");
    };
    if validators
        .iter()
        .copied()
        .any(|candidate| candidate == baker_id)
    {
        return Ok(());
    }
    bail!(
        "validator id {} is not valid on the selected network",
        baker_id
    )
}

fn validate_capital_input(
    value: &str,
    total_balance: Amount,
    pool_limit: Option<(BakerId, Amount, Amount, Amount)>,
) -> Result<()> {
    let capital = parse_capital(Some(value.trim()))?.context("delegated capital is required")?;
    validate_capital_amount(capital, total_balance, pool_limit)
}

fn validate_capital_amount(
    capital: Amount,
    total_balance: Amount,
    pool_limit: Option<(BakerId, Amount, Amount, Amount)>,
) -> Result<()> {
    if capital > total_balance {
        bail!(
            "delegated capital {} CCD exceeds the account total balance {} CCD",
            capital,
            total_balance
        );
    }
    if let Some((
        validator_id,
        current_delegated_capital,
        delegated_capital_cap,
        current_contribution,
    )) = pool_limit
    {
        let projected_delegated_capital = current_delegated_capital
            .checked_sub(current_contribution)
            .unwrap_or_else(Amount::zero)
            .checked_add(capital)
            .unwrap_or(capital);
        if projected_delegated_capital > delegated_capital_cap {
            bail!(
                "delegated capital {} CCD would exceed validator {} pool cap {} CCD (projected delegated capital: {} CCD)",
                capital,
                validator_id,
                delegated_capital_cap,
                projected_delegated_capital
            );
        }
    }
    Ok(())
}

fn current_validator_contribution(
    current_staking: Option<&AccountStakingInfo>,
    validator_id: BakerId,
) -> Amount {
    match current_staking {
        Some(AccountStakingInfo::Delegated {
            staked_amount,
            delegation_target: DelegationTarget::Baker { baker_id },
            ..
        }) if *baker_id == validator_id => *staked_amount,
        _ => Amount::zero(),
    }
}

fn render_confirmation(
    network_name: &str,
    endpoint_label: &str,
    account_label: &str,
    current_staking: Option<&AccountStakingInfo>,
    requested_state: &DelegationState,
) -> String {
    let mut lines = vec![
        format!("Configure delegation on {network_name} ({endpoint_label})"),
        format!("account: {account_label}"),
        format!("current mode: {}", staking_mode_label(current_staking)),
    ];
    if matches!(current_staking, Some(AccountStakingInfo::Baker { .. })) {
        lines.push("transition: validator -> delegated".to_owned());
    }
    if let Some(capital) = requested_state.capital {
        lines.push(format!("capital: {} CCD", capital));
    }
    if let Some(restake_earnings) = requested_state.restake_earnings {
        lines.push(format!(
            "restake earnings: {}",
            if restake_earnings {
                "enabled"
            } else {
                "disabled"
            }
        ));
    }
    if let Some(target) = requested_state.delegation_target.as_ref() {
        lines.push(format!("target: {}", render_target(target)));
    }
    lines.join("\n")
}

fn validator_target(validator: u64) -> DelegationTarget {
    DelegationTarget::Baker {
        baker_id: BakerId {
            id: validator.into(),
        },
    }
}

fn render_target(target: &DelegationTarget) -> String {
    match target {
        DelegationTarget::Passive => "passive".to_owned(),
        DelegationTarget::Baker { baker_id } => format!("validator {baker_id}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_omits_unchanged_values() {
        let current = DelegationState {
            capital: Some(Amount::from_micro_ccd(1_000_000)),
            restake_earnings: Some(true),
            delegation_target: Some(DelegationTarget::Passive),
        };
        let diff = diff_requested_state(&current, &current);
        assert_eq!(diff.capital, None);
        assert_eq!(diff.restake_earnings, None);
        assert_eq!(diff.delegation_target, None);
    }

    #[test]
    fn validator_contribution_only_counts_matching_target() {
        let staking = AccountStakingInfo::Delegated {
            staked_amount: Amount::from_micro_ccd(500),
            restake_earnings: true,
            delegation_target: validator_target(42),
            pending_change: None,
        };
        assert_eq!(
            current_validator_contribution(Some(&staking), BakerId { id: 42u64.into() }),
            Amount::from_micro_ccd(500)
        );
        assert_eq!(
            current_validator_contribution(Some(&staking), BakerId { id: 7u64.into() }),
            Amount::zero()
        );
    }

    #[test]
    fn capital_validation_rejects_exceeding_total_balance() {
        let error =
            validate_capital_amount(Amount::from_micro_ccd(11), Amount::from_micro_ccd(10), None)
                .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("exceeds the account total balance")
        );
    }
}
