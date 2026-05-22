use crate::{
    cli::ConnectArgs,
    commands::ui::{SelectItem, select_or_single},
};
use anyhow::{Context, Result, bail};
use ccd_wallet_connect::{
    ConnectServer, PairingApproval, PairingRejection, PairingRequest, SessionContext,
};
use ccd_wallet_core::store::{
    accounts::{self, AccountRecord, AccountSourceKind, AccountStatus},
    config::{NetworkEntry, load},
    seeds,
};
use cliclack::{input, password};
use futures_util::FutureExt;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

pub async fn run(conn: Connection, args: ConnectArgs) -> Result<()> {
    let conn = Arc::new(Mutex::new(conn));
    let server_conn = Arc::clone(&conn);
    let server = ConnectServer::new(move |request| {
        let conn = Arc::clone(&server_conn);
        async move {
            let conn = conn
                .lock()
                .map_err(|_| PairingRejection::new("wallet database connection is unavailable"))?;
            approve_pairing(&conn, request).map_err(|err| PairingRejection::new(err.to_string()))
        }
        .boxed()
    });

    println!(
        "Starting ccd-wallet browser pairing session on ws://{}",
        args.bind
    );
    println!("Press Ctrl-C to stop the connect session.");

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        let _ = shutdown_tx.send(());
    });

    server.serve(args.bind, shutdown_rx).await
}

fn approve_pairing(conn: &Connection, request: PairingRequest) -> Result<PairingApproval> {
    cliclack::log::info(format!(
        "Browser pairing request\norigin: {}\nchallenge: {}",
        request.origin, request.challenge
    ))?;

    let expected_challenge = request.challenge.clone();
    let confirmation: String = input("Type the six-digit browser challenge to approve pairing:")
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

    let (network_name, network_entry) = select_network()?;
    let account = select_account(conn, &network_entry.genesis_hash)?;
    let account_address = read_account_address(conn, &account, &network_name)?;

    cliclack::log::success(format!(
        "Paired {} with account {} on network {}.",
        request.origin, account_address, network_name
    ))?;

    Ok(PairingApproval {
        context: SessionContext {
            network_genesis_hash: network_entry.genesis_hash,
            account_address,
        },
    })
}

fn select_network() -> Result<(String, NetworkEntry)> {
    let app_config = load()?;
    if app_config.networks.is_empty() {
        bail!("no networks are configured; run `ccd-wallet network add` first");
    }
    let entries = app_config.networks.into_iter().collect::<Vec<_>>();
    let items = entries
        .iter()
        .map(|(name, entry)| SelectItem {
            value: name.clone(),
            label: name.clone(),
            hint: entry.node_endpoint.clone(),
        })
        .collect::<Vec<_>>();
    let selected = select_or_single("Select network for browser session", &items, None)?;
    let entry = entries
        .into_iter()
        .find(|(name, _)| name == &selected)
        .map(|(_, entry)| entry)
        .context("selected network was not found")?;
    Ok((selected, entry))
}

fn select_account(conn: &Connection, network_genesis_hash: &str) -> Result<AccountRecord> {
    let accounts = accounts::list(conn)?
        .into_iter()
        .filter(|record| record.network_genesis_hash == network_genesis_hash)
        .filter(|record| record.status == AccountStatus::Finalized)
        .collect::<Vec<_>>();
    if accounts.is_empty() {
        bail!("no finalized accounts are available for the selected network");
    }

    let seed_labels = seeds::list(conn)?
        .into_iter()
        .map(|seed| (seed.id, seed.label))
        .collect::<std::collections::BTreeMap<_, _>>();
    let items = accounts
        .iter()
        .map(|record| SelectItem {
            value: record.id,
            label: render_account_label(record, &seed_labels),
            hint: account_hint(record),
        })
        .collect::<Vec<_>>();
    let selected = select_or_single("Select account for browser session", &items, None)?;
    accounts
        .into_iter()
        .find(|record| record.id == selected)
        .context("selected account was not found")
}

fn render_account_label(
    record: &AccountRecord,
    seed_labels: &std::collections::BTreeMap<String, String>,
) -> String {
    if record.source_kind == AccountSourceKind::Imported {
        format!("[imported] {}", record.label)
    } else {
        let seed_label = seed_labels
            .get(&record.seed_id)
            .map(String::as_str)
            .unwrap_or("<unknown-seed>");
        format!("[{seed_label}] {}", record.label)
    }
}

fn account_hint(record: &AccountRecord) -> String {
    match record.source_kind {
        AccountSourceKind::Imported => "imported account".to_owned(),
        AccountSourceKind::Derived => format!(
            "provider:{} identity:{} credential:{}",
            record.ip_identity, record.identity_index, record.credential_counter
        ),
    }
}

fn read_account_address(
    conn: &Connection,
    account: &AccountRecord,
    network_name: &str,
) -> Result<String> {
    match account.source_kind {
        AccountSourceKind::Derived => {
            let seed = seeds::list(conn)?
                .into_iter()
                .find(|seed| seed.id == account.seed_id)
                .context("selected account references unknown seed")?;
            let password: String =
                password(format!("Password for seed '{}':", seed.label)).interact()?;
            let unlocked = seeds::unlock_context(conn, &seed.label, &password)?;
            let payload = accounts::decrypt_private_payload(conn, account.id, &unlocked.dek)?;
            Ok(payload.account_address)
        }
        AccountSourceKind::Imported => {
            let vault_password: String = password(format!(
                "Imported accounts vault password for '{}':",
                network_name
            ))
            .allow_empty()
            .interact()?;
            let unlocked = accounts::unlock_imported_vault(
                conn,
                &account.network_genesis_hash,
                &vault_password,
            )?;
            let payload = accounts::decrypt_imported_payload(conn, account.id, &unlocked.dek)?;
            Ok(payload.account_address)
        }
    }
}
