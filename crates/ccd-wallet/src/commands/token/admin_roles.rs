//! Token admin-role command implementations.

use crate::{cli::TokenAdminRolesArgs, commands::token::shared};
use anyhow::Result;
use cliclack::spinner;
use concordium_rust_sdk::protocol_level_tokens::token_client::TransactionMetadata;
use rusqlite::Connection;

/// Assign token admin roles.
pub(super) async fn assign(conn: &Connection, args: TokenAdminRolesArgs) -> Result<()> {
    submit_admin_role_update(conn, args, true).await
}

/// Revoke token admin roles.
pub(super) async fn revoke(conn: &Connection, args: TokenAdminRolesArgs) -> Result<()> {
    submit_admin_role_update(conn, args, false).await
}

async fn submit_admin_role_update(
    conn: &Connection,
    args: TokenAdminRolesArgs,
    assign: bool,
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
    let target = shared::resolve_account_address(
        conn,
        &mut context,
        args.target.as_deref(),
        "Target account address or local label:",
        "target",
        args.non_interactive,
    )?;
    let roles = shared::resolve_admin_roles(&args.roles, args.non_interactive)?;
    let mut token_client = shared::init_token_client(context.client.clone(), token_id).await?;
    let action = if assign { "assign" } else { "revoke" };

    cliclack::log::info(format!(
        "Token admin-roles {action}\nnetwork: {} ({})\naccount: {}\ntoken: {}\ntarget: {}\nroles: {}",
        context.network_name,
        context.endpoint_label,
        context.wallet.address,
        token_client.token_info().token_id,
        target,
        args.roles.join(", "),
    ))?;
    if !shared::confirm_submission(
        &format!("Approve and submit this token admin-roles {action}? Type y to approve:"),
        &format!("token admin-roles {action} declined by user"),
    )? {
        return Ok(());
    }

    let spin = spinner();
    spin.start(format!("Submitting token admin-roles {action}..."));
    let transaction_hash = if assign {
        token_client
            .assign_admin_roles(
                &context.wallet,
                Some(TransactionMetadata::default()),
                target,
                roles,
            )
            .await?
    } else {
        token_client
            .revoke_admin_roles(
                &context.wallet,
                Some(TransactionMetadata::default()),
                target,
                roles,
            )
            .await?
    };
    spin.clear();
    cliclack::log::success(format!(
        "Submitted token admin-roles {action} on {} ({}): {transaction_hash}",
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
