//! Ledger hardware-wallet setup commands.
//!
//! This module owns user-facing Ledger enrollment flows. The wallet stores the
//! enrolled Ledger as a Ledger-backed key source while keeping the internal
//! persisted model in signer-owner tables.

use crate::{
    cli::{LedgerRemoveArgs, LedgerSetupArgs, LedgerSubcommand, LedgerSyncArgs},
    commands::{
        ledger_construction::{self, LedgerIdentityIssuanceInput},
        seed::{self, AccountRecoveryMaterial, IdentityRecoveryMaterial, TerminalSeedPrompts},
        ui::{ContextLine, ResolutionSource, SelectItem, log_resolved_context, select_or_single},
    },
};
use anyhow::{Context, Result, bail};
use ccd_wallet_core::{
    store::{
        accounts,
        config::NetworkEntry,
        identities,
        signer_owners::{
            self, LEDGER_OWNER_ENROLLMENT_PATH, NewLedgerOwnerDetails, SignerOwnerKind,
        },
        wallet_state,
    },
    wallet::Net,
};
use ccd_wallet_ledger::{
    ConcordiumLedgerApp, DerivationPath, ExportPrivateKeyNetwork, HidTransport, LedgerError,
    LedgerTransport, PublicKeyOptions,
};
use cliclack::{confirm, input, password};
use rusqlite::Connection;
use std::{cell::RefCell, str::FromStr};

trait LedgerPrompts {
    fn prompt_remove_confirmation(
        &mut self,
        label: &str,
        identity_count: usize,
        account_count: usize,
    ) -> Result<String>;
}

struct TerminalLedgerPrompts;

