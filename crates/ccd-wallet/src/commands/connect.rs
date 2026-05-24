use crate::{
    cli::ConnectArgs,
    commands::ui::{SelectItem, select_or_single},
};
use anyhow::{Context, Result, bail};
use ccd_wallet_connect::{
    AccountRequest, AccountRequestApproval, ConnectServer, PairingApproval, PairingRejection,
    PairingRequest,
};
use ccd_wallet_core::store::{
    accounts::{self, AccountRecord, AccountSourceKind, AccountStatus},
    config::{self, load},
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
    let account_conn = Arc::clone(&conn);
    let server = ConnectServer::new(
        move |request| {
            let conn = Arc::clone(&server_conn);
            async move {
                let conn = conn.lock().map_err(|_| {
                    PairingRejection::new("wallet database connection is unavailable")
                })?;
                match approve_pairing(&conn, request) {
                    Ok(approval) => Ok(approval),
                    Err(err) => {
                        let message = err.to_string();
                        let _ = cliclack::log::error(&message);
                        Err(PairingRejection::new(message))
                    }
                }
            }
            .boxed()
        },
        move |request| {
            let conn = Arc::clone(&account_conn);
            async move {
                let conn = conn.lock().map_err(|_| {
                    PairingRejection::new("wallet database connection is unavailable")
                })?;
                match approve_account_request(&conn, request) {
                    Ok(approval) => Ok(approval),
                    Err(err) => {
                        let message = err.to_string();
                        let _ = cliclack::log::error(&message);
                        Err(PairingRejection::new(message))
                    }
                }
            }
            .boxed()
        },
    );

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

fn approve_pairing(_conn: &Connection, request: PairingRequest) -> Result<PairingApproval> {
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

    cliclack::log::success(format!("Paired {}.", request.origin))?;

    Ok(PairingApproval)
}

fn approve_account_request(
    conn: &Connection,
    request: AccountRequest,
) -> Result<AccountRequestApproval> {
    let network_name = resolve_network_display_name(&request.network_genesis_hash)?;
    let account = select_account(conn, &request.network_genesis_hash)?;
    let account_address = read_account_address(conn, &account, &network_name)?;

    cliclack::log::success(format!(
        "Approved account {} for network {}.",
        account_address, network_name
    ))?;

    Ok(AccountRequestApproval { account_address })
}

fn resolve_network_display_name(network_genesis_hash: &str) -> Result<String> {
    let config = load()?;
    let matches = config::aliases_by_genesis_hash(&config, network_genesis_hash);
    if matches.is_empty() {
        Ok(network_genesis_hash.to_owned())
    } else {
        Ok(matches.join(", "))
    }
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
            let password: String = password(format!("Password for seed '{}':", seed.label))
                .allow_empty()
                .interact()?;
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
