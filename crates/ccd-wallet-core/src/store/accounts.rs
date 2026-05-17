use crate::store::crypto::{
    Argon2Params, KEY_LEN, aead_decrypt, aead_encrypt, derive_kek, generate_dek, object_aad,
    random_salt, zeroizing_array_from_slice,
};
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use zeroize::Zeroizing;

const KDF_ALGORITHM: &str = "argon2id";
const CIPHER_VERSION: u32 = 1;
const ACCOUNT_PRIVATE_PAYLOAD_KIND: &str = "account_private_payload";
const IMPORTED_ACCOUNT_PAYLOAD_KIND: &str = "imported_account_payload";
const IMPORTED_ACCOUNT_VAULT_DEK_KIND: &str = "imported_account_vault_dek";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountSourceKind {
    Derived,
    Imported,
}

impl AccountSourceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Derived => "derived",
            Self::Imported => "imported",
        }
    }

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "derived" => Ok(Self::Derived),
            "imported" => Ok(Self::Imported),
            other => bail!("unsupported account source kind '{other}'"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRecord {
    pub id: i64,
    pub seed_id: String,
    pub network_genesis_hash: String,
    pub ip_identity: u32,
    pub identity_index: u32,
    pub credential_counter: u32,
    pub source_kind: AccountSourceKind,
    pub imported_vault_id: Option<String>,
    pub import_kind: Option<String>,
    pub source_metadata_json: Option<String>,
    pub label: String,
    pub status: AccountStatus,
    pub transaction_hash: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountPrivatePayload {
    pub account_address: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedAccountSecretPayload {
    pub account_address: String,
    pub account_keys: serde_json::Value,
    pub credentials: serde_json::Value,
    pub encryption_public_key: Option<String>,
    pub encryption_secret_key: Option<String>,
    pub credential_holder_info: Option<serde_json::Value>,
    pub source: ImportedAccountSourceMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountSigningSource {
    Derived {
        seed_id: String,
        ip_identity: u32,
        identity_index: u32,
        credential_counter: u32,
    },
    Imported(ImportedAccountSecretPayload),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedAccountSourceMetadata {
    pub import_kind: String,
    pub original_filename: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedAccountVaultRecord {
    pub id: String,
    pub network_genesis_hash: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug)]
pub struct UnlockedImportedAccountVault {
    pub record: ImportedAccountVaultRecord,
    pub dek: Zeroizing<[u8; KEY_LEN]>,
}

#[derive(Debug)]
struct ImportedAccountVault {
    record: ImportedAccountVaultRecord,
    kdf_algorithm: String,
    kdf_params: Argon2Params,
    salt: Vec<u8>,
    encrypted_dek: Vec<u8>,
    dek_nonce: Vec<u8>,
    cipher_version: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountStatus {
    Pending,
    Finalized,
}

impl AccountStatus {
    fn as_str(self) -> &'static str {
        match self {
            AccountStatus::Pending => "pending",
            AccountStatus::Finalized => "finalized",
        }
    }

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "finalized" => Ok(Self::Finalized),
            other => bail!("unsupported account status '{other}'"),
        }
    }
}

pub struct PendingAccount<'a> {
    pub network_genesis_hash: &'a str,
    pub seed_id: &'a str,
    pub ip_identity: u32,
    pub identity_index: u32,
    pub credential_counter: u32,
    pub label: &'a str,
}

pub fn insert_pending(conn: &Connection, pending: PendingAccount<'_>) -> Result<i64> {
    if find_by_network_and_label(conn, pending.network_genesis_hash, pending.label)?.is_some() {
        bail!(
            "account label '{}' already exists for network '{}'",
            pending.label,
            pending.network_genesis_hash
        );
    }

    if find_by_derivation(
        conn,
        pending.network_genesis_hash,
        pending.seed_id,
        pending.ip_identity,
        pending.identity_index,
        pending.credential_counter,
    )?
    .is_some()
    {
        bail!(
            "credential counter {} already exists for seed '{}', provider {}, identity index {} on network '{}'",
            pending.credential_counter,
            pending.seed_id,
            pending.ip_identity,
            pending.identity_index,
            pending.network_genesis_hash
        );
    }

    let now = now_unix_seconds()?;
    conn.execute(
        "INSERT INTO accounts (
            seed_id, network_genesis_hash, ip_identity, identity_index, credential_counter,
            label, status, source_kind, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            pending.seed_id,
            pending.network_genesis_hash,
            pending.ip_identity,
            pending.identity_index,
            pending.credential_counter,
            pending.label,
            AccountStatus::Pending.as_str(),
            AccountSourceKind::Derived.as_str(),
            now,
            now,
        ],
    )
    .with_context(|| format!("failed to insert account '{}'", pending.label))?;

    Ok(conn.last_insert_rowid())
}

pub fn set_submitted_transaction(conn: &Connection, id: i64, transaction_hash: &str) -> Result<()> {
    let now = now_unix_seconds()?;
    let affected = conn
        .execute(
            "UPDATE accounts SET transaction_hash = ?1, updated_at = ?2 WHERE id = ?3",
            params![transaction_hash, now, id],
        )
        .with_context(|| format!("failed to update account {id} transaction hash"))?;

    if affected == 0 {
        bail!("account {id} is not configured");
    }

    Ok(())
}

pub fn set_finalized(
    conn: &mut Connection,
    id: i64,
    seed_dek: &[u8; KEY_LEN],
    transaction_hash: Option<&str>,
    account_address: &str,
) -> Result<()> {
    let tx = conn
        .transaction()
        .context("failed to start account finalization transaction")?;
    let mut record = find_by_id_in_tx(&tx, id)?;
    let now = now_unix_seconds()?;

    let affected = tx
        .execute(
            "UPDATE accounts
             SET status = ?1, transaction_hash = COALESCE(?2, transaction_hash), updated_at = ?3
             WHERE id = ?4",
            params![AccountStatus::Finalized.as_str(), transaction_hash, now, id,],
        )
        .with_context(|| format!("failed to mark account {id} finalized"))?;

    if affected == 0 {
        bail!("account {id} is not configured");
    }

    record.status = AccountStatus::Finalized;
    if let Some(transaction_hash) = transaction_hash {
        record.transaction_hash = Some(transaction_hash.to_owned());
    }
    record.updated_at = now;

    upsert_private_payload_in_tx(
        &tx,
        &record,
        seed_dek,
        &AccountPrivatePayload {
            account_address: account_address.to_owned(),
        },
    )?;
    tx.commit()
        .context("failed to commit account finalization transaction")?;
    Ok(())
}

pub struct RecoveredAccount<'a> {
    pub network_genesis_hash: &'a str,
    pub seed_id: &'a str,
    pub ip_identity: u32,
    pub identity_index: u32,
    pub credential_counter: u32,
    pub label: &'a str,
    pub account_address: &'a str,
}

