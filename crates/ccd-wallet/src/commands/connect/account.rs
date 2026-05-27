//! Account-authority approval flow for browser connect sessions.

use super::shared;
use anyhow::Result;
use ccd_wallet_connect::{AccountRequest, AccountRequestApproval};
use rusqlite::Connection;

pub(super) fn approve_account_request(
    conn: &Connection,
    request: AccountRequest,
) -> Result<AccountRequestApproval> {
    let network_name = shared::resolve_network_display_name(&request.network_genesis_hash)?;
    cliclack::log::info(format!(
        "Account authority request\norigin: {}\nnetwork: {}",
        request.origin, network_name
    ))?;

    let account = shared::select_account(conn, &request.network_genesis_hash)?;
    let account_address = shared::read_account_address(conn, &account, &network_name)?;

    cliclack::log::success(format!(
        "Approved account authority {} for {} on network {}.",
        account_address, request.origin, network_name
    ))?;

    Ok(AccountRequestApproval { account_address })
}
