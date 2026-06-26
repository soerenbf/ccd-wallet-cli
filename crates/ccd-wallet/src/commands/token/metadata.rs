//! Token metadata command implementations.

use crate::{
    cli::TokenMetadataUpdateArgs,
    commands::{input::Promptable, token::shared},
};
use anyhow::Result;
use cliclack::spinner;
use concordium_rust_sdk::protocol_level_tokens::{TokenId, token_client::TransactionMetadata};
use rusqlite::Connection;

#[derive(Clone, Debug)]
struct PreparedTokenMetadataUpdate {
    context: shared::PreparedTokenMutationContext,
    token_id: Promptable<TokenId>,
    url: Promptable<String>,
    checksum_sha_256: Option<String>,
}

impl PreparedTokenMetadataUpdate {
    fn from_args(args: TokenMetadataUpdateArgs) -> Result<Self> {
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
            url: Promptable::from_option(args.url, "url"),
            checksum_sha_256: args.checksum_sha_256,
        })
    }
}

/// Update token metadata.
pub(super) async fn update(conn: &Connection, args: TokenMetadataUpdateArgs) -> Result<()> {
    let prepared = PreparedTokenMetadataUpdate::from_args(args)?;
    let mut context = shared::resolve_prepared_mutation_context(conn, &prepared.context).await?;
    let token_id = prepared
        .token_id
        .resolve_with(prepared.context.input_mode(), shared::prompt_token_id)?
        .into_value();
    let url = prepared
        .url
        .clone()
        .resolve_with(prepared.context.input_mode(), || {
            shared::prompt_required_string("Metadata URL:")
        })?
        .into_value();
    let metadata = shared::build_metadata_url(url, prepared.checksum_sha_256.as_deref())?;
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
        "Approve and submit this token metadata update?",
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