pub fn import_recovered(
    conn: &mut Connection,
    seed_dek: &[u8; KEY_LEN],
    recovered: RecoveredAccount<'_>,
) -> Result<(AccountRecord, bool)> {
    let tx = conn
        .transaction()
        .context("failed to start recovered account import transaction")?;

    if let Some(existing) = find_by_derivation(
        &tx,
        recovered.network_genesis_hash,
        recovered.seed_id,
        recovered.ip_identity,
        recovered.identity_index,
        recovered.credential_counter,
    )? {
        let now = now_unix_seconds()?;
        tx.execute(
            "UPDATE accounts SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![AccountStatus::Finalized.as_str(), now, existing.id],
        )
        .with_context(|| format!("failed to update recovered account {}", existing.id))?;

        let updated = AccountRecord {
            status: AccountStatus::Finalized,
            updated_at: now,
            ..existing
        };
        upsert_private_payload_in_tx(
            &tx,
            &updated,
            seed_dek,
            &AccountPrivatePayload {
                account_address: recovered.account_address.to_owned(),
            },
        )?;
        tx.commit()
            .context("failed to commit recovered account update transaction")?;
        return Ok((updated, false));
    }

    if find_by_network_and_label(&tx, recovered.network_genesis_hash, recovered.label)?.is_some() {
        bail!(
            "account label '{}' already exists for network '{}'",
            recovered.label,
            recovered.network_genesis_hash
        );
    }

    let now = now_unix_seconds()?;
    tx.execute(
        "INSERT INTO accounts (
            seed_id, network_genesis_hash, ip_identity, identity_index, credential_counter,
            label, status, source_kind, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            recovered.seed_id,
            recovered.network_genesis_hash,
            recovered.ip_identity,
            recovered.identity_index,
            recovered.credential_counter,
            recovered.label,
            AccountStatus::Finalized.as_str(),
            AccountSourceKind::Derived.as_str(),
            now,
            now,
        ],
    )
    .with_context(|| format!("failed to insert recovered account '{}'", recovered.label))?;

    let record = AccountRecord {
        id: tx.last_insert_rowid(),
        seed_id: recovered.seed_id.to_owned(),
        network_genesis_hash: recovered.network_genesis_hash.to_owned(),
        ip_identity: recovered.ip_identity,
        identity_index: recovered.identity_index,
        credential_counter: recovered.credential_counter,
        source_kind: AccountSourceKind::Derived,
        imported_vault_id: None,
        import_kind: None,
        source_metadata_json: None,
        label: recovered.label.to_owned(),
        status: AccountStatus::Finalized,
        transaction_hash: None,
        created_at: now,
        updated_at: now,
    };
    upsert_private_payload_in_tx(
        &tx,
        &record,
        seed_dek,
        &AccountPrivatePayload {
            account_address: recovered.account_address.to_owned(),
        },
    )?;
    tx.commit()
        .context("failed to commit recovered account insert transaction")?;
    Ok((record, true))
}

pub fn parse_genesis_account_json(
    json: &str,
    original_filename: Option<String>,
) -> Result<ImportedAccountSecretPayload> {
    let value: Value =
        serde_json::from_str(json).context("failed to parse genesis account JSON")?;
    parse_genesis_account_value(&value, original_filename)
}

pub fn parse_genesis_account_value(
    value: &Value,
    original_filename: Option<String>,
) -> Result<ImportedAccountSecretPayload> {
    let address = value
        .get("address")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .context("genesis account JSON is missing non-empty 'address'")?;
    let account_keys = value
        .get("accountKeys")
        .cloned()
        .context("genesis account JSON is missing 'accountKeys'")?;
    validate_account_keys(&account_keys)?;
    let credentials = value
        .get("credentials")
        .cloned()
        .context("genesis account JSON is missing 'credentials'")?;
    let encryption_public_key = value
        .get("encryptionPublicKey")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let encryption_secret_key = value
        .get("encryptionSecretKey")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let credential_holder_info = value.get("aci").cloned();

    Ok(ImportedAccountSecretPayload {
        account_address: address.to_owned(),
        account_keys,
        credentials,
        encryption_public_key,
        encryption_secret_key,
        credential_holder_info,
        source: ImportedAccountSourceMetadata {
            import_kind: "genesis".to_owned(),
            original_filename,
        },
    })
}

fn validate_account_keys(account_keys: &Value) -> Result<()> {
    let keys = account_keys
        .get("keys")
        .and_then(Value::as_object)
        .context("genesis account JSON 'accountKeys.keys' must be an object")?;
    let has_signing_key = keys.values().any(|credential_keys| {
        credential_keys
            .get("keys")
            .and_then(Value::as_object)
            .map(|inner| {
                inner.values().any(|key| {
                    key.get("signKey").and_then(Value::as_str).is_some()
                        && key.get("verifyKey").and_then(Value::as_str).is_some()
                })
            })
            .unwrap_or(false)
    });
    if !has_signing_key {
        bail!("genesis account JSON does not contain account signing key material");
    }
    Ok(())
}

pub struct ImportedAccount<'a> {
    pub network_genesis_hash: &'a str,
    pub label: &'a str,
    pub import_kind: &'a str,
    pub source_metadata_json: Option<&'a str>,
    pub payload: &'a ImportedAccountSecretPayload,
}

