//! Pairing approval flow for browser connect sessions.

use super::shared;
use anyhow::{Result, bail};
use ccd_wallet_connect::{PairingApproval, PairingRequest};
use cliclack::input;
use rusqlite::Connection;

pub(super) fn approve_pairing(
    _conn: &Connection,
    request: PairingRequest,
) -> Result<PairingApproval> {
    cliclack::log::info(format!(
        "Browser pairing request\norigin: {}",
        request.origin
    ))?;

    let expected_challenge = request.challenge.clone();
    let confirmation: String =
        input("Enter the six-digit challenge shown in the web application to approve pairing:")
            .validate(move |value: &String| {
                if value == &expected_challenge {
                    Ok(())
                } else {
                    Err("Challenge does not match.")
                }
            })
            .interact()?;
    if confirmation != request.challenge {
        bail!("pairing rejected because challenge confirmation did not match");
    }

    let (network_name, network_entry) = shared::select_network()?;

    cliclack::log::success(format!(
        "Paired {} on network {}.",
        request.origin, network_name
    ))?;

    Ok(PairingApproval {
        network_genesis_hash: network_entry.genesis_hash,
    })
}
