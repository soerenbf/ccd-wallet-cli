//! Shared helpers for connect command feature handlers.

use crate::commands::ui::{SelectItem, select_or_single};
use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use ccd_wallet_core::{
    store::{
        accounts::{self, AccountRecord, AccountSourceKind, AccountStatus},
        config::{self, NetworkEntry, load},
        seeds,
        signer_owners::{self, SignerOwnerKind},
    },
    wallet::ConcordiumHdWallet,
};
use cliclack::password;
use concordium_rust_sdk::{
    common::types::{AccountAddress, Amount, KeyIndex, KeyPair},
    id::types::{AccountKeys, CredentialData, SignatureThreshold},
    smart_contracts::common::{
        OwnedParameter,
        schema::{Type as SchemaType, VersionedModuleSchema},
    },
    types::WalletAccount,
};
use rusqlite::Connection;
use std::{collections::BTreeMap, str::FromStr};

pub(super) fn resolve_network_display_name(network_genesis_hash: &str) -> Result<String> {
    let config = load()?;
    let matches = config::aliases_by_genesis_hash(&config, network_genesis_hash);
    if matches.is_empty() {
        Ok(network_genesis_hash.to_owned())
    } else {
        Ok(matches.join(", "))
    }
}

pub(super) fn select_network() -> Result<(String, NetworkEntry)> {
    let config = load()?;
    if config.networks.is_empty() {
        bail!("no networks are configured; run `ccd-wallet network add` first");
    }

    let items = config
        .networks
        .iter()
        .map(|(name, entry)| SelectItem {
            value: name.clone(),
            label: name.clone(),
            hint: entry.node_endpoint.clone(),
        })
        .collect::<Vec<_>>();
    let selected = select_or_single("Select network for browser session", &items, None)?;
    let entry = config
        .networks
        .get(&selected)
        .cloned()
        .context("selected network was not found")?;
    Ok((selected, entry))
}

pub(super) fn resolve_network_entry(network_genesis_hash: &str) -> Result<(String, NetworkEntry)> {
    let config = load()?;
    config
        .networks
        .iter()
        .find(|(_, entry)| entry.genesis_hash == network_genesis_hash)
        .map(|(name, entry)| (name.clone(), entry.clone()))
        .with_context(|| {
            format!(
                "no registered network matches genesis hash {network_genesis_hash}; run `ccd-wallet network add` to register it"
            )
        })
}

pub(super) fn select_account(
    conn: &Connection,
    network_genesis_hash: &str,
) -> Result<AccountRecord> {
    let accounts = accounts::list(conn)?
        .into_iter()
        .filter(|record| record.network_genesis_hash == network_genesis_hash)
        .filter(|record| record.status == AccountStatus::Finalized)
        .collect::<Vec<_>>();
    if accounts.is_empty() {
        bail!("no finalized accounts are available for the selected network");
    }

    let seed_labels = signer_owners::list(conn)?
        .into_iter()
        .map(|owner| (owner.id, owner.label))
        .collect::<BTreeMap<_, _>>();
    let items = accounts
        .iter()
        .map(|record| SelectItem {
            value: record.id,
            label: render_account_label(record, &seed_labels),
            hint: account_hint(record),
        })
        .collect::<Vec<_>>();
    let selected = select_or_single("Select account authority for browser session", &items, None)?;
    accounts
        .into_iter()
        .find(|record| record.id == selected)
        .context("selected account was not found")
}