pub fn import_imported_account(
    conn: &mut Connection,
    vault_dek: &[u8; KEY_LEN],
    vault: &ImportedAccountVaultRecord,
    imported: ImportedAccount<'_>,
) -> Result<AccountRecord> {
    if vault.network_genesis_hash != imported.network_genesis_hash {
        bail!(
            "imported account vault belongs to network '{}', not '{}'",
            vault.network_genesis_hash,
            imported.network_genesis_hash
        );
    }
    if find_by_network_and_label(conn, imported.network_genesis_hash, imported.label)?.is_some() {
        bail!(
            "account label '{}' already exists for network '{}'",
            imported.label,
            imported.network_genesis_hash
        );
    }

    let tx = conn
        .transaction()
        .context("failed to start imported account transaction")?;
    let now = now_unix_seconds()?;
    tx.execute(
        "INSERT INTO accounts (
            network_genesis_hash, label, status, source_kind, imported_vault_id,
            import_kind, source_metadata_json, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            imported.network_genesis_hash,
            imported.label,
            AccountStatus::Finalized.as_str(),
            AccountSourceKind::Imported.as_str(),
            vault.id,
            imported.import_kind,
            imported.source_metadata_json,
            now,
            now,
        ],
    )
    .with_context(|| format!("failed to insert imported account '{}'", imported.label))?;

    let record = AccountRecord {
        id: tx.last_insert_rowid(),
        seed_id: String::new(),
        network_genesis_hash: imported.network_genesis_hash.to_owned(),
        ip_identity: 0,
        identity_index: 0,
        credential_counter: 0,
        source_kind: AccountSourceKind::Imported,
        imported_vault_id: Some(vault.id.clone()),
        import_kind: Some(imported.import_kind.to_owned()),
        source_metadata_json: imported.source_metadata_json.map(ToOwned::to_owned),
        label: imported.label.to_owned(),
        status: AccountStatus::Finalized,
        transaction_hash: None,
        created_at: now,
        updated_at: now,
    };
    upsert_imported_payload_in_tx(&tx, &record, vault_dek, imported.payload)?;
    tx.commit()
        .context("failed to commit imported account transaction")?;
    Ok(record)
}

pub fn imported_vault_exists(conn: &Connection, network_genesis_hash: &str) -> Result<bool> {
    Ok(load_imported_vault(conn, network_genesis_hash)?.is_some())
}

pub fn create_or_unlock_imported_vault(
    conn: &Connection,
    network_genesis_hash: &str,
    password: &str,
) -> Result<UnlockedImportedAccountVault> {
    if let Some(vault) = load_imported_vault(conn, network_genesis_hash)? {
        return unlock_imported_vault_record(vault, password);
    }

    let id = Uuid::new_v4().to_string();
    let now = now_unix_seconds()?;
    let params = Argon2Params::default();
    let salt = random_salt();
    let dek = generate_dek();
    let kek = derive_kek(password, &salt, &params)?;
    let dek_aad = object_aad(&id, IMPORTED_ACCOUNT_VAULT_DEK_KIND, CIPHER_VERSION);
    let (encrypted_dek, dek_nonce) = aead_encrypt(&kek, &*dek, &dek_aad)?;
    let kdf_params_json =
        serde_json::to_string(&params).context("failed to serialise imported vault KDF params")?;

    conn.execute(
        "INSERT INTO imported_account_vaults (
            id, network_genesis_hash, kdf_algorithm, kdf_params_json, salt, encrypted_dek,
            dek_nonce, cipher_version, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            id,
            network_genesis_hash,
            KDF_ALGORITHM,
            kdf_params_json,
            salt.as_slice(),
            encrypted_dek,
            dek_nonce.as_slice(),
            CIPHER_VERSION,
            now,
            now,
        ],
    )
    .with_context(|| {
        format!("failed to create imported accounts vault for network '{network_genesis_hash}'")
    })?;

    Ok(UnlockedImportedAccountVault {
        record: ImportedAccountVaultRecord {
            id,
            network_genesis_hash: network_genesis_hash.to_owned(),
            created_at: now,
            updated_at: now,
        },
        dek,
    })
}

pub fn unlock_imported_vault(
    conn: &Connection,
    network_genesis_hash: &str,
    password: &str,
) -> Result<UnlockedImportedAccountVault> {
    let vault = load_imported_vault(conn, network_genesis_hash)?.with_context(|| {
        format!("imported accounts vault for network '{network_genesis_hash}' is not configured")
    })?;
    unlock_imported_vault_record(vault, password)
}

pub fn decrypt_imported_payload(
    conn: &Connection,
    id: i64,
    vault_dek: &[u8; KEY_LEN],
) -> Result<ImportedAccountSecretPayload> {
    let record = find_by_id(conn, id)?;
    decrypt_imported_payload_for_record(conn, &record, vault_dek)
}

pub fn resolve_signing_source(
    conn: &Connection,
    id: i64,
    imported_vault_dek: Option<&[u8; KEY_LEN]>,
) -> Result<AccountSigningSource> {
    let record = find_by_id(conn, id)?;
    match record.source_kind {
        AccountSourceKind::Derived => Ok(AccountSigningSource::Derived {
            seed_id: record.seed_id,
            ip_identity: record.ip_identity,
            identity_index: record.identity_index,
            credential_counter: record.credential_counter,
        }),
        AccountSourceKind::Imported => {
            let dek = imported_vault_dek.context(
                "imported accounts vault must be unlocked to sign with imported account",
            )?;
            Ok(AccountSigningSource::Imported(
                decrypt_imported_payload_for_record(conn, &record, dek)?,
            ))
        }
    }
}

pub fn next_generated_label(
    conn: &Connection,
    network_genesis_hash: &str,
    prefix: &str,
) -> Result<String> {
    let existing = list(conn)?
        .into_iter()
        .filter(|record| record.network_genesis_hash == network_genesis_hash)
        .map(|record| record.label)
        .collect::<std::collections::BTreeSet<_>>();

    for index in 1u32.. {
        let candidate = format!("{prefix}_{index}");
        if !existing.contains(&candidate) {
            return Ok(candidate);
        }
    }

    unreachable!("u32 label space exhausted")
}

pub fn prune_by_network(conn: &Connection, network_genesis_hash: &str) -> Result<usize> {
    let deleted = conn
        .execute(
            "DELETE FROM accounts WHERE network_genesis_hash = ?1",
            params![network_genesis_hash],
        )
        .with_context(|| {
            format!("failed to prune accounts for network '{network_genesis_hash}'")
        })?;
    conn.execute(
        "DELETE FROM imported_account_vaults WHERE network_genesis_hash = ?1",
        params![network_genesis_hash],
    )
    .with_context(|| {
        format!("failed to prune imported account vaults for network '{network_genesis_hash}'")
    })?;
    Ok(deleted)
}

