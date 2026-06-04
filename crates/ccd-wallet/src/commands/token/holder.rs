//! Token holder command implementations.

use crate::{
    cli::{TokenAmountArgs, TokenTransferArgs},
    commands::token::shared,
};
use anyhow::Result;
use cliclack::spinner;
use concordium_rust_sdk::protocol_level_tokens::token_client::{
    TransactionMetadata, TransferTokens, Validation,
};
use rusqlite::Connection;

/// Submit a protocol-level token transfer.
pub(super) async fn transfer(conn: &Connection, args: TokenTransferArgs) -> Result<()> {
    let mut context = shared::resolve_mutation_context(
        conn,
        args.account.as_deref(),
        args.network.as_deref(),
        args.node,
        args.non_interactive,
        args.no_defaults,
        true,
    )
    .await?;
    let mut balance_client = context.client.clone();
    let available_balances =
        shared::account_available_balances(&mut balance_client, context.wallet.address).await?;
    let token_id = shared::resolve_token_from_balances(
        args.token_id,
        &available_balances,
        args.non_interactive,
    )?;
    let mut token_client = shared::init_token_client(context.client.clone(), token_id).await?;
    let recipient = shared::resolve_account_address(
        args.recipient.as_deref(),
        "Recipient account address:",
        "recipient",
        args.non_interactive,
    )?;
    let token_amount = shared::resolve_token_amount(
        args.amount.as_deref(),
        token_client.token_info().token_state.decimals,
        available_balances
            .get(&token_client.token_info().token_id)
            .copied(),
        "available",
        args.non_interactive,
    )?;
    let payload = TransferTokens {
        amount: token_amount,
        recipient,
        memo: None,
    };

    cliclack::log::info(format!(
        "Token transfer\nnetwork: {} ({})\naccount: {}\ntoken: {}\nrecipient: {}\namount: {}",
        context.network_name,
        context.endpoint_label,
        context.wallet.address,
        token_client.token_info().token_id,
        payload.recipient,
        payload.amount,
    ))?;
    if !shared::confirm_submission(
        "Approve and submit this token transfer? Type y to approve:",
        "token transfer declined by user",
    )? {
        return Ok(());
    }

    let spin = spinner();
    spin.start("Submitting token transfer...");
    let transaction_hash = token_client
        .transfer(
            &context.wallet,
            vec![payload.clone()],
            Some(TransactionMetadata::default()),
            Validation::Validate,
        )
        .await?;
    spin.clear();
    cliclack::log::success(format!(
        "Submitted token transfer on {} ({}): {transaction_hash}",
        context.network_name, context.endpoint_label
    ))?;
    if !args.no_wait {
        shared::wait_for_finalization(
            &mut context.client,
            &transaction_hash,
            &context.network_name,
            &context.endpoint_label,
        )
        .await?;
    }
    Ok(())
}

/// Submit a protocol-level token mint.
pub(super) async fn mint(conn: &Connection, args: TokenAmountArgs) -> Result<()> {
    submit_amount_change(conn, args, true).await
}

/// Submit a protocol-level token burn.
pub(super) async fn burn(conn: &Connection, args: TokenAmountArgs) -> Result<()> {
    submit_amount_change(conn, args, false).await
}

async fn submit_amount_change(conn: &Connection, args: TokenAmountArgs, mint: bool) -> Result<()> {
    let mut context = shared::resolve_mutation_context(
        conn,
        args.account.as_deref(),
        args.network.as_deref(),
        args.node,
        args.non_interactive,
        args.no_defaults,
        true,
    )
    .await?;
    let token_id = shared::resolve_token_id(args.token_id, args.non_interactive)?;
    let mut token_client = shared::init_token_client(context.client.clone(), token_id).await?;
    let token_amount = shared::resolve_token_amount(
        args.amount.as_deref(),
        token_client.token_info().token_state.decimals,
        None,
        "amount",
        args.non_interactive,
    )?;
    let verb = if mint { "mint" } else { "burn" };

    cliclack::log::info(format!(
        "Token {verb}\nnetwork: {} ({})\naccount: {}\ntoken: {}\namount: {}",
        context.network_name,
        context.endpoint_label,
        context.wallet.address,
        token_client.token_info().token_id,
        token_amount,
    ))?;
    if !shared::confirm_submission(
        &format!("Approve and submit this token {verb}? Type y to approve:"),
        &format!("token {verb} declined by user"),
    )? {
        return Ok(());
    }

    let spin = spinner();
    spin.start(format!("Submitting token {verb}..."));
    let transaction_hash = if mint {
        token_client
            .mint(
                &context.wallet,
                token_amount,
                Some(TransactionMetadata::default()),
                Validation::Validate,
            )
            .await?
    } else {
        token_client
            .burn(
                &context.wallet,
                token_amount,
                Some(TransactionMetadata::default()),
                Validation::Validate,
            )
            .await?
    };
    spin.clear();
    cliclack::log::success(format!(
        "Submitted token {verb} on {} ({}): {transaction_hash}",
        context.network_name, context.endpoint_label
    ))?;
    if !args.no_wait {
        shared::wait_for_finalization(
            &mut context.client,
            &transaction_hash,
            &context.network_name,
            &context.endpoint_label,
        )
        .await?;
    }
    Ok(())
}
