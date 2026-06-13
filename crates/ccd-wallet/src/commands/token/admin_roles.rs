//! Token admin-role command implementations.

use crate::{
    cli::TokenAdminRolesArgs,
    commands::{
        input::{AccountReference, Promptable},
        token::shared,
    },
};
use anyhow::Result;
use cliclack::spinner;
use concordium_rust_sdk::protocol_level_tokens::{TokenId, token_client::TransactionMetadata};
use rusqlite::Connection;

#[derive(Clone, Debug)]
struct PreparedTokenAdminRoles {
    context: shared::PreparedTokenMutationContext,
    token_id: Promptable<TokenId>,
    target: Promptable<AccountReference>,
    roles: Vec<String>,
}

impl PreparedTokenAdminRoles {
    fn from_args(args: TokenAdminRolesArgs) -> Result<Self> {
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
            target: Promptable::from_option(args.target, "target"),
            roles: args.roles,
        })
    }
}

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
    let prepared = PreparedTokenAdminRoles::from_args(args)?;
    let mut context = shared::resolve_prepared_mutation_context(conn, &prepared.context).await?;
    let token_id = prepared
        .token_id
        .resolve_with(prepared.context.input_mode(), shared::prompt_token_id)?
        .into_value();
    let target_input = match &prepared.target {
        Promptable::Provided(target) => Some(target.to_string()),
        Promptable::Missing { .. } => None,
    };
    let target = shared::resolve_account_address(
        conn,
        &mut context,
        target_input.as_deref(),
        "Target account address or local label:",
        "target",
        !prepared.context.input_mode().prompts_allowed(),
    )?;
    let roles = shared::resolve_admin_roles(
        &prepared.roles,
        !prepared.context.input_mode().prompts_allowed(),
    )?;
    let mut token_client = shared::init_token_client(context.client.clone(), token_id).await?;
    let action = if assign { "assign" } else { "revoke" };

    cliclack::log::info(format!(
        "Token admin-roles {action}\nnetwork: {} ({})\naccount: {}\ntoken: {}\ntarget: {}\nroles: {}",
        context.network_name,
        context.endpoint_label,
        context.wallet.address,
        token_client.token_info().token_id,
        target,
        prepared.roles.join(", "),
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
