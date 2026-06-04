//! Token pause and unpause command implementations.

use crate::{cli::TokenPauseArgs, commands::token::shared};
use anyhow::Result;
use cliclack::spinner;
use concordium_rust_sdk::protocol_level_tokens::token_client::TransactionMetadata;
use rusqlite::Connection;

/// Pause a token.
pub(super) async fn pause(conn: &Connection, args: TokenPauseArgs) -> Result<()> {
    submit_pause_change(conn, args, true).await
}

/// Unpause a token.
pub(super) async fn unpause(conn: &Connection, args: TokenPauseArgs) -> Result<()> {
    submit_pause_change(conn, args, false).await
}

async fn submit_pause_change(conn: &Connection, args: TokenPauseArgs, pause: bool) -> Result<()> {
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
