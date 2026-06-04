//! Token metadata command implementations.

use crate::{cli::TokenMetadataUpdateArgs, commands::token::shared};
use anyhow::Result;
use cliclack::spinner;
use concordium_rust_sdk::protocol_level_tokens::token_client::TransactionMetadata;
use rusqlite::Connection;

/// Update token metadata.
pub(super) async fn update(conn: &Connection, args: TokenMetadataUpdateArgs) -> Result<()> {
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
    let url = shared::resolve_required_string(
        args.url.as_deref(),
        "Metadata URL:",
        "url",
        args.non_interactive,
    )?;
    let metadata = shared::build_metadata_url(url, args.checksum_sha_256.as_deref())?;
    let mut token_client = shared::init_token_client(context.client.clone(), token_id).await?;

    cliclack::log::info(format!(
        "Token metadata update\nnetwork: {} ({})\naccount: {}\ntoken: {}\nurl: {}\nchecksum: {}",
        context.network_name,
        context.endpoint_label,
        context.wallet.address,
        token_client.token_info().token_id,
        metadata.url,
        metadata
            .checksum_sha_256
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "none".to_owned()),
    ))?;
    if !shared::confirm_submission(
        "Approve and submit this token metadata update? Type y to approve:",
        "token metadata update declined by user",
    )? {
        return Ok(());
    }

    let spin = spinner();
    spin.start("Submitting token metadata update...");
    let transaction_hash = token_client
        .update_metadata(
            &context.wallet,
            Some(TransactionMetadata::default()),
            metadata,
        )
        .await?;
    spin.clear();
    cliclack::log::success(format!(
        "Submitted token metadata update on {} ({}): {transaction_hash}",
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
