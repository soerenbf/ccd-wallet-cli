//! Ledger hardware-wallet setup commands.
//!
//! This module owns user-facing Ledger enrollment flows. The wallet stores the
//! enrolled Ledger as a Ledger-backed key source while keeping the internal
//! persisted model in signer-owner tables.

use crate::cli::{LedgerSetupArgs, LedgerSubcommand};
use anyhow::{Context, Result, bail};
use ccd_wallet_core::store::signer_owners::{
    self, LEDGER_OWNER_ENROLLMENT_PATH, NewLedgerOwnerDetails, SignerOwnerKind,
};
use ccd_wallet_ledger::{
    ConcordiumLedgerApp, DerivationPath, HidTransport, LedgerError, LedgerTransport,
    PublicKeyOptions,
};
use cliclack::{input, password};
use rusqlite::Connection;
use std::str::FromStr;

/// Run a Ledger command.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `command` - Parsed Ledger subcommand.
///
/// # Errors
/// Returns an error if command execution, Ledger communication, or database
/// writes fail.
///
/// # Examples
///
/// ```ignore
/// commands::ledger::run(&conn, command).await?;
/// ```
pub async fn run(conn: &Connection, command: LedgerSubcommand) -> Result<()> {
    match command {
        LedgerSubcommand::Setup(args) => setup(conn, args).await,
        LedgerSubcommand::Show => show().await,
    }
}

async fn show() -> Result<()> {
    let transport = HidTransport::open_first()
        .context("failed to open Ledger device; connect a Ledger with the Concordium app open")?;
    let mut app = ConcordiumLedgerApp::new(transport);
    let app_name = read_app_name(&mut app)?.unwrap_or_else(|| "<unknown>".to_owned());
    let version = read_app_version(&mut app)?;

    println!("Ledger app");
    println!("  name:    {app_name}");
    match version {
        Some(version) => println!("  version: {version}"),
        None => println!("  version: unavailable (requires Concordium Ledger app 5.4.1 or newer)"),
    }
    Ok(())
}

async fn setup(conn: &Connection, args: LedgerSetupArgs) -> Result<()> {
    let label = resolve_label(args.label, args.non_interactive)?;
    validate_key_source_label(&label)?;
    if signer_owners::find_by_label(conn, &label)?.is_some() {
        bail!("key source label '{label}' already exists");
    }

    let transport = HidTransport::open_first()
        .context("failed to open Ledger device; connect a Ledger with the Concordium app open")?;
    let mut app = ConcordiumLedgerApp::new(transport);
    let app_name = read_app_name(&mut app)?;
    let canonical_public_key = read_canonical_public_key(&mut app, true)?;
    if signer_owners::find_ledger_details_by_canonical_public_key(conn, &canonical_public_key)?
        .is_some()
    {
        let fingerprint = signer_owners::ledger_owner_fingerprint(&canonical_public_key);
        bail!("Ledger key source with fingerprint {fingerprint} is already enrolled");
    }

    let local_password = password("Local password for Ledger key source:")
        .allow_empty()
        .interact()?;
    let confirmation = password("Confirm local password:")
        .allow_empty()
        .interact()?;
    if local_password != confirmation {
        bail!("passwords do not match");
    }

    let owner = signer_owners::create(conn, SignerOwnerKind::Ledger, &label)?;
    signer_owners::create_vault(conn, &owner.id, &local_password)?;
    let fingerprint = signer_owners::ledger_owner_fingerprint(&canonical_public_key);
    signer_owners::insert_ledger_details(
        conn,
        NewLedgerOwnerDetails {
            signer_owner_id: &owner.id,
            canonical_public_key: &canonical_public_key,
            fingerprint: &fingerprint,
            enrollment_path: LEDGER_OWNER_ENROLLMENT_PATH,
            app_name: app_name.as_deref(),
        },
    )?;

    println!("Ledger key source '{label}' enrolled with fingerprint {fingerprint}.");
    Ok(())
}

fn resolve_label(label: Option<String>, non_interactive: bool) -> Result<String> {
    match label {
        Some(label) => Ok(label),
        None if non_interactive => {
            bail!("Ledger key-source label must be provided in --non-interactive mode")
        }
        None => input("Ledger key-source label:")
            .validate(|value: &String| match validate_key_source_label(value) {
                Ok(()) => Ok(()),
                Err(err) => Err(err.to_string()),
            })
            .interact()
            .context("failed to read Ledger key-source label"),
    }
}

fn validate_key_source_label(label: &str) -> Result<()> {
    if label.is_empty() {
        bail!("key-source label must not be empty");
    }
    if !label
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        bail!("key-source label must contain only ASCII letters, digits, '-' or '_'");
    }
    Ok(())
}

pub(crate) fn verify_connected_ledger_owner<T: LedgerTransport>(
    conn: &Connection,
    signer_owner_id: &str,
    app: &mut ConcordiumLedgerApp<T>,
) -> Result<()> {
    let details = signer_owners::find_ledger_details_by_owner_id(conn, signer_owner_id)?
        .with_context(|| {
            format!("signer owner '{signer_owner_id}' is not an enrolled Ledger key source")
        })?;
    let connected_key = read_canonical_public_key(app, false)?;
    if connected_key != details.canonical_public_key {
        bail!(
            "connected Ledger does not match key source '{}' (expected fingerprint {})",
            details.signer_owner_id,
            details.fingerprint
        );
    }
    Ok(())
}

