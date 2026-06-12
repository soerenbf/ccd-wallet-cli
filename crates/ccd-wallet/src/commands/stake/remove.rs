//! Stake-removal command.

use crate::{
    cli::StakeRemoveArgs,
    commands::stake::shared::{
        build_configure_delegation_transaction, build_remove_validator_transaction,
        query_account_info, resolve_mutation_context, wait_for_finalization,
    },
};
use anyhow::{Context, Result, bail};
use cliclack::{confirm, spinner};
use concordium_rust_sdk::{
    base::transactions::ConfigureDelegationPayload, types::AccountStakingInfo, v2::BlockIdentifier,
};
use rusqlite::Connection;

/// Run `stake remove`.
///
/// # Arguments
/// * `conn` - Open wallet store connection.
/// * `args` - Parsed command arguments.
///
/// # Errors
/// Returns an error if the account has no staking, or if submission/finalization fails.
pub(super) async fn remove(conn: &Connection, args: StakeRemoveArgs) -> Result<()> {
    let mut context = resolve_mutation_context(
        conn,
        args.account.as_deref(),
        args.network.as_deref(),
        args.node,
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
    let prompt = match current_staking {
        Some(AccountStakingInfo::Delegated { .. }) => format!(
            "Remove delegation on {} ({})\naccount: {}",
            context.network_name, context.endpoint_label, context.account_label
        ),
        Some(AccountStakingInfo::Baker { .. }) => format!(
            "Remove validator staking on {} ({})\naccount: {}",
            context.network_name, context.endpoint_label, context.account_label
        ),
        None => bail!(
            "no staking configuration exists to remove for account '{}'",
            context.account_label
        ),
    };
    let approved = confirm(prompt)
        .initial_value(false)
        .interact()
        .context("failed to read confirmation")?;
    if !approved {
        cliclack::log::warning("stake removal declined by user")?;
        return Ok(());
    }

    let nonce = context
        .client
        .get_next_account_sequence_number(&context.wallet.address)
        .await
        .context("failed to query next account sequence number")?
        .nonce;
    let transaction = match current_staking {
        Some(AccountStakingInfo::Delegated { .. }) => build_configure_delegation_transaction(
            &context.wallet,
            nonce,
            ConfigureDelegationPayload::new_remove_delegation(),
        ),
        Some(AccountStakingInfo::Baker { .. }) => {
            build_remove_validator_transaction(&context.wallet, nonce)
        }
        None => unreachable!("checked above"),
    };

    let spin = spinner();
    spin.start("Submitting stake removal transaction...");
    let transaction_hash = context
        .client
        .send_block_item(&transaction)
        .await
        .context("failed to submit stake removal transaction")?;
    spin.clear();
    cliclack::log::success(format!(
        "Submitted stake removal transaction on {} ({}): {}",
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
