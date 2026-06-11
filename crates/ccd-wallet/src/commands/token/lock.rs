//! Protocol-level lock command implementations.

use crate::{
    cli::{
        TokenLockCancelArgs, TokenLockCreateArgs, TokenLockFundArgs, TokenLockReturnArgs,
        TokenLockSendArgs, TokenLockShowArgs,
    },
    commands::token::shared,
};
use anyhow::Result;
use cliclack::spinner;
use concordium_rust_sdk::protocol_level_tokens::lock_client::{
    self, FundTokens, ReturnTokens, SendTokens, Validation,
};
use rusqlite::Connection;

/// Create a protocol-level lock.
pub(super) async fn create(conn: &Connection, args: TokenLockCreateArgs) -> Result<()> {
    let mut context = shared::resolve_mutation_context(
        conn,
        args.account.as_deref(),
        args.network.as_deref(),
        args.node,
        args.non_interactive,
        args.no_defaults,
        false,
    )
    .await?;
    let recipients =
        shared::parse_account_addresses(conn, &mut context, &args.recipients, "recipient")?;
    let expiry = shared::parse_expiry_time(&args.expiry)?;
    let unresolved_grants = if args.grants.is_empty() {
        if args.non_interactive {
            anyhow::bail!("at least one --grant must be provided in --non-interactive mode");
        }
        shared::prompt_unresolved_lock_grants()?
    } else {
        args.grants
            .iter()
            .map(String::as_str)
            .map(shared::parse_unresolved_lock_grant)
            .collect::<Result<Vec<_>>>()?
    };
    let grants = unresolved_grants
        .iter()
        .map(|grant| shared::resolve_lock_grant(conn, &mut context, grant))
        .collect::<Result<Vec<_>>>()?;
    let token_summary = args
        .tokens
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let recipient_summary = recipients
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let config =
        shared::build_lock_config(recipients, expiry, grants, args.tokens, args.keep_alive);

    cliclack::log::info(format!(
        "Token lock create\nnetwork: {} ({})\naccount: {}\nrecipients: {}\nexpiry: {}\ntokens: {}\nkeep alive: {}",
        context.network_name,
        context.endpoint_label,
        context.wallet.address,
        recipient_summary,
        args.expiry,
        token_summary,
        if args.keep_alive { "yes" } else { "no" },
    ))?;
    if !shared::confirm_submission(
        "Approve and submit this token lock creation? Type y to approve:",
        "token lock creation declined by user",
    )? {
        return Ok(());
    }

    let spin = spinner();
    spin.start("Submitting token lock creation...");
    let pending = lock_client::create_lock(
        context.client.clone(),
        &context.wallet,
        config,
        Some(lock_client::TransactionMetadata::default()),
    )
    .await?;
    spin.clear();
    let transaction_hash = pending.transaction_hash();
    cliclack::log::success(format!(
        "Submitted token lock creation on {} ({}): {transaction_hash}",
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

/// Fund an existing protocol-level lock.
pub(super) async fn fund(conn: &Connection, args: TokenLockFundArgs) -> Result<()> {
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
    let lock_id = shared::resolve_lock_id(args.lock_id, args.non_interactive)?;
    let mut lock_client =
        lock_client::LockClient::from_lock_id(context.client.clone(), lock_id).await?;
    let mut balance_client = context.client.clone();
    let available_balances =
        shared::account_available_balances(&mut balance_client, context.wallet.address).await?;
    let token_id = shared::resolve_lock_token(
        args.token_id,
        lock_client.lock_info(),
        &available_balances,
        args.non_interactive,
    )?;
    let mut query_client = context.client.clone();
    let token_info = shared::query_token_info(&mut query_client, token_id.clone()).await?;
    let amount = shared::resolve_token_amount(
        args.amount.as_deref(),
        token_info.token_state.decimals,
        available_balances.get(&token_id).copied(),
        "available",
        args.non_interactive,
    )?;
    let payload = FundTokens {
        token_id,
        amount,
        memo: None,
    };

    cliclack::log::info(format!(
        "Token lock fund\nnetwork: {} ({})\naccount: {}\nlock: {}\ntoken: {}\namount: {}",
        context.network_name,
        context.endpoint_label,
        context.wallet.address,
        lock_client.lock_info().lock,
        payload.token_id,
        payload.amount,
    ))?;
    if !shared::confirm_submission(
        "Approve and submit this token lock funding? Type y to approve:",
        "token lock funding declined by user",
    )? {
        return Ok(());
    }

    let spin = spinner();
    spin.start("Submitting token lock funding...");
    let transaction_hash = lock_client
        .fund(
            &context.wallet,
            payload,
            Some(lock_client::TransactionMetadata::default()),
            Validation::Validate,
        )
        .await?;
    spin.clear();
    cliclack::log::success(format!(
        "Submitted token lock funding on {} ({}): {transaction_hash}",
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

/// Send locked funds to a configured recipient.
pub(super) async fn send(conn: &Connection, args: TokenLockSendArgs) -> Result<()> {
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
    let lock_id = shared::resolve_lock_id(args.lock_id, args.non_interactive)?;
    let mut lock_client =
        lock_client::LockClient::from_lock_id(context.client.clone(), lock_id).await?;
    let source = shared::resolve_account_address(
        conn,
        &mut context,
        args.source.as_deref(),
        "Source account address or local label:",
        "source",
        args.non_interactive,
    )?;
    let locked_balances = shared::locked_balances_for_source(lock_client.lock_info(), source);
    let token_id = shared::resolve_lock_token(
        args.token_id,
        lock_client.lock_info(),
        &locked_balances,
        args.non_interactive,
    )?;
    let mut query_client = context.client.clone();
    let token_info = shared::query_token_info(&mut query_client, token_id.clone()).await?;
    let payload = SendTokens {
        token_id: token_id.clone(),
        source,
        recipient: shared::resolve_account_address(
            conn,
            &mut context,
            args.recipient.as_deref(),
            "Recipient account address or local label:",
            "recipient",
            args.non_interactive,
        )?,
        amount: shared::resolve_token_amount(
            args.amount.as_deref(),
            token_info.token_state.decimals,
            locked_balances.get(&token_id).copied(),
            "locked",
            args.non_interactive,
        )?,
        memo: None,
    };

    cliclack::log::info(format!(
        "Token lock send\nnetwork: {} ({})\naccount: {}\nlock: {}\ntoken: {}\nsource: {}\nrecipient: {}\namount: {}",
        context.network_name,
        context.endpoint_label,
        context.wallet.address,
        lock_client.lock_info().lock,
        payload.token_id,
        payload.source,
        payload.recipient,
        payload.amount,
    ))?;
    if !shared::confirm_submission(
        "Approve and submit this token lock send? Type y to approve:",
        "token lock send declined by user",
    )? {
        return Ok(());
    }

    let spin = spinner();
    spin.start("Submitting token lock send...");
    let transaction_hash = lock_client
        .send(
            &context.wallet,
            payload,
            Some(lock_client::TransactionMetadata::default()),
            Validation::Validate,
        )
        .await?;
    spin.clear();
    cliclack::log::success(format!(
        "Submitted token lock send on {} ({}): {transaction_hash}",
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

/// Return locked funds to the source account.
pub(super) async fn return_funds(conn: &Connection, args: TokenLockReturnArgs) -> Result<()> {
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
    let lock_id = shared::resolve_lock_id(args.lock_id, args.non_interactive)?;
    let mut lock_client =
        lock_client::LockClient::from_lock_id(context.client.clone(), lock_id).await?;
    let source = shared::resolve_account_address(
        conn,
        &mut context,
        args.source.as_deref(),
        "Source account address or local label:",
        "source",
        args.non_interactive,
    )?;
    let locked_balances = shared::locked_balances_for_source(lock_client.lock_info(), source);
    let token_id = shared::resolve_lock_token(
        args.token_id,
        lock_client.lock_info(),
        &locked_balances,
        args.non_interactive,
    )?;
    let mut query_client = context.client.clone();
    let token_info = shared::query_token_info(&mut query_client, token_id.clone()).await?;
    let payload = ReturnTokens {
        token_id: token_id.clone(),
        source,
        amount: shared::resolve_token_amount(
            args.amount.as_deref(),
            token_info.token_state.decimals,
            locked_balances.get(&token_id).copied(),
            "locked",
            args.non_interactive,
        )?,
        memo: None,
    };

    cliclack::log::info(format!(
        "Token lock return\nnetwork: {} ({})\naccount: {}\nlock: {}\ntoken: {}\nsource: {}\namount: {}",
        context.network_name,
        context.endpoint_label,
        context.wallet.address,
        lock_client.lock_info().lock,
        payload.token_id,
        payload.source,
        payload.amount,
    ))?;
    if !shared::confirm_submission(
        "Approve and submit this token lock return? Type y to approve:",
        "token lock return declined by user",
    )? {
        return Ok(());
    }

    let spin = spinner();
    spin.start("Submitting token lock return...");
    let transaction_hash = lock_client
        .return_funds(
            &context.wallet,
            payload,
            Some(lock_client::TransactionMetadata::default()),
            Validation::Validate,
        )
        .await?;
    spin.clear();
    cliclack::log::success(format!(
        "Submitted token lock return on {} ({}): {transaction_hash}",
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

/// Cancel an existing lock.
pub(super) async fn cancel(conn: &Connection, args: TokenLockCancelArgs) -> Result<()> {
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
    let lock_id = shared::resolve_lock_id(args.lock_id, args.non_interactive)?;
    let mut lock_client =
        lock_client::LockClient::from_lock_id(context.client.clone(), lock_id).await?;

    cliclack::log::info(format!(
        "Token lock cancel\nnetwork: {} ({})\naccount: {}\nlock: {}",
        context.network_name,
        context.endpoint_label,
        context.wallet.address,
        lock_client.lock_info().lock,
    ))?;
    if !shared::confirm_submission(
        "Approve and submit this token lock cancellation? Type y to approve:",
        "token lock cancellation declined by user",
    )? {
        return Ok(());
    }

    let spin = spinner();
    spin.start("Submitting token lock cancellation...");
    let transaction_hash = lock_client
        .cancel(
            &context.wallet,
            None,
            Some(lock_client::TransactionMetadata::default()),
            Validation::Validate,
        )
        .await?;
    spin.clear();
    cliclack::log::success(format!(
        "Submitted token lock cancellation on {} ({}): {transaction_hash}",
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

/// Show protocol-level lock information.
pub(super) async fn show(conn: &Connection, args: TokenLockShowArgs) -> Result<()> {
    let (_network_name, _endpoint_label, mut client) =
        shared::resolve_query_client(conn, args.network.as_deref(), args.node, args.no_defaults)
            .await?;
    let info = shared::query_lock_info(&mut client, args.lock_id).await?;
    println!("{}", shared::render_lock_info(&info)?);
    Ok(())
}
