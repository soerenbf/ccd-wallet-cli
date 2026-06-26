//! Token allow-list and deny-list command implementations.

use crate::{
    cli::TokenListMutationArgs,
    commands::{
        input::{AccountReference, Promptable},
        token::shared,
    },
};
use anyhow::Result;
use cliclack::spinner;
use concordium_rust_sdk::protocol_level_tokens::{
    TokenId,
    token_client::{TransactionMetadata, Validation},
};
use rusqlite::Connection;

#[derive(Clone, Debug)]
struct PreparedTokenListMutation {
    context: shared::PreparedTokenMutationContext,
    token_id: Promptable<TokenId>,
    targets: Vec<AccountReference>,
}

impl PreparedTokenListMutation {
    fn from_args(args: TokenListMutationArgs) -> Result<Self> {
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
            targets: args.targets,
        })
    }
}

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
    let prepared = PreparedTokenListMutation::from_args(args)?;
    let mut context = shared::resolve_prepared_mutation_context(conn, &prepared.context).await?;
    let token_id = prepared
        .token_id
        .resolve_with(prepared.context.input_mode(), shared::prompt_token_id)?
        .into_value();
    let target_inputs = prepared
        .targets
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let targets = shared::resolve_target_addresses(
        conn,
        &mut context,
        &target_inputs,
        !prepared.context.input_mode().prompts_allowed(),
    )?;
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
        &format!("Approve and submit this token {list_name} update?"),
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
