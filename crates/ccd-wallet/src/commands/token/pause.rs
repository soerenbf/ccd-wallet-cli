//! Token pause and unpause command implementations.

use crate::{
    cli::TokenPauseArgs,
    commands::{input::Promptable, token::shared},
};
use anyhow::Result;
use cliclack::spinner;
use concordium_rust_sdk::protocol_level_tokens::{TokenId, token_client::TransactionMetadata};
use rusqlite::Connection;

#[derive(Clone, Debug)]
struct PreparedTokenPause {
    context: shared::PreparedTokenMutationContext,
    token_id: Promptable<TokenId>,
}

impl PreparedTokenPause {
    fn from_args(args: TokenPauseArgs) -> Result<Self> {
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
        })
    }
}

/// Pause a token.
pub(super) async fn pause(conn: &Connection, args: TokenPauseArgs) -> Result<()> {
    submit_pause_change(conn, args, true).await
}

/// Unpause a token.
pub(super) async fn unpause(conn: &Connection, args: TokenPauseArgs) -> Result<()> {
    submit_pause_change(conn, args, false).await
}

async fn submit_pause_change(conn: &Connection, args: TokenPauseArgs, pause: bool) -> Result<()> {
    let prepared = PreparedTokenPause::from_args(args)?;
    let mut context = shared::resolve_prepared_mutation_context(conn, &prepared.context).await?;
    let token_id = prepared
        .token_id
        .resolve_with(prepared.context.input_mode(), shared::prompt_token_id)?
        .into_value();
    let mut token_client = shared::init_token_client(context.client.clone(), token_id).await?;
    let action = if pause { "pause" } else { "unpause" };

    cliclack::log::info(format!(
        "Token {action}\nnetwork: {} ({})\naccount: {}\ntoken: {}",
        context.network_name,
        context.endpoint_label,
        context.wallet.address,
        token_client.token_info().token_id,
    ))?;
    if !shared::confirm_submission(
        &format!("Approve and submit this token {action}? Type y to approve:"),
        &format!("token {action} declined by user"),
    )? {
        return Ok(());
    }

    let spin = spinner();
    spin.start(format!("Submitting token {action}..."));
    let transaction_hash = if pause {
        token_client
            .pause(&context.wallet, Some(TransactionMetadata::default()))
            .await?
    } else {
        token_client
            .unpause(&context.wallet, Some(TransactionMetadata::default()))
            .await?
    };
    spin.clear();
    cliclack::log::success(format!(
        "Submitted token {action} on {} ({}): {transaction_hash}",
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
