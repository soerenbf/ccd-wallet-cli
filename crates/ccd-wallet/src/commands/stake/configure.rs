//! Delegation-configuration command.

use crate::{
    cli::StakeConfigureDelegationArgs,
    commands::{
        input::{
            AccountLabel, Defaultable, FinalizationPolicy, InputMode, NetworkName, Promptable,
            ValidationPolicy,
        },
        stake::shared::{
            build_configure_delegation_transaction, parse_capital, query_account_info,
            query_validator_ids, query_validator_pool_delegation_capacity,
            resolve_mutation_context, staking_mode_label, validate_validator_id,
            wait_for_finalization,
        },
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

/// Prepared command input for `stake configure delegation`.
#[derive(Clone, Debug)]
struct PreparedStakeConfigureDelegation {
    account: Promptable<AccountLabel>,
    network: Defaultable<NetworkName>,
    node: Option<concordium_rust_sdk::v2::Endpoint>,
    input_mode: InputMode,
    finalization: FinalizationPolicy,
    validation: ValidationPolicy,
    target: Promptable<DelegationTarget>,
    capital: Promptable<Amount>,
    restake_earnings: Promptable<bool>,
}

impl PreparedStakeConfigureDelegation {
    fn from_args(args: &StakeConfigureDelegationArgs) -> Self {
        Self {
            account: Promptable::from_option(args.account.clone(), "account"),
            network: Defaultable::from_option(args.network_node.network.clone(), "network"),
            node: args.network_node.node.clone(),
            input_mode: InputMode::from(&args.input_mode),
            finalization: FinalizationPolicy::from(&args.submission),
            validation: ValidationPolicy::from_no_validate(args.no_validate),
            target: Promptable::from_option(
                DelegationTargetInput::from_args(args).map(DelegationTargetInput::into_target),
                "delegation target",
            ),
            capital: Promptable::from_option(args.capital, "delegated capital"),
            restake_earnings: Promptable::from_option(
                restake_from_args(args),
                "restake preference",
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum DelegationTargetInput {
    Passive,
    Validator(u64),
}

impl DelegationTargetInput {
    fn from_args(args: &StakeConfigureDelegationArgs) -> Option<Self> {
        if args.passive {
            Some(Self::Passive)
        } else {
            args.validator.map(Self::Validator)
        }
    }

    fn into_target(self) -> DelegationTarget {
        match self {
            Self::Passive => DelegationTarget::Passive,
            Self::Validator(validator) => validator_target(validator),
        }
    }
}

fn restake_from_args(args: &StakeConfigureDelegationArgs) -> Option<bool> {
    if args.restake {
        Some(true)
    } else if args.no_restake {
        Some(false)
    } else {
        None
    }
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
    let prepared = PreparedStakeConfigureDelegation::from_args(&args);
    let account = match &prepared.account {
        Promptable::Provided(account) => Some(account.as_str()),
        Promptable::Missing { .. } => None,
    };
    let network = match &prepared.network {
        Defaultable::Provided(network) => Some(network.as_str()),
        Defaultable::Missing { .. } => None,
    };
    let mut context = resolve_mutation_context(
        conn,
        account,
        network,
        prepared.node.clone(),
        !prepared.input_mode.prompts_allowed(),
        !prepared.input_mode.defaults_allowed(),
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
        &prepared,
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
    if !prepared.finalization.should_wait() {
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
    prepared: &PreparedStakeConfigureDelegation,
    current_info: &AccountInfo,
    current_staking: Option<&AccountStakingInfo>,
    current_state: &DelegationState,
) -> Result<DelegationState> {
    let should_validate = prepared.validation.should_validate();
    let defaults_allowed = prepared.input_mode.defaults_allowed();

    let target = prepared
        .target
        .clone()
        .resolve_with_async(prepared.input_mode, || async {
            prompt_target(
                client,
                current_staking,
                current_state,
                should_validate,
                defaults_allowed,
            )
            .await
        })
        .await?
        .into_value();
    if should_validate {
        validate_requested_target(client, &target).await?;
    }

    let capital = prepared
        .capital
        .clone()
        .resolve_with_async(prepared.input_mode, || async {
            prompt_capital(
                client,
                current_info,
                current_staking,
                &target,
                current_state,
                should_validate,
                defaults_allowed,
            )
            .await?
            .context("delegated capital prompt did not return a value")
        })
        .await?
        .into_value();
    if should_validate {
        validate_requested_capital(client, current_info, current_staking, &target, capital).await?;
    }

    let restake_earnings = prepared
        .restake_earnings
        .clone()
        .resolve_with(prepared.input_mode, || {
            prompt_restake(current_state, defaults_allowed)?
                .context("restake prompt did not return a value")
        })?
        .into_value();

    Ok(DelegationState {
        capital: Some(capital),
        restake_earnings: Some(restake_earnings),
        delegation_target: Some(target),
    })
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
    validate: bool,
    defaults_allowed: bool,
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
                format!("currently active: {baker_id}"),
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
            let default_validator = if defaults_allowed {
                match current_staking {
                    Some(AccountStakingInfo::Delegated {
                        delegation_target: DelegationTarget::Baker { baker_id },
                        ..
                    }) => Some(baker_id.to_string()),
                    _ => None,
                }
            } else {
                None
            };
            let mut prompt = input("Validator id:");
            if let Some(default_validator) = default_validator.as_deref() {
                prompt = prompt.default_input(default_validator);
            }
            if validate {
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
    validate: bool,
    defaults_allowed: bool,
) -> Result<Option<Amount>> {
    let prompt_label = match current_state.capital {
        Some(current_capital) => format!("Delegated capital in CCD (current: {current_capital}):"),
        None => "Delegated capital in CCD:".to_owned(),
    };
    let mut prompt = input(prompt_label);
    if defaults_allowed && let Some(current_capital) = current_state.capital {
        let default = current_capital.to_string();
        prompt = prompt.default_input(&default);
    }
    if validate {
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

fn prompt_restake(current_state: &DelegationState, defaults_allowed: bool) -> Result<Option<bool>> {
    let current_restake = current_state.restake_earnings;
    let prompt = match current_restake {
        Some(value) => format!("Restake delegation earnings? (current: {value})"),
        None => "Restake delegation earnings?".to_owned(),
    };
    let initial = if defaults_allowed {
        current_restake.unwrap_or(false)
    } else {
        false
    };
    Ok(Some(confirm(prompt).initial_value(initial).interact()?))
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
    use crate::commands::input::{InputModeArgs, NetworkNodeArgs, SubmissionWaitArgs};

    fn delegation_args() -> StakeConfigureDelegationArgs {
        StakeConfigureDelegationArgs {
            account: Some("alice".parse().unwrap()),
            validator: Some(42),
            passive: false,
            capital: Some("1.5".parse().unwrap()),
            restake: true,
            no_restake: false,
            network_node: NetworkNodeArgs {
                network: Some("testnet".parse().unwrap()),
                node: None,
            },
            submission: SubmissionWaitArgs { no_wait: true },
            no_validate: true,
            input_mode: InputModeArgs {
                non_interactive: true,
                no_defaults: false,
            },
        }
    }

    #[test]
    fn prepared_delegation_maps_explicit_args_to_policies_and_promptables() {
        let args = delegation_args();
        let prepared = PreparedStakeConfigureDelegation::from_args(&args);

        assert!(!prepared.input_mode.prompts_allowed());
        assert!(!prepared.input_mode.defaults_allowed());
        assert!(!prepared.finalization.should_wait());
        assert!(!prepared.validation.should_validate());
        assert!(matches!(prepared.account, Promptable::Provided(_)));
        assert!(matches!(prepared.network, Defaultable::Provided(_)));
        assert!(matches!(prepared.target, Promptable::Provided(_)));
        assert!(matches!(prepared.capital, Promptable::Provided(_)));
        assert!(matches!(
            prepared.restake_earnings,
            Promptable::Provided(true)
        ));
    }

    #[test]
    fn missing_restake_errors_in_non_interactive_mode() {
        let mut args = delegation_args();
        args.restake = false;
        args.no_restake = false;
        let prepared = PreparedStakeConfigureDelegation::from_args(&args);

        let error = prepared
            .restake_earnings
            .resolve_with(prepared.input_mode, || Ok(true))
            .unwrap_err();

        assert!(error.to_string().contains("restake preference"));
    }

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