pub fn distinct_network_genesis_hashes(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn
        .prepare("SELECT DISTINCT network_genesis_hash FROM accounts ORDER BY network_genesis_hash")
        .context("failed to prepare distinct account network hash query")?;
    let rows = stmt
        .query_map([], |row| row.get(0))
        .context("failed to query distinct account network hashes")?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read distinct account network hashes")
}

pub fn list(conn: &Connection) -> Result<Vec<AccountRecord>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, COALESCE(seed_id, ''), network_genesis_hash, COALESCE(ip_identity, 0), COALESCE(identity_index, 0),
                    COALESCE(credential_counter, 0), source_kind, imported_vault_id, import_kind, source_metadata_json,
                    label, status, transaction_hash, created_at, updated_at
             FROM accounts ORDER BY label",
        )
        .context("failed to prepare account list query")?;

    let rows = stmt
        .query_map([], map_account_row)
        .context("failed to query accounts")?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read account rows")
}

pub fn find_by_network_and_label(
    conn: &Connection,
    network_genesis_hash: &str,
    label: &str,
) -> Result<Option<AccountRecord>> {
    conn.query_row(
        "SELECT id, COALESCE(seed_id, ''), network_genesis_hash, COALESCE(ip_identity, 0), COALESCE(identity_index, 0),
                COALESCE(credential_counter, 0), source_kind, imported_vault_id, import_kind, source_metadata_json,
                label, status, transaction_hash, created_at, updated_at
         FROM accounts WHERE network_genesis_hash = ?1 AND label = ?2",
        params![network_genesis_hash, label],
        map_account_row,
    )
    .optional()
    .with_context(|| {
        format!("failed to query account '{label}' for network '{network_genesis_hash}'")
    })
}

pub fn find_by_derivation(
    conn: &Connection,
    network_genesis_hash: &str,
    seed_id: &str,
    ip_identity: u32,
    identity_index: u32,
    credential_counter: u32,
) -> Result<Option<AccountRecord>> {
    conn.query_row(
        "SELECT id, COALESCE(seed_id, ''), network_genesis_hash, COALESCE(ip_identity, 0), COALESCE(identity_index, 0),
                COALESCE(credential_counter, 0), source_kind, imported_vault_id, import_kind, source_metadata_json,
                label, status, transaction_hash, created_at, updated_at
         FROM accounts
         WHERE network_genesis_hash = ?1 AND seed_id = ?2 AND ip_identity = ?3
           AND identity_index = ?4 AND credential_counter = ?5 AND source_kind = 'derived'",
        params![
            network_genesis_hash,
            seed_id,
            ip_identity,
            identity_index,
            credential_counter,
        ],
        map_account_row,
    )
    .optional()
    .with_context(|| {
        format!(
            "failed to query account credential counter {credential_counter} for seed '{seed_id}', provider {ip_identity}, identity index {identity_index}, network '{network_genesis_hash}'"
        )
    })
}

pub fn next_credential_counter(
    conn: &Connection,
    network_genesis_hash: &str,
    seed_id: &str,
    ip_identity: u32,
    identity_index: u32,
) -> Result<u32> {
    let max_counter: Option<u32> = conn
        .query_row(
            "SELECT MAX(credential_counter) FROM accounts
             WHERE network_genesis_hash = ?1 AND seed_id = ?2 AND ip_identity = ?3 AND identity_index = ?4 AND source_kind = 'derived'",
            params![network_genesis_hash, seed_id, ip_identity, identity_index],
            |row| row.get(0),
        )
        .context("failed to query next account credential counter")?;

    Ok(max_counter.map(|idx| idx + 1).unwrap_or(0))
}

pub fn decrypt_private_payload(
    conn: &Connection,
    id: i64,
    seed_dek: &[u8; KEY_LEN],
) -> Result<AccountPrivatePayload> {
    let record = find_by_id(conn, id)?;
    decrypt_private_payload_for_record(conn, &record, seed_dek)
}

pub fn rename(conn: &Connection, id: i64, new_label: &str) -> Result<()> {
    let record = find_by_id(conn, id)?;
    if record.label == new_label {
        return Ok(());
    }
    if find_by_network_and_label(conn, &record.network_genesis_hash, new_label)?.is_some() {
        bail!(
            "account label '{}' already exists for network '{}'",
            new_label,
            record.network_genesis_hash
        );
    }
    let affected = conn
        .execute(
            "UPDATE accounts SET label = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_label, now_unix_seconds()?, id],
        )
        .with_context(|| format!("failed to rename account {id} to '{new_label}'"))?;
    if affected == 0 {
        bail!("account {id} is not configured");
    }
    Ok(())
}

pub fn find_by_id(conn: &Connection, id: i64) -> Result<AccountRecord> {
    conn.query_row(
        "SELECT id, COALESCE(seed_id, ''), network_genesis_hash, COALESCE(ip_identity, 0), COALESCE(identity_index, 0),
                COALESCE(credential_counter, 0), source_kind, imported_vault_id, import_kind, source_metadata_json,
                label, status, transaction_hash, created_at, updated_at
         FROM accounts WHERE id = ?1",
        params![id],
        map_account_row,
    )
    .optional()
    .with_context(|| format!("failed to query account {id}"))?
    .with_context(|| format!("account {id} is not configured"))
}

fn find_by_id_in_tx(tx: &rusqlite::Transaction<'_>, id: i64) -> Result<AccountRecord> {
    tx.query_row(
        "SELECT id, COALESCE(seed_id, ''), network_genesis_hash, COALESCE(ip_identity, 0), COALESCE(identity_index, 0),
                COALESCE(credential_counter, 0), source_kind, imported_vault_id, import_kind, source_metadata_json,
                label, status, transaction_hash, created_at, updated_at
         FROM accounts WHERE id = ?1",
        params![id],
        map_account_row,
    )
    .optional()
    .with_context(|| format!("failed to query account {id}"))?
    .with_context(|| format!("account {id} is not configured"))
}