fn render_account_label(record: &AccountRecord, seed_labels: &BTreeMap<String, String>) -> String {
    if record.source_kind == AccountSourceKind::Imported {
        format!("[imported] {}", record.label)
    } else {
        let seed_label = seed_labels
            .get(&record.signer_owner_id)
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

pub(super) fn read_account_address(
    conn: &Connection,
    account: &AccountRecord,
    network_name: &str,
) -> Result<String> {
    match account.source_kind {
        AccountSourceKind::Derived => {
            ensure_seed_backed_account(conn, account)?;
            let seed = seeds::list(conn)?
                .into_iter()
                .find(|seed| seed.id == account.signer_owner_id)
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

pub(super) fn unlock_wallet_account(
    conn: &Connection,
    network_name: &str,
    network_entry: &NetworkEntry,
    account_address: &str,
) -> Result<WalletAccount> {
    let accounts = accounts::list(conn)?
        .into_iter()
        .filter(|record| record.network_genesis_hash == network_entry.genesis_hash)
        .filter(|record| record.status == AccountStatus::Finalized)
        .collect::<Vec<_>>();
    for account in accounts {
        let candidate =
            unlock_wallet_account_candidate(conn, network_name, network_entry, &account)?;
        if candidate.address.to_string() == account_address {
            return Ok(candidate);
        }
    }
    bail!("no finalized wallet account matches the session-bound account address")
}

fn unlock_wallet_account_candidate(
    conn: &Connection,
    network_name: &str,
    network_entry: &NetworkEntry,
    account: &AccountRecord,
) -> Result<WalletAccount> {
    match account.source_kind {
        AccountSourceKind::Derived => unlock_derived_wallet_account(conn, network_entry, account),
        AccountSourceKind::Imported => unlock_imported_wallet_account(conn, network_name, account),
    }
}

fn ensure_seed_backed_account(conn: &Connection, account: &AccountRecord) -> Result<()> {
    let owner = signer_owners::find_by_id(conn, &account.signer_owner_id)?.with_context(|| {
        format!(
            "derived account '{}' references unknown key source",
            account.label
        )
    })?;
    if owner.kind == SignerOwnerKind::Ledger {
        bail!(
            "Ledger-backed account '{}' cannot be used for browser connect signing yet; Ledger transaction signing is not yet supported and no transaction was submitted",
            account.label
        );
    }
    Ok(())
}

fn unlock_derived_wallet_account(
    conn: &Connection,
    network_entry: &NetworkEntry,
    account: &AccountRecord,
) -> Result<WalletAccount> {
    ensure_seed_backed_account(conn, account)?;
    let seed = seeds::list(conn)?
        .into_iter()
        .find(|seed| seed.id == account.signer_owner_id)
        .context("selected account references unknown seed")?;
    let password: String = password(format!("Password for seed '{}':", seed.label))
        .allow_empty()
        .interact()?;
    let unlocked = seeds::unlock_context(conn, &seed.label, &password)?;
    let payload = accounts::decrypt_private_payload(conn, account.id, &unlocked.dek)?;
    let seed_phrase =
        std::str::from_utf8(&unlocked.secret).context("seed phrase is not valid UTF-8")?;
    let net = crate::commands::seed::infer_net(&network_entry.genesis_hash);
    let wallet = ConcordiumHdWallet::from_seed_phrase(seed_phrase, net)?;
    let signing_key = wallet.get_account_signing_key(
        account.ip_identity,
        account.identity_index,
        account.credential_counter,
    )?;
    let mut keys = BTreeMap::new();
    keys.insert(
        KeyIndex(0),
        KeyPair::from(ed25519_dalek::SigningKey::from_bytes(&signing_key)),
    );
    Ok(WalletAccount {
        address: AccountAddress::from_str(&payload.account_address)?,
        keys: AccountKeys::from(CredentialData {
            keys,
            threshold: SignatureThreshold::ONE,
        }),
    })
}

fn unlock_imported_wallet_account(
    conn: &Connection,
    network_name: &str,
    account: &AccountRecord,
) -> Result<WalletAccount> {
    let vault_password: String = password(format!(
        "Imported accounts vault password for '{}':",
        network_name
    ))
    .allow_empty()
    .interact()?;
    let unlocked =
        accounts::unlock_imported_vault(conn, &account.network_genesis_hash, &vault_password)?;
    let payload = accounts::decrypt_imported_payload(conn, account.id, &unlocked.dek)?;
    WalletAccount::from_json_value(serde_json::json!({
        "address": payload.account_address,
        "accountKeys": payload.account_keys,
    }))
    .context("failed to build signer for imported account")
}

pub(super) fn parse_amount_micro_ccd(value: &str) -> Result<Amount> {
    let micro_ccd = value
        .parse::<u64>()
        .with_context(|| format!("amountMicroCcd must be an unsigned integer, got '{value}'"))?;
    Ok(Amount::from_micro_ccd(micro_ccd))
}

pub(super) fn parse_parameter_hex(value: &str) -> Result<OwnedParameter> {
    let bytes = parse_hex_bytes(value, "parameterHex")?;
    Ok(OwnedParameter::new_unchecked(bytes))
}

pub(super) fn parse_hex_bytes(value: &str, field_name: &str) -> Result<Vec<u8>> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    hex::decode(value).with_context(|| format!("{field_name} must be lower- or upper-case hex"))
}

pub(super) fn display_parameter_with_schema(
    parameter_hex: &str,
    schema: Option<&serde_json::Value>,
    schema_type: impl FnOnce(&VersionedModuleSchema) -> Result<SchemaType>,
) -> String {
    match try_display_parameter_with_schema(parameter_hex, schema, schema_type) {
        Ok(value) => value,
        Err(_) => format!("0x{parameter_hex}"),
    }
}

fn try_display_parameter_with_schema(
    parameter_hex: &str,
    schema: Option<&serde_json::Value>,
    schema_type: impl FnOnce(&VersionedModuleSchema) -> Result<SchemaType>,
) -> Result<String> {
    let schema_base64 = schema
        .and_then(schema_base64_value)
        .context("schema is not a base64-encoded versioned module schema")?;
    let schema_base64 = schema_base64.trim_end_matches('=');
    let module_schema = VersionedModuleSchema::from_base64_str(schema_base64)
        .or_else(|_| {
            let bytes = BASE64.decode(schema_base64_value(schema.unwrap()).unwrap_or_default())?;
            let encoded = BASE64.encode(bytes).trim_end_matches('=').to_owned();
            VersionedModuleSchema::from_base64_str(&encoded).map_err(anyhow::Error::from)
        })
        .context("failed to decode supplied module schema")?;
    let ty = schema_type(&module_schema).context("failed to resolve parameter schema")?;
    let bytes = hex::decode(parameter_hex).context("parameterHex is not valid hex")?;
    ty.to_json_string_pretty(&bytes)
        .map_err(|err| anyhow::anyhow!(err.to_string()))
}

fn schema_base64_value(value: &serde_json::Value) -> Option<&str> {
    value.as_str().or_else(|| {
        value
            .get("base64")
            .or_else(|| value.get("moduleSchema"))
            .or_else(|| value.get("schema"))
            .and_then(serde_json::Value::as_str)
    })
}