fn read_app_name<T: LedgerTransport>(app: &mut ConcordiumLedgerApp<T>) -> Result<Option<String>> {
    match app.get_app_name() {
        Ok(bytes) => Ok(String::from_utf8(bytes).ok()),
        Err(_) => Ok(None),
    }
}

fn read_app_version<T: LedgerTransport>(
    app: &mut ConcordiumLedgerApp<T>,
) -> Result<Option<ccd_wallet_ledger::AppVersion>> {
    match app.get_app_version() {
        Ok(version) => Ok(Some(version)),
        Err(LedgerError::Status { status: 0x6D00, .. }) => Ok(None),
        Err(err) => Err(err).context("failed to read Concordium Ledger app version"),
    }
}

fn read_canonical_public_key<T: LedgerTransport>(
    app: &mut ConcordiumLedgerApp<T>,
    confirm_on_device: bool,
) -> Result<Vec<u8>> {
    let path = DerivationPath::from_str(LEDGER_OWNER_ENROLLMENT_PATH)
        .context("invalid Ledger enrollment derivation path")?;
    let response = app
        .get_public_key(
            path,
            PublicKeyOptions {
                confirm_on_device,
                signed_key: false,
            },
        )
        .context("failed to read canonical public key from Ledger")?;
    Ok(response.public_key.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccd_wallet_core::store::migrations;
    use ccd_wallet_ledger::MockTransport;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    fn public_key_reply(byte: u8) -> Vec<u8> {
        let mut reply = vec![byte; 32];
        reply.extend_from_slice(&[0x90, 0x00]);
        reply
    }

    #[test]
    fn validates_key_source_labels() {
        validate_key_source_label("ledger-main_1").unwrap();
        assert!(validate_key_source_label("").is_err());
        assert!(validate_key_source_label("bad label").is_err());
        assert!(validate_key_source_label("bad.label").is_err());
    }

    #[test]
    fn verify_connected_ledger_owner_accepts_matching_key() {
        let conn = conn();
        let owner = signer_owners::create(&conn, SignerOwnerKind::Ledger, "ledger").unwrap();
        signer_owners::create_vault(&conn, &owner.id, "password").unwrap();
        let key = [7u8; 32];
        signer_owners::insert_ledger_details(
            &conn,
            NewLedgerOwnerDetails {
                signer_owner_id: &owner.id,
                canonical_public_key: &key,
                fingerprint: &signer_owners::ledger_owner_fingerprint(&key),
                enrollment_path: LEDGER_OWNER_ENROLLMENT_PATH,
                app_name: Some("Concordium"),
            },
        )
        .unwrap();

        let transport = MockTransport::new([public_key_reply(7)]);
        let mut app = ConcordiumLedgerApp::new(transport);
        verify_connected_ledger_owner(&conn, &owner.id, &mut app).unwrap();

        let commands = app.transport().commands();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].p1, 0x01);
    }

    #[test]
    fn verify_connected_ledger_owner_rejects_mismatch() {
        let conn = conn();
        let owner = signer_owners::create(&conn, SignerOwnerKind::Ledger, "ledger").unwrap();
        signer_owners::create_vault(&conn, &owner.id, "password").unwrap();
        let key = [7u8; 32];
        signer_owners::insert_ledger_details(
            &conn,
            NewLedgerOwnerDetails {
                signer_owner_id: &owner.id,
                canonical_public_key: &key,
                fingerprint: &signer_owners::ledger_owner_fingerprint(&key),
                enrollment_path: LEDGER_OWNER_ENROLLMENT_PATH,
                app_name: Some("Concordium"),
            },
        )
        .unwrap();

        let transport = MockTransport::new([public_key_reply(8)]);
        let mut app = ConcordiumLedgerApp::new(transport);
        let err = verify_connected_ledger_owner(&conn, &owner.id, &mut app).unwrap_err();
        assert!(err.to_string().contains("does not match"));
    }

    #[test]
    fn duplicate_ledger_details_are_rejected() {
        let conn = conn();
        let first = signer_owners::create(&conn, SignerOwnerKind::Ledger, "ledger-1").unwrap();
        signer_owners::create_vault(&conn, &first.id, "password").unwrap();
        let second = signer_owners::create(&conn, SignerOwnerKind::Ledger, "ledger-2").unwrap();
        signer_owners::create_vault(&conn, &second.id, "password").unwrap();
        let key = [7u8; 32];
        signer_owners::insert_ledger_details(
            &conn,
            NewLedgerOwnerDetails {
                signer_owner_id: &first.id,
                canonical_public_key: &key,
                fingerprint: &signer_owners::ledger_owner_fingerprint(&key),
                enrollment_path: LEDGER_OWNER_ENROLLMENT_PATH,
                app_name: Some("Concordium"),
            },
        )
        .unwrap();

        assert!(
            signer_owners::insert_ledger_details(
                &conn,
                NewLedgerOwnerDetails {
                    signer_owner_id: &second.id,
                    canonical_public_key: &key,
                    fingerprint: &signer_owners::ledger_owner_fingerprint(&key),
                    enrollment_path: LEDGER_OWNER_ENROLLMENT_PATH,
                    app_name: Some("Concordium"),
                },
            )
            .is_err()
        );
    }
}