fn load_imported_vault(
    conn: &Connection,
    network_genesis_hash: &str,
) -> Result<Option<ImportedAccountVault>> {
    conn.query_row(
        "SELECT id, network_genesis_hash, kdf_algorithm, kdf_params_json, salt,
                encrypted_dek, dek_nonce, cipher_version, created_at, updated_at
         FROM imported_account_vaults WHERE network_genesis_hash = ?1",
        params![network_genesis_hash],
        |row| {
            let kdf_params_json: String = row.get(3)?;
            let kdf_params = serde_json::from_str(&kdf_params_json).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
            })?;
            Ok(ImportedAccountVault {
                record: ImportedAccountVaultRecord {
                    id: row.get(0)?,
                    network_genesis_hash: row.get(1)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                },
                kdf_algorithm: row.get(2)?,
                kdf_params,
                salt: row.get(4)?,
                encrypted_dek: row.get(5)?,
                dek_nonce: row.get(6)?,
                cipher_version: row.get(7)?,
            })
        },
    )
    .optional()
    .with_context(|| {
        format!("failed to query imported accounts vault for network '{network_genesis_hash}'")
    })
}

fn unlock_imported_vault_record(
    vault: ImportedAccountVault,
    password: &str,
) -> Result<UnlockedImportedAccountVault> {
    if vault.kdf_algorithm != KDF_ALGORITHM {
        bail!("unsupported KDF algorithm: {}", vault.kdf_algorithm);
    }
    let kek = derive_kek(password, &vault.salt, &vault.kdf_params)?;
    let dek_aad = object_aad(
        &vault.record.id,
        IMPORTED_ACCOUNT_VAULT_DEK_KIND,
        vault.cipher_version,
    );
    let dek = aead_decrypt(&kek, &vault.dek_nonce, &vault.encrypted_dek, &dek_aad)
        .context("failed to decrypt imported accounts vault data encryption key")?;
    Ok(UnlockedImportedAccountVault {
        record: vault.record,
        dek: zeroizing_array_from_slice::<KEY_LEN>(&dek, "imported vault DEK")?,
    })
}

fn decrypt_private_payload_for_record(
    conn: &Connection,
    record: &AccountRecord,
    seed_dek: &[u8; KEY_LEN],
) -> Result<AccountPrivatePayload> {
    conn.query_row(
        "SELECT cipher_version, ciphertext, nonce FROM account_private_payloads WHERE account_id = ?1",
        params![record.id],
        |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, Vec<u8>>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        },
    )
    .optional()
    .with_context(|| format!("failed to query private payload for account {}", record.id))?
    .with_context(|| format!("account {} has no private payload", record.id))
    .and_then(|(cipher_version, ciphertext, nonce)| {
        decrypt_payload_bytes(record, seed_dek, cipher_version, &ciphertext, &nonce)
    })
}

fn decrypt_payload_bytes(
    record: &AccountRecord,
    seed_dek: &[u8; KEY_LEN],
    cipher_version: u32,
    ciphertext: &[u8],
    nonce: &[u8],
) -> Result<AccountPrivatePayload> {
    let aad = account_payload_aad(record, cipher_version);
    let plaintext = aead_decrypt(seed_dek, nonce, ciphertext, &aad).with_context(|| {
        format!(
            "failed to decrypt private payload for account {}",
            record.id
        )
    })?;
    serde_json::from_slice(&plaintext)
        .with_context(|| format!("failed to parse private payload for account {}", record.id))
}

fn decrypt_imported_payload_for_record(
    conn: &Connection,
    record: &AccountRecord,
    vault_dek: &[u8; KEY_LEN],
) -> Result<ImportedAccountSecretPayload> {
    if record.source_kind != AccountSourceKind::Imported {
        bail!("account '{}' is not imported", record.label);
    }
    conn.query_row(
        "SELECT cipher_version, ciphertext, nonce FROM imported_account_payloads WHERE account_id = ?1",
        params![record.id],
        |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, Vec<u8>>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        },
    )
    .optional()
    .with_context(|| format!("failed to query imported payload for account {}", record.id))?
    .with_context(|| format!("account {} has no imported payload", record.id))
    .and_then(|(cipher_version, ciphertext, nonce)| {
        let aad = imported_payload_aad(record, cipher_version)?;
        let plaintext = aead_decrypt(vault_dek, &nonce, &ciphertext, &aad).with_context(|| {
            format!("failed to decrypt imported payload for account {}", record.id)
        })?;
        serde_json::from_slice(&plaintext).with_context(|| {
            format!("failed to parse imported payload for account {}", record.id)
        })
    })
}

fn upsert_private_payload_in_tx(
    tx: &rusqlite::Transaction<'_>,
    record: &AccountRecord,
    seed_dek: &[u8; KEY_LEN],
    payload: &AccountPrivatePayload,
) -> Result<()> {
    let plaintext = Zeroizing::new(serde_json::to_vec(payload).with_context(|| {
        format!(
            "failed to serialise private payload for account {}",
            record.id
        )
    })?);
    let aad = account_payload_aad(record, CIPHER_VERSION);
    let (ciphertext, nonce) = aead_encrypt(seed_dek, &plaintext, &aad).with_context(|| {
        format!(
            "failed to encrypt private payload for account {}",
            record.id
        )
    })?;

    tx.execute(
        "INSERT INTO account_private_payloads (account_id, cipher_version, ciphertext, nonce)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(account_id) DO UPDATE SET
            cipher_version = excluded.cipher_version,
            ciphertext = excluded.ciphertext,
            nonce = excluded.nonce",
        params![record.id, CIPHER_VERSION, ciphertext, nonce.as_slice()],
    )
    .with_context(|| format!("failed to store private payload for account {}", record.id))?;

    Ok(())
}

