//! Token holder command implementations.

use crate::{
    cli::{TokenAmountArgs, TokenTransferArgs},
    commands::{
        input::{AccountReference, Promptable, TokenAmountInput},
        token::shared,
    },
};
use anyhow::Result;
use cliclack::spinner;
use concordium_rust_sdk::protocol_level_tokens::{
    TokenId,
    token_client::{TransactionMetadata, TransferTokens, Validation},
};
use rusqlite::Connection;

#[derive(Clone, Debug)]
struct PreparedTokenTransfer {
    context: shared::PreparedTokenMutationContext,
    token_id: Promptable<TokenId>,
    recipient: Promptable<AccountReference>,
    amount: Promptable<TokenAmountInput>,
}

impl PreparedTokenTransfer {
    fn from_args(args: TokenTransferArgs) -> Result<Self> {
        Ok(Self {
            context: shared::PreparedTokenMutationContext::from_raw(
                args.account.as_deref(),
                args.network.as_deref(),
                args.node,
                args.non_interactive,
                args.no_defaults,
                args.no_wait,
                true,
            )?,
            token_id: Promptable::from_option(args.token_id, "token id"),
            recipient: Promptable::from_option(args.recipient, "recipient"),
            amount: Promptable::from_option(args.amount, "amount"),
        })
    }
}

#[derive(Clone, Debug)]
struct PreparedTokenAmountChange {
    context: shared::PreparedTokenMutationContext,
    token_id: Promptable<TokenId>,
    amount: Promptable<TokenAmountInput>,
}

impl PreparedTokenAmountChange {
    fn from_args(args: TokenAmountArgs) -> Result<Self> {
        Ok(Self {
            context: shared::PreparedTokenMutationContext::from_raw(
                args.account.as_deref(),
                args.network.as_deref(),
                args.node,
                args.non_interactive,
                args.no_defaults,
                args.no_wait,
                true,
            )?,
            token_id: Promptable::from_option(args.token_id, "token id"),
            amount: Promptable::from_option(args.amount, "amount"),
        })
    }
}

/// Submit a protocol-level token transfer.
pub(super) async fn transfer(conn: &Connection, args: TokenTransferArgs) -> Result<()> {
    let prepared = PreparedTokenTransfer::from_args(args)?;
    let mut context = shared::resolve_prepared_mutation_context(conn, &prepared.context).await?;
    let mut balance_client = context.client.clone();
    let available_balances =
        shared::account_available_balances(&mut balance_client, context.wallet.address).await?;
    let token_id = prepared
        .token_id
        .resolve_with(prepared.context.input_mode(), || {
            shared::select_token_from_balances(&available_balances)
        })?
        .into_value();
    let mut token_client = shared::init_token_client(context.client.clone(), token_id).await?;
    let recipient_input = match &prepared.recipient {
        Promptable::Provided(recipient) => Some(recipient.to_string()),
        Promptable::Missing { .. } => None,
    };
    let recipient = shared::resolve_account_address(
        conn,
        &mut context,
        recipient_input.as_deref(),
        "Recipient account address or local label:",
        "recipient",
        !prepared.context.input_mode().prompts_allowed(),
    )?;
    let token_amount = shared::resolve_token_amount(
        match &prepared.amount {
            Promptable::Provided(amount) => Some(amount.as_str()),
            Promptable::Missing { .. } => None,
        },
        token_client.token_info().token_state.decimals,
        available_balances
            .get(&token_client.token_info().token_id)
            .copied(),
        "available",
        !prepared.context.input_mode().prompts_allowed(),
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
        "Approve and submit this token transfer?",
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
    if prepared.context.should_wait_for_finalization() {
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
    let prepared = PreparedTokenAmountChange::from_args(args)?;
    let mut context = shared::resolve_prepared_mutation_context(conn, &prepared.context).await?;
    let token_id = prepared
        .token_id
        .resolve_with(prepared.context.input_mode(), shared::prompt_token_id)?
        .into_value();
    let mut token_client = shared::init_token_client(context.client.clone(), token_id).await?;
    let token_amount = shared::resolve_token_amount(
        match &prepared.amount {
            Promptable::Provided(amount) => Some(amount.as_str()),
            Promptable::Missing { .. } => None,
        },
        token_client.token_info().token_state.decimals,
        None,
        "amount",
        !prepared.context.input_mode().prompts_allowed(),
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
        &format!("Approve and submit this token {verb}?"),
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
    if prepared.context.should_wait_for_finalization() {
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
