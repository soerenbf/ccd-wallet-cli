//! Read-only protocol-level token inspection command.

use crate::{cli::TokenShowArgs, commands::token::shared};
use anyhow::Result;
use rusqlite::Connection;

/// Show protocol-level token information.
pub(super) async fn show(conn: &Connection, args: TokenShowArgs) -> Result<()> {
    let (_network_name, _endpoint_label, mut client) =
        shared::resolve_query_client(conn, args.network.as_deref(), args.node, args.no_defaults)
            .await?;
    let info = shared::query_token_info(&mut client, args.token_id).await?;
    println!("{}", shared::render_token_info(&info)?);
    Ok(())
}