fn upsert_imported_payload_in_tx(
    tx: &rusqlite::Transaction<'_>,
    record: &AccountRecord,
    vault_dek: &[u8; KEY_LEN],
    payload: &ImportedAccountSecretPayload,
) -> Result<()> {
    let vault_id = record
        .imported_vault_id
        .as_deref()
        .context("imported account has no vault id")?;
    let plaintext = Zeroizing::new(serde_json::to_vec(payload).with_context(|| {
        format!(
            "failed to serialise imported payload for account {}",
            record.id
        )
    })?);
    let aad = imported_payload_aad(record, CIPHER_VERSION)?;
    let (ciphertext, nonce) = aead_encrypt(vault_dek, &plaintext, &aad).with_context(|| {
        format!(
            "failed to encrypt imported payload for account {}",
            record.id
        )
    })?;

    tx.execute(
        "INSERT INTO imported_account_payloads (account_id, vault_id, cipher_version, ciphertext, nonce)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![record.id, vault_id, CIPHER_VERSION, ciphertext, nonce.as_slice()],
    )
    .with_context(|| format!("failed to store imported payload for account {}", record.id))?;
    Ok(())
}

fn account_payload_aad(record: &AccountRecord, cipher_version: u32) -> Vec<u8> {
    object_aad(
        &format!(
            "{}:{}:{}:{}:{}:{}",
            record.id,
            record.network_genesis_hash,
            record.seed_id,
            record.ip_identity,
            record.identity_index,
            record.credential_counter
        ),
        ACCOUNT_PRIVATE_PAYLOAD_KIND,
        cipher_version,
    )
}

fn imported_payload_aad(record: &AccountRecord, cipher_version: u32) -> Result<Vec<u8>> {
    let vault_id = record
        .imported_vault_id
        .as_deref()
        .context("imported account has no vault id")?;
    Ok(object_aad(
        &format!("{}:{}:{}", record.id, record.network_genesis_hash, vault_id),
        IMPORTED_ACCOUNT_PAYLOAD_KIND,
        cipher_version,
    ))
}

fn map_account_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AccountRecord> {
    let source_kind: String = row.get(6)?;
    let status: String = row.get(11)?;
    Ok(AccountRecord {
        id: row.get(0)?,
        seed_id: row.get(1)?,
        network_genesis_hash: row.get(2)?,
        ip_identity: row.get(3)?,
        identity_index: row.get(4)?,
        credential_counter: row.get(5)?,
        source_kind: AccountSourceKind::from_str(&source_kind).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(
                6,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    err.to_string(),
                )),
            )
        })?,
        imported_vault_id: row.get(7)?,
        import_kind: row.get(8)?,
        source_metadata_json: row.get(9)?,
        label: row.get(10)?,
        status: AccountStatus::from_str(&status).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(
                11,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    err.to_string(),
                )),
            )
        })?,
        transaction_hash: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

