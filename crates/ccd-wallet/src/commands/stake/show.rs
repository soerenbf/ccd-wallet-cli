//! Stake inspection command.

use crate::{
    cli::StakeShowArgs,
    commands::stake::shared::{
        query_account_info, render_stake_details_lines, resolve_query_context,
        resolve_stake_query_address, stake_details_view_from_account_info,
    },
    smart_contracts::shared::parse_block_identifier,
};
use anyhow::Result;
use rusqlite::Connection;

/// Run `stake show`.
///
/// # Arguments
/// * `conn` - Open wallet store connection.
/// * `args` - Parsed command arguments.
///
/// # Errors
/// Returns an error if account resolution or chain querying fails.
pub(super) async fn show(conn: &Connection, args: StakeShowArgs) -> Result<()> {
    let mut context = resolve_query_context(
        conn,
        Some(&args.account),
        args.network.as_deref(),
        args.node,
        args.no_defaults,
    )
    .await?;
    let block = parse_block_identifier(args.block.as_deref())?;
    let (address, local_label) = match context.selected_account.as_ref() {
        Some((record, _source)) => (
            crate::commands::account::decrypt_local_account_address(
                conn,
                &context.network_name,
                record,
            )?,
            Some(record.label.clone()),
        ),
        None => resolve_stake_query_address(
            conn,
            &context.network_name,
            &context.network_genesis_hash,
            &args.account,
        )?,
    };
    let info = query_account_info(&mut context.client, address, block).await?;
    let details = stake_details_view_from_account_info(&info);

    let mut lines = vec![match local_label {
        Some(label) => format!(
            "[{label}] {} @ {}",
            info.account_address, context.network_name
        ),
        None => format!("{} @ {}", info.account_address, context.network_name),
    }];
    lines.push(String::new());
    lines.extend(render_stake_details_lines(details.as_ref()));
    println!("{}", lines.join("\n"));
    Ok(())
}
