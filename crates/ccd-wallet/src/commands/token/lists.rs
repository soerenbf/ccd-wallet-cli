//! Token allow-list and deny-list command implementations.

use crate::{cli::TokenListMutationArgs, commands::token::shared};
use anyhow::Result;
use cliclack::spinner;
use concordium_rust_sdk::protocol_level_tokens::token_client::{TransactionMetadata, Validation};
use rusqlite::Connection;

/// Add accounts to a token allow list.
pub(super) async fn allow_list_add(conn: &Connection, args: TokenListMutationArgs) -> Result<()> {
    submit_list_update(conn, args, "allow-list", true).await
}

/// Remove accounts from a token allow list.
pub(super) async fn allow_list_remove(
    conn: &Connection,
    args: TokenListMutationArgs,
) -> Result<()> {
    submit_list_update(conn, args, "allow-list", false).await
}

/// Add accounts to a token deny list.
pub(super) async fn deny_list_add(conn: &Connection, args: TokenListMutationArgs) -> Result<()> {
    submit_list_update(conn, args, "deny-list", true).await
}

/// Remove accounts from a token deny list.
pub(super) async fn deny_list_remove(conn: &Connection, args: TokenListMutationArgs) -> Result<()> {
    submit_list_update(conn, args, "deny-list", false).await
}

async fn submit_list_update(
    conn: &Connection,
    args: TokenListMutationArgs,
    list_name: &str,
    add: bool,
) -> Result<()> {
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
    let targets = shared::resolve_target_addresses(&args.targets, args.non_interactive)?;
    let mut token_client = shared::init_token_client(context.client.clone(), token_id).await?;
    let action = if add { "add" } else { "remove" };
    let target_summary = targets
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");

    cliclack::log::info(format!(
        "Token {list_name} {action}\nnetwork: {} ({})\naccount: {}\ntoken: {}\ntargets: {}",
        context.network_name,
        context.endpoint_label,
        context.wallet.address,
        token_client.token_info().token_id,
        target_summary,
    ))?;
    if !shared::confirm_submission(
        &format!("Approve and submit this token {list_name} update? Type y to approve:"),
        &format!("token {list_name} update declined by user"),
    )? {
        return Ok(());
    }

    let spin = spinner();
    spin.start(format!("Submitting token {list_name} update..."));
    let transaction_hash = match (list_name, add) {
        ("allow-list", true) => {
            token_client
                .add_allow_list(
                    &context.wallet,
                    targets,
                    Some(TransactionMetadata::default()),
                    Validation::Validate,
                )
                .await?
        }
        ("allow-list", false) => {
            token_client
                .remove_allow_list(
                    &context.wallet,
                    targets,
                    Some(TransactionMetadata::default()),
                    Validation::Validate,
                )
                .await?
        }
        ("deny-list", true) => {
            token_client
                .add_deny_list(
                    &context.wallet,
                    targets,
                    Some(TransactionMetadata::default()),
                    Validation::Validate,
                )
                .await?
        }
        ("deny-list", false) => {
            token_client
                .remove_deny_list(
                    &context.wallet,
                    targets,
                    Some(TransactionMetadata::default()),
                    Validation::Validate,
                )
                .await?
        }
        _ => unreachable!("unsupported token list update"),
    };
    spin.clear();
    cliclack::log::success(format!(
        "Submitted token {list_name} update on {} ({}): {transaction_hash}",
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