fn now_unix_seconds() -> Result<i64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?;
    Ok(duration.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{migrations, seeds};
    use rusqlite::Connection;
    use serde_json::json;

    const MAINNET: &str = "mainnet-hash";
    const TESTNET: &str = "testnet-hash";

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    fn seed(conn: &Connection, label: &str) -> (String, Zeroizing<[u8; KEY_LEN]>) {
        let record = seeds::add(conn, label, b"seed secret", "password").unwrap();
        let unlocked = seeds::unlock_context(conn, label, "password").unwrap();
        (record.id, unlocked.dek)
    }

    fn imported_payload(address: &str) -> ImportedAccountSecretPayload {
        ImportedAccountSecretPayload {
            account_address: address.to_owned(),
            account_keys: json!({"keys":{"0":{"keys":{"0":{"signKey":"00","verifyKey":"11"}},"threshold":1}},"threshold":1}),
            credentials: json!({"v":0,"value":{}}),
            encryption_public_key: Some("public".to_owned()),
            encryption_secret_key: Some("secret".to_owned()),
            credential_holder_info: Some(json!({"idCredSecret":"id-secret"})),
            source: ImportedAccountSourceMetadata {
                import_kind: "genesis".to_owned(),
                original_filename: Some("baker-0.json".to_owned()),
            },
        }
    }

    fn pending<'a>(
        seed_id: &'a str,
        label: &'a str,
        credential_counter: u32,
    ) -> PendingAccount<'a> {
        PendingAccount {
            network_genesis_hash: MAINNET,
            seed_id,
            ip_identity: 1,
            identity_index: 0,
            credential_counter,
            label,
        }
    }

    #[test]
    fn insert_pending_stores_metadata_and_allocates_next_counter() {
        let conn = conn();
        let (seed_id, _) = seed(&conn, "seed_a");

        assert_eq!(
            next_credential_counter(&conn, MAINNET, &seed_id, 1, 0).unwrap(),
            0
        );
        let id = insert_pending(&conn, pending(&seed_id, "account-1", 0)).unwrap();
        assert!(id > 0);

        let record = find_by_network_and_label(&conn, MAINNET, "account-1")
            .unwrap()
            .unwrap();
        assert_eq!(record.seed_id, seed_id);
        assert_eq!(record.status, AccountStatus::Pending);
        assert_eq!(record.credential_counter, 0);
        assert_eq!(
            next_credential_counter(&conn, MAINNET, &record.seed_id, 1, 0).unwrap(),
            1
        );
    }

    #[test]
    fn import_recovered_inserts_and_reuses_tuple() {
        let mut conn = conn();
        let (seed_id, dek) = seed(&conn, "seed_a");

        let (record, inserted) = import_recovered(
            &mut conn,
            &dek,
            RecoveredAccount {
                network_genesis_hash: MAINNET,
                seed_id: &seed_id,
                ip_identity: 1,
                identity_index: 0,
                credential_counter: 0,
                label: "account_1",
                account_address: "addr-1",
            },
        )
        .unwrap();
        assert!(inserted);
        assert_eq!(record.status, AccountStatus::Finalized);

        let (updated, inserted_again) = import_recovered(
            &mut conn,
            &dek,
            RecoveredAccount {
                network_genesis_hash: MAINNET,
                seed_id: &seed_id,
                ip_identity: 1,
                identity_index: 0,
                credential_counter: 0,
                label: "account_other",
                account_address: "addr-1b",
            },
        )
        .unwrap();
        assert!(!inserted_again);
        assert_eq!(updated.id, record.id);
        assert_eq!(updated.label, "account_1");

        let payload = decrypt_private_payload(&conn, record.id, &dek).unwrap();
        assert_eq!(payload.account_address, "addr-1b");
    }

    #[test]
    fn next_generated_label_skips_existing_suffixes() {
        let conn = conn();
        let (seed_id, _) = seed(&conn, "seed_a");
        insert_pending(&conn, pending(&seed_id, "account_1", 0)).unwrap();
        insert_pending(&conn, pending(&seed_id, "account_2", 1)).unwrap();

        assert_eq!(
            next_generated_label(&conn, MAINNET, "account").unwrap(),
            "account_3"
        );
    }

    #[test]
    fn duplicate_label_is_rejected_per_network() {
        let conn = conn();
        let (seed_id, _) = seed(&conn, "seed_a");

        insert_pending(&conn, pending(&seed_id, "account", 0)).unwrap();
        let err = insert_pending(&conn, pending(&seed_id, "account", 1)).unwrap_err();
        assert!(err.to_string().contains("account label 'account'"));

        insert_pending(
            &conn,
            PendingAccount {
                network_genesis_hash: TESTNET,
                seed_id: &seed_id,
                ip_identity: 1,
                identity_index: 0,
                credential_counter: 0,
                label: "account",
            },
        )
        .unwrap();
    }

    #[test]
    fn duplicate_derivation_tuple_is_rejected() {
        let conn = conn();
        let (seed_id, _) = seed(&conn, "seed_a");

        insert_pending(&conn, pending(&seed_id, "account-1", 0)).unwrap();
        let err = insert_pending(&conn, pending(&seed_id, "account-2", 0)).unwrap_err();
        assert!(err.to_string().contains("credential counter 0"));
    }

    #[test]
    fn list_returns_all_accounts() {
        let conn = conn();
        let (seed_id, _) = seed(&conn, "seed_a");
        insert_pending(&conn, pending(&seed_id, "account-b", 0)).unwrap();
        insert_pending(
            &conn,
            PendingAccount {
                network_genesis_hash: TESTNET,
                seed_id: &seed_id,
                ip_identity: 1,
                identity_index: 0,
                credential_counter: 1,
                label: "account-a",
            },
        )
        .unwrap();

        let labels = list(&conn)
            .unwrap()
            .into_iter()
            .map(|record| record.label)
            .collect::<Vec<_>>();
        assert_eq!(labels, vec!["account-a".to_owned(), "account-b".to_owned()]);
    }

    #[test]
    fn rename_updates_label_within_network_scope() {
        let conn = conn();
        let (seed_id, _) = seed(&conn, "seed_a");
        let id = insert_pending(&conn, pending(&seed_id, "account", 0)).unwrap();

        rename(&conn, id, "account-renamed").unwrap();

        assert!(
            find_by_network_and_label(&conn, MAINNET, "account")
                .unwrap()
                .is_none()
        );
        assert!(
            find_by_network_and_label(&conn, MAINNET, "account-renamed")
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn rename_rejects_duplicate_label_in_network_scope() {
        let conn = conn();
        let (seed_id, _) = seed(&conn, "seed_a");
        let id = insert_pending(&conn, pending(&seed_id, "account-a", 0)).unwrap();
        insert_pending(&conn, pending(&seed_id, "account-b", 1)).unwrap();

        let err = rename(&conn, id, "account-b").unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn parses_genesis_account_json() {
        let payload = parse_genesis_account_json(
            r#"{
                "accountKeys":{"keys":{"0":{"keys":{"0":{"signKey":"00","verifyKey":"11"}},"threshold":1}},"threshold":1},
                "address":"addr-1",
                "credentials":{"v":0,"value":{}},
                "encryptionPublicKey":"public",
                "encryptionSecretKey":"secret",
                "aci":{"credentialHolderInformation":{"idCredSecret":"id-secret"}}
            }"#,
            Some("baker-0.json".to_owned()),
        )
        .unwrap();
        assert_eq!(payload.account_address, "addr-1");
        assert_eq!(payload.source.import_kind, "genesis");
        assert_eq!(
            payload.source.original_filename.as_deref(),
            Some("baker-0.json")
        );
        assert_eq!(payload.encryption_secret_key.as_deref(), Some("secret"));
    }

    #[test]
    fn malformed_genesis_account_json_errors_actionably() {
        let err = parse_genesis_account_json(r#"{"address":"addr"}"#, None).unwrap_err();
        assert!(err.to_string().contains("missing 'accountKeys'"));

        let err = parse_genesis_account_json(
            r#"{"address":"addr","accountKeys":{"keys":{}},"credentials":{}}"#,
            None,
        )
        .unwrap_err();
        assert!(err.to_string().contains("signing key material"));
    }

    #[test]
    fn imported_vault_is_created_reused_and_rejects_wrong_password() {
        let conn = conn();
        let unlocked = create_or_unlock_imported_vault(&conn, MAINNET, "password").unwrap();
        let reused = create_or_unlock_imported_vault(&conn, MAINNET, "password").unwrap();
        assert_eq!(unlocked.record.id, reused.record.id);

        let count: u32 = conn
            .query_row("SELECT COUNT(*) FROM imported_account_vaults", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
        assert!(unlock_imported_vault(&conn, MAINNET, "wrong").is_err());
    }

    #[test]
    fn imported_account_payload_is_encrypted_and_decrypts() {
        let mut conn = conn();
        let vault = create_or_unlock_imported_vault(&conn, MAINNET, "password").unwrap();
        let payload = imported_payload("addr-imported");

        let record = import_imported_account(
            &mut conn,
            &vault.dek,
            &vault.record,
            ImportedAccount {
                network_genesis_hash: MAINNET,
                label: "baker-0",
                import_kind: "genesis",
                source_metadata_json: Some("{\"file\":\"baker-0.json\"}"),
                payload: &payload,
            },
        )
        .unwrap();

        assert_eq!(record.source_kind, AccountSourceKind::Imported);
        assert_eq!(record.seed_id, "");
        assert_eq!(record.imported_vault_id, Some(vault.record.id.clone()));
        let decrypted = decrypt_imported_payload(&conn, record.id, &vault.dek).unwrap();
        assert_eq!(decrypted.account_address, "addr-imported");
        let raw_count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM imported_account_payloads WHERE CAST(ciphertext AS TEXT) LIKE '%addr-imported%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(raw_count, 0);
    }

    #[test]
    fn imported_account_label_collides_with_derived_label() {
        let mut conn = conn();
        let (seed_id, _) = seed(&conn, "seed_a");
        insert_pending(&conn, pending(&seed_id, "account", 0)).unwrap();
        let vault = create_or_unlock_imported_vault(&conn, MAINNET, "password").unwrap();
        let payload = imported_payload("addr-imported");

        let err = import_imported_account(
            &mut conn,
            &vault.dek,
            &vault.record,
            ImportedAccount {
                network_genesis_hash: MAINNET,
                label: "account",
                import_kind: "genesis",
                source_metadata_json: None,
                payload: &payload,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("account label 'account'"));
    }

    #[test]
    fn deleting_seed_leaves_imported_accounts_but_network_prune_removes_them() {
        let mut conn = conn();
        let (seed_id, dek) = seed(&conn, "seed_a");
        let derived_id = insert_pending(&conn, pending(&seed_id, "derived", 0)).unwrap();
        set_finalized(&mut conn, derived_id, &dek, None, "derived-address").unwrap();
        let vault = create_or_unlock_imported_vault(&conn, MAINNET, "password").unwrap();
        let payload = imported_payload("addr-imported");
        import_imported_account(
            &mut conn,
            &vault.dek,
            &vault.record,
            ImportedAccount {
                network_genesis_hash: MAINNET,
                label: "imported",
                import_kind: "genesis",
                source_metadata_json: None,
                payload: &payload,
            },
        )
        .unwrap();

        seeds::remove(&conn, "seed_a").unwrap();
        let remaining = list(&conn).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].source_kind, AccountSourceKind::Imported);

        assert_eq!(prune_by_network(&conn, MAINNET).unwrap(), 1);
        let payload_count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM imported_account_payloads",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(payload_count, 0);
        let vault_count: u32 = conn
            .query_row("SELECT COUNT(*) FROM imported_account_vaults", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(vault_count, 0);
    }

    #[test]
    fn signing_source_resolves_derived_and_imported_paths() {
        let mut conn = conn();
        let (seed_id, _) = seed(&conn, "seed_a");
        let derived_id = insert_pending(&conn, pending(&seed_id, "derived", 0)).unwrap();
        match resolve_signing_source(&conn, derived_id, None).unwrap() {
            AccountSigningSource::Derived {
                seed_id: resolved_seed,
                credential_counter,
                ..
            } => {
                assert_eq!(resolved_seed, seed_id);
                assert_eq!(credential_counter, 0);
            }
            AccountSigningSource::Imported(_) => panic!("expected derived signing source"),
        }

        let vault = create_or_unlock_imported_vault(&conn, MAINNET, "password").unwrap();
        let payload = imported_payload("addr-imported");
        let imported = import_imported_account(
            &mut conn,
            &vault.dek,
            &vault.record,
            ImportedAccount {
                network_genesis_hash: MAINNET,
                label: "imported",
                import_kind: "genesis",
                source_metadata_json: None,
                payload: &payload,
            },
        )
        .unwrap();
        let err = resolve_signing_source(&conn, imported.id, None).unwrap_err();
        assert!(err.to_string().contains("vault must be unlocked"));
        match resolve_signing_source(&conn, imported.id, Some(&vault.dek)).unwrap() {
            AccountSigningSource::Imported(payload) => {
                assert_eq!(payload.account_address, "addr-imported");
            }
            AccountSigningSource::Derived { .. } => panic!("expected imported signing source"),
        }
    }

    #[test]
    fn finalization_encrypts_structured_payload() {
        let mut conn = conn();
        let (seed_id, dek) = seed(&conn, "seed_a");
        let id = insert_pending(&conn, pending(&seed_id, "account-1", 0)).unwrap();

        set_finalized(
            &mut conn,
            id,
            &dek,
            Some("tx-hash"),
            "4TnQQRj4gspS5JYwGqNRZB2G8d6nVdFyV1pE8bQxG7PwoPpXbM",
        )
        .unwrap();

        let record = find_by_network_and_label(&conn, MAINNET, "account-1")
            .unwrap()
            .unwrap();
        assert_eq!(record.status, AccountStatus::Finalized);
        assert_eq!(record.transaction_hash, Some("tx-hash".to_owned()));

        let plaintext_count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM accounts WHERE transaction_hash = ?1",
                params!["tx-hash"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(plaintext_count, 1);

        let payload = decrypt_private_payload(&conn, id, &dek).unwrap();
        assert_eq!(
            payload.account_address,
            "4TnQQRj4gspS5JYwGqNRZB2G8d6nVdFyV1pE8bQxG7PwoPpXbM"
        );

        let raw_address_count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM account_private_payloads WHERE CAST(ciphertext AS TEXT) LIKE '%4TnQQ%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(raw_address_count, 0);
    }

    #[test]
    fn deleting_seed_cascades_accounts_and_payloads() {
        let mut conn = conn();
        let (seed_id, dek) = seed(&conn, "seed_a");
        let id = insert_pending(&conn, pending(&seed_id, "account-1", 0)).unwrap();
        set_finalized(&mut conn, id, &dek, None, "account-address").unwrap();

        seeds::remove(&conn, "seed_a").unwrap();

        let account_count: u32 = conn
            .query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))
            .unwrap();
        let payload_count: u32 = conn
            .query_row("SELECT COUNT(*) FROM account_private_payloads", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(account_count, 0);
        assert_eq!(payload_count, 0);
    }

    #[test]
    fn pruning_network_cascades_account_payloads() {
        let mut conn = conn();
        let (seed_id, dek) = seed(&conn, "seed_a");
        let id = insert_pending(&conn, pending(&seed_id, "account-1", 0)).unwrap();
        set_finalized(&mut conn, id, &dek, None, "account-address").unwrap();
        insert_pending(
            &conn,
            PendingAccount {
                network_genesis_hash: TESTNET,
                seed_id: &seed_id,
                ip_identity: 1,
                identity_index: 0,
                credential_counter: 1,
                label: "account-2",
            },
        )
        .unwrap();

        assert_eq!(prune_by_network(&conn, MAINNET).unwrap(), 1);
        assert_eq!(
            distinct_network_genesis_hashes(&conn).unwrap(),
            vec![TESTNET.to_owned()]
        );
        let payload_count: u32 = conn
            .query_row("SELECT COUNT(*) FROM account_private_payloads", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(payload_count, 0);
    }
}