impl LedgerPrompts for TerminalLedgerPrompts {
    fn prompt_remove_confirmation(
        &mut self,
        label: &str,
        identity_count: usize,
        account_count: usize,
    ) -> Result<String> {
        cliclack::log::warning(format!(
            "This will remove Ledger key source '{label}' from this wallet and delete {} and {} owned by it. This does not modify the physical Ledger device.",
            format_count(identity_count, "identity", "identities"),
            format_count(account_count, "account", "accounts"),
        ))?;
        Ok(input(format!("Type '{label}' to confirm:"))
            .validate(|value: &String| {
                if value.is_empty() {
                    Err("Confirmation is required.")
                } else {
                    Ok(())
                }
            })
            .interact()?)
    }
}

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
/// commands::ledger::run(&mut conn, command).await?;
/// ```
pub async fn run(conn: &mut Connection, command: LedgerSubcommand) -> Result<()> {
    match command {
        LedgerSubcommand::Setup(args) => setup(conn, args).await,
        LedgerSubcommand::Sync(args) => sync(conn, args).await,
        LedgerSubcommand::Show => show().await,
        LedgerSubcommand::Remove(args) => remove(conn, args).await,
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

async fn remove(conn: &Connection, args: LedgerRemoveArgs) -> Result<()> {
    let mut prompts = TerminalLedgerPrompts;
    remove_with_prompts(conn, args, &mut prompts).await
}

async fn remove_with_prompts(
    conn: &Connection,
    args: LedgerRemoveArgs,
    prompts: &mut impl LedgerPrompts,
) -> Result<()> {
    let owner = resolve_remove_ledger_owner(conn, args.label.as_deref(), args.non_interactive)?;
    let identity_count = identities::list(conn)?
        .into_iter()
        .filter(|record| record.signer_owner_id == owner.id)
        .count();
    let account_count = accounts::list(conn)?
        .into_iter()
        .filter(|record| record.signer_owner_id == owner.id)
        .count();
    let confirmation =
        prompts.prompt_remove_confirmation(&owner.label, identity_count, account_count)?;
    if confirmation != owner.label {
        bail!(
            "Ledger key-source removal aborted: confirmation did not match '{}'",
            owner.label
        );
    }

    signer_owners::delete_by_id(conn, &owner.id)?;
    if wallet_state::get(conn, wallet_state::ACTIVE_KEY_SOURCE_KEY)?.as_deref()
        == Some(owner.label.as_str())
    {
        wallet_state::remove(conn, wallet_state::ACTIVE_KEY_SOURCE_KEY)?;
    }

    println!("Ledger key source '{}' removed successfully.", owner.label);

    Ok(())
}

async fn setup(conn: &mut Connection, args: LedgerSetupArgs) -> Result<()> {
    let label = resolve_label(args.label, args.non_interactive)?;
    validate_key_source_label(&label)?;
    if signer_owners::find_by_label(conn, &label)?.is_some() {
        bail!("key source label '{label}' already exists");
    }

    let restore_target = if let Some(network_name) = args.restore.as_deref() {
        Some(
            seed::resolve_sync_network_context(
                conn,
                Some(network_name),
                args.non_interactive,
                false,
            )
            .await?,
        )
    } else {
        None
    };

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
    let owner_dek = signer_owners::create_vault(conn, &owner.id, &local_password)?;
    let fingerprint = signer_owners::ledger_owner_fingerprint(&canonical_public_key);
    let details = signer_owners::insert_ledger_details(
        conn,
        NewLedgerOwnerDetails {
            signer_owner_id: &owner.id,
            canonical_public_key: &canonical_public_key,
            fingerprint: &fingerprint,
            enrollment_path: LEDGER_OWNER_ENROLLMENT_PATH,
            app_name: app_name.as_deref(),
        },
    )?;

    if restore_target.is_some() {
        cliclack::log::success(format!(
            "Ledger key source '{label}' enrolled with fingerprint {fingerprint}."
        ))?;
    } else {
        println!("Ledger key source '{label}' enrolled with fingerprint {fingerprint}.");
    }

    if let Some((network_name, network_entry, endpoint, endpoint_label, _)) = restore_target {
        approve_ledger_recovery_export(
            &label,
            args.non_interactive,
            args.allow_ledger_secret_export,
        )?;
        log_resolved_context(&[
            ContextLine {
                label: "key source:",
                value: label.clone(),
                source: ResolutionSource::Explicit,
            },
            ContextLine {
                label: "network:",
                value: format!("{network_name} @ {endpoint_label}"),
                source: ResolutionSource::Explicit,
            },
        ])?;
        let mut prompts = TerminalSeedPrompts;
        run_ledger_recovery_with_app(
            conn,
            &label,
            &owner.id,
            &owner_dek,
            &details,
            &network_name,
            &network_entry,
            endpoint,
            &[],
            args.non_interactive,
            &mut prompts,
            &mut app,
        )
        .await?;
    }

    Ok(())
}

async fn sync(conn: &mut Connection, args: LedgerSyncArgs) -> Result<()> {
    let (key_source, source) =
        resolve_sync_ledger_label(conn, args.label.as_deref(), args.non_interactive)?;
    let (network_name, network_entry, endpoint, endpoint_label, network_source) =
        seed::resolve_sync_network_context(
            conn,
            args.network.as_deref(),
            args.non_interactive,
            args.no_defaults,
        )
        .await?;

    approve_ledger_recovery_export(
        &key_source.label,
        args.non_interactive,
        args.allow_ledger_secret_export,
    )?;

    log_resolved_context(&[
        ContextLine {
            label: "key source:",
            value: key_source.label.clone(),
            source,
        },
        ContextLine {
            label: "network:",
            value: format!("{network_name} @ {endpoint_label}"),
            source: network_source,
        },
    ])?;

    let local_password = password(format!(
        "Local password for Ledger key source '{}': ",
        key_source.label
    ))
    .allow_empty()
    .interact()?;
    let unlocked_owner = signer_owners::unlock_by_id(conn, &key_source.id, &local_password)?;
    let details = signer_owners::find_ledger_details_by_owner_id(conn, &key_source.id)?
        .with_context(|| {
            format!(
                "Ledger key source '{}' has no enrollment details",
                key_source.label
            )
        })?;

    let transport = HidTransport::open_first()
        .context("failed to open Ledger device; connect a Ledger with the Concordium app open")?;
    let mut app = ConcordiumLedgerApp::new(transport);
    verify_connected_ledger_owner(conn, &key_source.id, &mut app)?;

    let mut prompts = TerminalSeedPrompts;
    run_ledger_recovery_with_app(
        conn,
        &key_source.label,
        &unlocked_owner.record.id,
        &unlocked_owner.dek,
        &details,
        &network_name,
        &network_entry,
        endpoint,
        &args.providers,
        args.non_interactive,
        &mut prompts,
        &mut app,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_ledger_recovery_with_app<T: LedgerTransport>(
    conn: &mut Connection,
    key_source_label: &str,
    signer_owner_id: &str,
    signer_owner_dek: &zeroize::Zeroizing<[u8; ccd_wallet_core::store::crypto::KEY_LEN]>,
    owner_details: &signer_owners::LedgerOwnerDetailsRecord,
    network_name: &str,
    network_entry: &NetworkEntry,
    endpoint: concordium_rust_sdk::v2::Endpoint,
    providers: &[String],
    non_interactive: bool,
    prompts: &mut impl seed::SeedPrompts,
    app: &mut ConcordiumLedgerApp<T>,
) -> Result<()> {
    let export_network = ledger_export_network(seed::infer_net(
        network_name,
        network_entry.wallet_proxy.as_deref(),
        &network_entry.node_endpoint,
    ));
    let app = RefCell::new(app);
    let mut identity_material_for =
        |ip_identity: u32, identity_index: u32| -> Result<IdentityRecoveryMaterial> {
            let mut app = app.borrow_mut();
            let id_cred_sec = ledger_construction::construct_identity_recovery(
                LedgerIdentityIssuanceInput {
                    owner_details,
                    network_genesis_hash: &network_entry.genesis_hash,
                    export_network,
                    ip_identity,
                    identity_index,
                    approved_secret_export: true,
                },
                &mut **app,
            )?;
            Ok(IdentityRecoveryMaterial { id_cred_sec })
        };
    let mut account_material_for =
        |ip_identity: u32, identity_index: u32| -> Result<AccountRecoveryMaterial> {
            let mut app = app.borrow_mut();
            let prf_key = ledger_construction::construct_account_credential_discovery(
                LedgerIdentityIssuanceInput {
                    owner_details,
                    network_genesis_hash: &network_entry.genesis_hash,
                    export_network,
                    ip_identity,
                    identity_index,
                    approved_secret_export: true,
                },
                &mut **app,
            )?;
            Ok(AccountRecoveryMaterial { prf_key })
        };

    seed::run_ledger_recovery(
        conn,
        key_source_label,
        signer_owner_id,
        signer_owner_dek,
        network_name,
        network_entry,
        endpoint,
        providers,
        non_interactive,
        prompts,
        &mut identity_material_for,
        &mut account_material_for,
    )
    .await
}

fn approve_ledger_recovery_export(
    key_source_label: &str,
    non_interactive: bool,
    allow_ledger_secret_export: bool,
) -> Result<()> {
    if allow_ledger_secret_export {
        return Ok(());
    }
    if non_interactive {
        bail!(
            "Ledger recovery for key source '{key_source_label}' requires secret export; rerun with --allow-ledger-secret-export to explicitly allow this non-interactive flow"
        );
    }

    let approved = confirm(format!(
        "Ledger recovery for key source '{key_source_label}' must export recovery secrets into this process temporarily. This is not an on-device signing flow, and you will be prompted on the Ledger device for each identity recovery and account discovery step. Continue?"
    ))
    .initial_value(false)
    .interact()?;
    if !approved {
        bail!("Ledger recovery export was not approved; no recovery state was written");
    }
    Ok(())
}

fn ledger_export_network(net: Net) -> ExportPrivateKeyNetwork {
    match net {
        Net::Mainnet => ExportPrivateKeyNetwork::Mainnet,
        Net::Testnet => ExportPrivateKeyNetwork::Testnet,
    }
}

fn resolve_sync_ledger_label(
    conn: &Connection,
    explicit: Option<&str>,
    non_interactive: bool,
) -> Result<(signer_owners::SignerOwnerRecord, ResolutionSource)> {
    match resolve_ledger_owner(conn, explicit, non_interactive)? {
        (owner, Some(source)) => Ok((owner, source)),
        (owner, None) => Ok((owner, ResolutionSource::Prompted)),
    }
}

fn resolve_remove_ledger_owner(
    conn: &Connection,
    explicit: Option<&str>,
    non_interactive: bool,
) -> Result<signer_owners::SignerOwnerRecord> {
    Ok(resolve_ledger_owner(conn, explicit, non_interactive)?.0)
}

fn resolve_ledger_owner(
    conn: &Connection,
    explicit: Option<&str>,
    non_interactive: bool,
) -> Result<(signer_owners::SignerOwnerRecord, Option<ResolutionSource>)> {
    match explicit {
        Some(label) => signer_owners::find_by_label(conn, label)?
            .filter(|owner| owner.kind == SignerOwnerKind::Ledger)
            .map(|owner| (owner, Some(ResolutionSource::Explicit)))
            .with_context(|| format!("Ledger key source '{}' is not configured", label)),
        None if non_interactive => {
            bail!("Ledger key-source label must be provided in --non-interactive mode")
        }
        None => Ok((select_ledger_label(conn)?, None)),
    }
}

fn select_ledger_label(conn: &Connection) -> Result<signer_owners::SignerOwnerRecord> {
    let owners = signer_owners::list(conn)?
        .into_iter()
        .filter(|owner| owner.kind == SignerOwnerKind::Ledger)
        .collect::<Vec<_>>();
    if owners.is_empty() {
        bail!("no Ledger key sources are configured; run `ccd-wallet ledger setup <LABEL>` first");
    }

    let active = wallet_state::get(conn, wallet_state::ACTIVE_KEY_SOURCE_KEY)?;
    let active_ledger = active.as_deref().and_then(|label| {
        owners
            .iter()
            .find(|owner| owner.label == label)
            .map(|owner| owner.label.clone())
    });
    let items = owners
        .iter()
        .map(|owner| SelectItem {
            value: owner.label.clone(),
            label: owner.label.clone(),
            hint: "ledger".to_owned(),
        })
        .collect::<Vec<_>>();
    let selected = select_or_single("Select Ledger key source", &items, active_ledger.as_ref())?;
    owners
        .into_iter()
        .find(|owner| owner.label == selected)
        .with_context(|| format!("Ledger key source '{}' is not configured", selected))
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

fn format_count(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("1 {singular}")
    } else {
        format!("{count} {plural}")
    }
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
    use rusqlite::params;

    #[derive(Debug, Default)]
    struct TestLedgerPrompts {
        confirmation: String,
        confirmations: Vec<(String, usize, usize)>,
    }

    impl LedgerPrompts for TestLedgerPrompts {
        fn prompt_remove_confirmation(
            &mut self,
            label: &str,
            identity_count: usize,
            account_count: usize,
        ) -> Result<String> {
            self.confirmations
                .push((label.to_owned(), identity_count, account_count));
            Ok(self.confirmation.clone())
        }
    }

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

    fn ledger_owner(
        conn: &Connection,
        label: &str,
        key_byte: u8,
    ) -> signer_owners::SignerOwnerRecord {
        let owner = signer_owners::create(conn, SignerOwnerKind::Ledger, label).unwrap();
        signer_owners::create_vault(conn, &owner.id, "password").unwrap();
        let key = [key_byte; 32];
        signer_owners::insert_ledger_details(
            conn,
            NewLedgerOwnerDetails {
                signer_owner_id: &owner.id,
                canonical_public_key: &key,
                fingerprint: &signer_owners::ledger_owner_fingerprint(&key),
                enrollment_path: LEDGER_OWNER_ENROLLMENT_PATH,
                app_name: Some("Concordium"),
            },
        )
        .unwrap();
        owner
    }

    fn insert_owned_identity_and_account(conn: &Connection, owner_id: &str) -> (i64, i64) {
        conn.execute(
            "INSERT INTO identities (
                signer_owner_id, network_genesis_hash, ip_identity, identity_index,
                label, status, created_at
             ) VALUES (?1, 'net', 7, 0, 'ledger-identity', 'done', 1)",
            params![owner_id],
        )
        .unwrap();
        let identity_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO identity_private_payloads (identity_id, cipher_version, ciphertext, nonce)
             VALUES (?1, 1, x'01', x'02')",
            params![identity_id],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO accounts (
                network_genesis_hash, label, status, source_kind, signer_owner_id,
                ip_identity, identity_index, credential_counter, created_at, updated_at
             ) VALUES ('net', 'ledger-account', 'finalized', 'derived', ?1, 7, 0, 0, 1, 1)",
            params![owner_id],
        )
        .unwrap();
        let account_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO derived_account_private_payloads (account_id, cipher_version, ciphertext, nonce)
             VALUES (?1, 1, x'03', x'04')",
            params![account_id],
        )
        .unwrap();

        (identity_id, account_id)
    }

    fn table_count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
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
    fn non_interactive_recovery_export_requires_allow_flag() {
        let err = approve_ledger_recovery_export("ledger", true, false).unwrap_err();
        assert!(err.to_string().contains("--allow-ledger-secret-export"));
    }

    #[test]
    fn explicit_recovery_export_flag_skips_prompt_guard() {
        approve_ledger_recovery_export("ledger", true, true).unwrap();
    }

    #[test]
    fn sync_label_resolution_requires_ledger_owner() {
        let conn = conn();
        signer_owners::create(&conn, SignerOwnerKind::Seed, "seed-main").unwrap();
        let err = resolve_sync_ledger_label(&conn, Some("seed-main"), true).unwrap_err();
        assert!(
            err.to_string()
                .contains("Ledger key source 'seed-main' is not configured")
        );
    }

    #[tokio::test]
    async fn remove_deletes_ledger_owner_and_cascades_owned_state() {
        let conn = conn();
        let owner = ledger_owner(&conn, "ledger-main", 7);
        let (identity_id, account_id) = insert_owned_identity_and_account(&conn, &owner.id);
        wallet_state::set(&conn, wallet_state::ACTIVE_KEY_SOURCE_KEY, "ledger-main").unwrap();
        let mut prompts = TestLedgerPrompts {
            confirmation: "ledger-main".to_owned(),
            ..Default::default()
        };

        remove_with_prompts(
            &conn,
            LedgerRemoveArgs {
                label: Some("ledger-main".to_owned()),
                non_interactive: false,
            },
            &mut prompts,
        )
        .await
        .unwrap();

        assert_eq!(
            prompts.confirmations,
            vec![("ledger-main".to_owned(), 1, 1)]
        );
        assert!(
            signer_owners::find_by_label(&conn, "ledger-main")
                .unwrap()
                .is_none()
        );
        assert_eq!(table_count(&conn, "signer_owner_vaults"), 0);
        assert_eq!(table_count(&conn, "ledger_owner_details"), 0);
        assert_eq!(table_count(&conn, "identities"), 0);
        assert_eq!(table_count(&conn, "accounts"), 0);
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM identity_private_payloads WHERE identity_id = ?1",
                params![identity_id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            0
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM derived_account_private_payloads WHERE account_id = ?1",
                params![account_id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            0
        );
        assert_eq!(
            wallet_state::get(&conn, wallet_state::ACTIVE_KEY_SOURCE_KEY).unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn remove_confirmation_mismatch_preserves_ledger_owner() {
        let conn = conn();
        ledger_owner(&conn, "ledger-main", 7);
        let mut prompts = TestLedgerPrompts {
            confirmation: "wrong".to_owned(),
            ..Default::default()
        };

        let err = remove_with_prompts(
            &conn,
            LedgerRemoveArgs {
                label: Some("ledger-main".to_owned()),
                non_interactive: false,
            },
            &mut prompts,
        )
        .await
        .unwrap_err();

        assert!(err.to_string().contains("confirmation did not match"));
        assert!(
            signer_owners::find_by_label(&conn, "ledger-main")
                .unwrap()
                .is_some()
        );
        assert_eq!(table_count(&conn, "signer_owner_vaults"), 1);
        assert_eq!(table_count(&conn, "ledger_owner_details"), 1);
    }

    #[tokio::test]
    async fn remove_inactive_ledger_leaves_active_key_source_unchanged() {
        let conn = conn();
        ledger_owner(&conn, "ledger-main", 7);
        ledger_owner(&conn, "other-ledger", 8);
        wallet_state::set(&conn, wallet_state::ACTIVE_KEY_SOURCE_KEY, "other-ledger").unwrap();
        let mut prompts = TestLedgerPrompts {
            confirmation: "ledger-main".to_owned(),
            ..Default::default()
        };

        remove_with_prompts(
            &conn,
            LedgerRemoveArgs {
                label: Some("ledger-main".to_owned()),
                non_interactive: false,
            },
            &mut prompts,
        )
        .await
        .unwrap();

        assert_eq!(
            wallet_state::get(&conn, wallet_state::ACTIVE_KEY_SOURCE_KEY).unwrap(),
            Some("other-ledger".to_owned())
        );
        assert!(
            signer_owners::find_by_label(&conn, "other-ledger")
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn remove_rejects_seed_key_source_without_deleting_it() {
        let conn = conn();
        signer_owners::create(&conn, SignerOwnerKind::Seed, "seed-main").unwrap();
        let mut prompts = TestLedgerPrompts::default();

        let err = remove_with_prompts(
            &conn,
            LedgerRemoveArgs {
                label: Some("seed-main".to_owned()),
                non_interactive: false,
            },
            &mut prompts,
        )
        .await
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("Ledger key source 'seed-main' is not configured")
        );
        assert!(prompts.confirmations.is_empty());
        assert!(
            signer_owners::find_by_label(&conn, "seed-main")
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn remove_missing_label_errors_in_non_interactive_mode() {
        let conn = conn();
        ledger_owner(&conn, "ledger-main", 7);
        let mut prompts = TestLedgerPrompts::default();

        let err = remove_with_prompts(
            &conn,
            LedgerRemoveArgs {
                label: None,
                non_interactive: true,
            },
            &mut prompts,
        )
        .await
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("Ledger key-source label must be provided")
        );
        assert!(prompts.confirmations.is_empty());
        assert!(
            signer_owners::find_by_label(&conn, "ledger-main")
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn setup_restore_missing_network_errors_before_writing_owner() {
        let mut conn = conn();
        let err = setup(
            &mut conn,
            LedgerSetupArgs {
                label: Some("ledger-main".to_owned()),
                restore: Some("missingnet".to_owned()),
                allow_ledger_secret_export: true,
                non_interactive: true,
            },
        )
        .await
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("network 'missingnet' is not registered")
        );
        assert!(
            signer_owners::find_by_label(&conn, "ledger-main")
                .unwrap()
                .is_none()
        );
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
