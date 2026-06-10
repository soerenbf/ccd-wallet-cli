use crate::store::crypto::{
    KEY_LEN, aead_decrypt, aead_encrypt, object_aad, zeroizing_array_from_slice,
};
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroizing;

const CIPHER_VERSION: u32 = 1;
const IDENTITY_PRIVATE_PAYLOAD_KIND: &str = "identity_private_payload";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityRecord {
    pub id: i64,
    pub signer_owner_id: String,
    pub network_genesis_hash: String,
    pub ip_identity: u32,
    pub identity_index: u32,
    pub label: String,
    pub status: IdentityStatus,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IdentityPrivatePayload {
    Pending { code_uri: String },
    Done { identity_object: serde_json::Value },
}

impl IdentityPrivatePayload {
    pub fn pending(code_uri: impl Into<String>) -> Self {
        Self::Pending {
            code_uri: code_uri.into(),
        }
    }

    pub fn done(identity_object: serde_json::Value) -> Self {
        Self::Done { identity_object }
    }

    pub fn code_uri(&self) -> Option<&str> {
        match self {
            Self::Pending { code_uri } => Some(code_uri.as_str()),
            Self::Done { .. } => None,
        }
    }

    pub fn identity_object(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Pending { .. } => None,
            Self::Done { identity_object } => Some(identity_object),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityStatus {
    Pending,
    Done,
}

impl IdentityStatus {
    fn as_str(self) -> &'static str {
        match self {
            IdentityStatus::Pending => "pending",
            IdentityStatus::Done => "done",
        }
    }

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "done" => Ok(Self::Done),
            other => bail!("unsupported identity status '{other}'"),
        }
    }
}

pub fn next_index(
    conn: &Connection,
    network_genesis_hash: &str,
    signer_owner_id: &str,
    ip_identity: u32,
) -> Result<u32> {
    let max_index: Option<u32> = conn
        .query_row(
            "SELECT MAX(identity_index) FROM identities WHERE network_genesis_hash = ?1 AND signer_owner_id = ?2 AND ip_identity = ?3",
            params![network_genesis_hash, signer_owner_id, ip_identity],
            |row| row.get(0),
        )
        .context("failed to query next identity index")?;

    Ok(max_index.map(|idx| idx + 1).unwrap_or(0))
}

pub struct PendingIdentity<'a> {
    pub network_genesis_hash: &'a str,
    pub signer_owner_id: &'a str,
    pub ip_identity: u32,
    pub identity_index: u32,
    pub label: &'a str,
    pub code_uri: &'a str,
}

pub fn insert_pending(
    conn: &mut Connection,
    signer_owner_dek: &[u8; KEY_LEN],
    pending: PendingIdentity<'_>,
) -> Result<i64> {
    if find_by_network_and_label(conn, pending.network_genesis_hash, pending.label)?.is_some() {
        bail!(
            "identity label '{}' already exists for network '{}'",
            pending.label,
            pending.network_genesis_hash
        );
    }

    if find_by_network_signer_owner_ip_and_index(
        conn,
        pending.network_genesis_hash,
        pending.signer_owner_id,
        pending.ip_identity,
        pending.identity_index,
    )?
    .is_some()
    {
        bail!(
            "identity index {} for provider {} already exists for signer owner '{}' on network '{}'",
            pending.identity_index,
            pending.ip_identity,
            pending.signer_owner_id,
            pending.network_genesis_hash
        );
    }

    let tx = conn
        .transaction()
        .context("failed to start identity insert transaction")?;
    let created_at = now_unix_seconds()?;
    tx.execute(
        "INSERT INTO identities (
            signer_owner_id, network_genesis_hash, ip_identity, identity_index, label, status, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            pending.signer_owner_id,
            pending.network_genesis_hash,
            pending.ip_identity,
            pending.identity_index,
            pending.label,
            IdentityStatus::Pending.as_str(),
            created_at,
        ],
    )
    .with_context(|| format!("failed to insert identity '{}'", pending.label))?;
    let id = tx.last_insert_rowid();

    let record = IdentityRecord {
        id,
        signer_owner_id: pending.signer_owner_id.to_owned(),
        network_genesis_hash: pending.network_genesis_hash.to_owned(),
        ip_identity: pending.ip_identity,
        identity_index: pending.identity_index,
        label: pending.label.to_owned(),
        status: IdentityStatus::Pending,
        created_at,
        expires_at: None,
    };
    let payload = IdentityPrivatePayload::pending(pending.code_uri);
    upsert_private_payload_in_tx(&tx, &record, signer_owner_dek, &payload)?;
    tx.commit()
        .context("failed to commit identity insert transaction")?;

    Ok(id)
}

pub fn set_done(
    conn: &mut Connection,
    id: i64,
    signer_owner_dek: &[u8; KEY_LEN],
    identity_object: serde_json::Value,
) -> Result<()> {
    let tx = conn
        .transaction()
        .context("failed to start identity update transaction")?;
    let mut record = find_by_id_in_tx(&tx, id)?;
    let expires_at = extract_identity_expires_at(&identity_object);
    let payload = IdentityPrivatePayload::done(identity_object);

    let affected = tx
        .execute(
            "UPDATE identities SET status = ?1, expires_at = ?2 WHERE id = ?3",
            params![IdentityStatus::Done.as_str(), expires_at, id],
        )
        .with_context(|| format!("failed to mark identity {id} done"))?;

    if affected == 0 {
        bail!("identity {id} is not configured");
    }

    record.status = IdentityStatus::Done;
    record.expires_at = expires_at;
    upsert_private_payload_in_tx(&tx, &record, signer_owner_dek, &payload)?;
    tx.commit()
        .context("failed to commit identity done transaction")?;
    Ok(())
}

pub struct RecoveredIdentity<'a> {
    pub network_genesis_hash: &'a str,
    pub signer_owner_id: &'a str,
    pub ip_identity: u32,
    pub identity_index: u32,
    pub label: &'a str,
    pub identity_object: &'a serde_json::Value,
}

pub fn import_recovered(
    conn: &mut Connection,
    signer_owner_dek: &[u8; KEY_LEN],
    recovered: RecoveredIdentity<'_>,
) -> Result<(IdentityRecord, bool)> {
    let tx = conn
        .transaction()
        .context("failed to start recovered identity import transaction")?;

    if let Some(existing) = find_by_network_signer_owner_ip_and_index(
        &tx,
        recovered.network_genesis_hash,
        recovered.signer_owner_id,
        recovered.ip_identity,
        recovered.identity_index,
    )? {
        let payload = IdentityPrivatePayload::done(recovered.identity_object.clone());
        let expires_at = extract_identity_expires_at(recovered.identity_object);

        tx.execute(
            "UPDATE identities SET status = ?1, expires_at = ?2 WHERE id = ?3",
            params![IdentityStatus::Done.as_str(), expires_at, existing.id],
        )
        .with_context(|| format!("failed to update recovered identity {}", existing.id))?;

        let updated = IdentityRecord {
            status: IdentityStatus::Done,
            expires_at,
            ..existing
        };
        upsert_private_payload_in_tx(&tx, &updated, signer_owner_dek, &payload)?;
        tx.commit()
            .context("failed to commit recovered identity update transaction")?;
        return Ok((updated, false));
    }

    if find_by_network_and_label(&tx, recovered.network_genesis_hash, recovered.label)?.is_some() {
        bail!(
            "identity label '{}' already exists for network '{}'",
            recovered.label,
            recovered.network_genesis_hash
        );
    }

    let created_at = now_unix_seconds()?;
    let expires_at = extract_identity_expires_at(recovered.identity_object);
    tx.execute(
        "INSERT INTO identities (
            signer_owner_id, network_genesis_hash, ip_identity, identity_index, label, status, created_at, expires_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            recovered.signer_owner_id,
            recovered.network_genesis_hash,
            recovered.ip_identity,
            recovered.identity_index,
            recovered.label,
            IdentityStatus::Done.as_str(),
            created_at,
            expires_at,
        ],
    )
    .with_context(|| format!("failed to insert recovered identity '{}'", recovered.label))?;

    let record = IdentityRecord {
        id: tx.last_insert_rowid(),
        signer_owner_id: recovered.signer_owner_id.to_owned(),
        network_genesis_hash: recovered.network_genesis_hash.to_owned(),
        ip_identity: recovered.ip_identity,
        identity_index: recovered.identity_index,
        label: recovered.label.to_owned(),
        status: IdentityStatus::Done,
        created_at,
        expires_at,
    };
    let payload = IdentityPrivatePayload::done(recovered.identity_object.clone());
    upsert_private_payload_in_tx(&tx, &record, signer_owner_dek, &payload)?;
    tx.commit()
        .context("failed to commit recovered identity insert transaction")?;
    Ok((record, true))
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

pub fn delete(conn: &Connection, id: i64) -> Result<()> {
    let affected = conn
        .execute("DELETE FROM identities WHERE id = ?1", params![id])
        .with_context(|| format!("failed to delete identity {id}"))?;

    if affected == 0 {
        bail!("identity {id} is not configured");
    }

    Ok(())
}

pub fn prune_by_network(conn: &Connection, network_genesis_hash: &str) -> Result<usize> {
    conn.execute(
        "DELETE FROM identities WHERE network_genesis_hash = ?1",
        params![network_genesis_hash],
    )
    .with_context(|| format!("failed to prune identities for network '{network_genesis_hash}'"))
}

pub fn distinct_network_genesis_hashes(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT network_genesis_hash FROM identities ORDER BY network_genesis_hash",
        )
        .context("failed to prepare distinct identity network hash query")?;
    let rows = stmt
        .query_map([], |row| row.get(0))
        .context("failed to query distinct identity network hashes")?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read distinct identity network hashes")
}

pub fn list(conn: &Connection) -> Result<Vec<IdentityRecord>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, signer_owner_id, network_genesis_hash, ip_identity, identity_index, label, status, created_at, expires_at
             FROM identities ORDER BY label",
        )
        .context("failed to prepare identity list query")?;

    let rows = stmt
        .query_map([], map_identity_row)
        .context("failed to query identities")?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read identity rows")
}

pub fn list_by_network_and_signer_owner(
    conn: &Connection,
    network_genesis_hash: &str,
    signer_owner_id: &str,
) -> Result<Vec<IdentityRecord>> {
    Ok(list(conn)?
        .into_iter()
        .filter(|record| {
            record.network_genesis_hash == network_genesis_hash
                && record.signer_owner_id == signer_owner_id
        })
        .collect())
}

pub fn find_by_network_signer_owner_and_label(
    conn: &Connection,
    network_genesis_hash: &str,
    signer_owner_id: &str,
    label: &str,
) -> Result<Option<IdentityRecord>> {
    conn.query_row(
        "SELECT id, signer_owner_id, network_genesis_hash, ip_identity, identity_index, label, status, created_at, expires_at
         FROM identities WHERE network_genesis_hash = ?1 AND signer_owner_id = ?2 AND label = ?3",
        params![network_genesis_hash, signer_owner_id, label],
        map_identity_row,
    )
    .optional()
    .with_context(|| {
        format!(
            "failed to query identity '{label}' for signer owner '{signer_owner_id}' on network '{network_genesis_hash}'"
        )
    })
}

pub fn find_by_network_and_label(
    conn: &Connection,
    network_genesis_hash: &str,
    label: &str,
) -> Result<Option<IdentityRecord>> {
    conn.query_row(
        "SELECT id, signer_owner_id, network_genesis_hash, ip_identity, identity_index, label, status, created_at, expires_at
         FROM identities WHERE network_genesis_hash = ?1 AND label = ?2",
        params![network_genesis_hash, label],
        map_identity_row,
    )
    .optional()
    .with_context(|| {
        format!(
            "failed to query identity '{label}' for network '{network_genesis_hash}'"
        )
    })
}

pub fn find_by_network_signer_owner_ip_and_index(
    conn: &Connection,
    network_genesis_hash: &str,
    signer_owner_id: &str,
    ip_identity: u32,
    identity_index: u32,
) -> Result<Option<IdentityRecord>> {
    conn.query_row(
        "SELECT id, signer_owner_id, network_genesis_hash, ip_identity, identity_index, label, status, created_at, expires_at
         FROM identities WHERE network_genesis_hash = ?1 AND signer_owner_id = ?2 AND ip_identity = ?3 AND identity_index = ?4",
        params![network_genesis_hash, signer_owner_id, ip_identity, identity_index],
        map_identity_row,
    )
    .optional()
    .with_context(|| {
        format!(
            "failed to query identity index {identity_index} for signer owner '{signer_owner_id}', provider {ip_identity}, network '{network_genesis_hash}'"
        )
    })
}

pub fn decrypt_private_payload(
    conn: &Connection,
    id: i64,
    signer_owner_dek: &[u8; KEY_LEN],
) -> Result<IdentityPrivatePayload> {
    let record = find_by_id(conn, id)?;
    decrypt_private_payload_for_record(conn, &record, signer_owner_dek)
}

pub fn rename(conn: &Connection, id: i64, new_label: &str) -> Result<()> {
    let record = find_by_id(conn, id)?;
    if record.label == new_label {
        return Ok(());
    }
    if find_by_network_and_label(conn, &record.network_genesis_hash, new_label)?.is_some() {
        bail!(
            "identity label '{}' already exists for network '{}'",
            new_label,
            record.network_genesis_hash
        );
    }
    let affected = conn
        .execute(
            "UPDATE identities SET label = ?1 WHERE id = ?2",
            params![new_label, id],
        )
        .with_context(|| format!("failed to rename identity {id} to '{new_label}'"))?;
    if affected == 0 {
        bail!("identity {id} is not configured");
    }
    Ok(())
}

pub fn find_by_id(conn: &Connection, id: i64) -> Result<IdentityRecord> {
    conn.query_row(
        "SELECT id, signer_owner_id, network_genesis_hash, ip_identity, identity_index, label, status, created_at, expires_at
         FROM identities WHERE id = ?1",
        params![id],
        map_identity_row,
    )
    .optional()
    .with_context(|| format!("failed to query identity {id}"))?
    .with_context(|| format!("identity {id} is not configured"))
}

fn find_by_id_in_tx(tx: &rusqlite::Transaction<'_>, id: i64) -> Result<IdentityRecord> {
    tx.query_row(
        "SELECT id, signer_owner_id, network_genesis_hash, ip_identity, identity_index, label, status, created_at, expires_at
         FROM identities WHERE id = ?1",
        params![id],
        map_identity_row,
    )
    .optional()
    .with_context(|| format!("failed to query identity {id}"))?
    .with_context(|| format!("identity {id} is not configured"))
}

fn decrypt_private_payload_for_record(
    conn: &Connection,
    record: &IdentityRecord,
    signer_owner_dek: &[u8; KEY_LEN],
) -> Result<IdentityPrivatePayload> {
    conn.query_row(
        "SELECT cipher_version, ciphertext, nonce FROM identity_private_payloads WHERE identity_id = ?1",
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
    .with_context(|| format!("failed to query private payload for identity {}", record.id))?
    .with_context(|| format!("identity {} has no private payload", record.id))
    .and_then(|(cipher_version, ciphertext, nonce)| {
        decrypt_payload_bytes(record, signer_owner_dek, cipher_version, &ciphertext, &nonce)
    })
}

fn decrypt_payload_bytes(
    record: &IdentityRecord,
    signer_owner_dek: &[u8; KEY_LEN],
    cipher_version: u32,
    ciphertext: &[u8],
    nonce: &[u8],
) -> Result<IdentityPrivatePayload> {
    let aad = identity_payload_aad(record, cipher_version);
    let plaintext = aead_decrypt(signer_owner_dek, nonce, ciphertext, &aad).with_context(|| {
        format!(
            "failed to decrypt private payload for identity {}",
            record.id
        )
    })?;
    serde_json::from_slice(&plaintext)
        .with_context(|| format!("failed to parse private payload for identity {}", record.id))
}

fn upsert_private_payload_in_tx(
    tx: &rusqlite::Transaction<'_>,
    record: &IdentityRecord,
    signer_owner_dek: &[u8; KEY_LEN],
    payload: &IdentityPrivatePayload,
) -> Result<()> {
    let plaintext = Zeroizing::new(serde_json::to_vec(payload).with_context(|| {
        format!(
            "failed to serialise private payload for identity {}",
            record.id
        )
    })?);
    let aad = identity_payload_aad(record, CIPHER_VERSION);
    let (ciphertext, nonce) =
        aead_encrypt(signer_owner_dek, &plaintext, &aad).with_context(|| {
            format!(
                "failed to encrypt private payload for identity {}",
                record.id
            )
        })?;

    tx.execute(
        "INSERT INTO identity_private_payloads (identity_id, cipher_version, ciphertext, nonce)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(identity_id) DO UPDATE SET
            cipher_version = excluded.cipher_version,
            ciphertext = excluded.ciphertext,
            nonce = excluded.nonce",
        params![record.id, CIPHER_VERSION, ciphertext, nonce.as_slice()],
    )
    .with_context(|| format!("failed to store private payload for identity {}", record.id))?;

    Ok(())
}

fn identity_payload_aad(record: &IdentityRecord, cipher_version: u32) -> Vec<u8> {
    object_aad(
        &format!(
            "{}:{}:{}:{}:{}",
            record.id,
            record.network_genesis_hash,
            record.signer_owner_id,
            record.ip_identity,
            record.identity_index
        ),
        IDENTITY_PRIVATE_PAYLOAD_KIND,
        cipher_version,
    )
}

fn map_identity_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<IdentityRecord> {
    let status: String = row.get(6)?;
    Ok(IdentityRecord {
        id: row.get(0)?,
        signer_owner_id: row.get(1)?,
        network_genesis_hash: row.get(2)?,
        ip_identity: row.get(3)?,
        identity_index: row.get(4)?,
        label: row.get(5)?,
        status: IdentityStatus::from_str(&status).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(
                6,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    err.to_string(),
                )),
            )
        })?,
        created_at: row.get(7)?,
        expires_at: row.get(8)?,
    })
}

pub fn extract_identity_expires_at(identity_token: &serde_json::Value) -> Option<i64> {
    find_valid_to(identity_token).and_then(valid_to_to_unix_expiry)
}

fn find_valid_to(value: &serde_json::Value) -> Option<&str> {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(value) = map.get("validTo").and_then(serde_json::Value::as_str) {
                return Some(value);
            }
            map.values().find_map(find_valid_to)
        }
        serde_json::Value::Array(values) => values.iter().find_map(find_valid_to),
        _ => None,
    }
}

fn valid_to_to_unix_expiry(valid_to: &str) -> Option<i64> {
    if valid_to.len() != 6 || !valid_to.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let year = valid_to[..4].parse::<i32>().ok()?;
    let month = valid_to[4..].parse::<u32>().ok()?;
    if !(1..=12).contains(&month) {
        return None;
    }

    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    Some(days_from_civil(next_year, next_month, 1) * 86_400)
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = month as i32;
    let day = day as i32;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    i64::from(era * 146_097 + doe - 719_468)
}

fn now_unix_seconds() -> Result<i64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?;
    Ok(duration.as_secs() as i64)
}

pub fn test_key(byte: u8) -> Zeroizing<[u8; KEY_LEN]> {
    zeroizing_array_from_slice::<KEY_LEN>(&[byte; KEY_LEN], "test key").unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{
        migrations, seeds,
        signer_owners::{self, SignerOwnerKind},
    };

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

    fn ledger_owner(conn: &Connection, label: &str) -> (String, Zeroizing<[u8; KEY_LEN]>) {
        let owner = signer_owners::create(conn, SignerOwnerKind::Ledger, label).unwrap();
        let dek = signer_owners::create_vault(conn, &owner.id, "password").unwrap();
        signer_owners::insert_ledger_details(
            conn,
            signer_owners::NewLedgerOwnerDetails {
                signer_owner_id: &owner.id,
                canonical_public_key: &[7; 32],
                fingerprint: "ledgerfp",
                enrollment_path: signer_owners::LEDGER_OWNER_ENROLLMENT_PATH,
                app_name: Some("Concordium"),
            },
        )
        .unwrap();
        (owner.id, dek)
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_pending(
        conn: &mut Connection,
        signer_owner_dek: &[u8; KEY_LEN],
        network_genesis_hash: &str,
        signer_owner_id: &str,
        ip_identity: u32,
        identity_index: u32,
        label: &str,
        code_uri: &str,
    ) -> Result<i64> {
        super::insert_pending(
            conn,
            signer_owner_dek,
            PendingIdentity {
                network_genesis_hash,
                signer_owner_id,
                ip_identity,
                identity_index,
                label,
                code_uri,
            },
        )
    }

    #[test]
    fn next_index_starts_at_zero_and_increments_per_network_seed_and_provider() {
        let mut conn = conn();
        let (seed_a, key_a) = seed(&conn, "seed_a");
        let (seed_b, _) = seed(&conn, "seed_b");
        assert_eq!(next_index(&conn, MAINNET, &seed_a, 7).unwrap(), 0);

        let id = insert_pending(
            &mut conn,
            &key_a,
            MAINNET,
            &seed_a,
            7,
            0,
            "id-1",
            "https://code",
        )
        .unwrap();
        assert!(id > 0);
        assert_eq!(next_index(&conn, MAINNET, &seed_a, 7).unwrap(), 1);
        assert_eq!(next_index(&conn, MAINNET, &seed_a, 8).unwrap(), 0);
        assert_eq!(next_index(&conn, MAINNET, &seed_b, 7).unwrap(), 0);
        assert_eq!(next_index(&conn, TESTNET, &seed_a, 7).unwrap(), 0);
    }

    #[test]
    fn duplicate_network_label_pair_is_rejected() {
        let mut conn = conn();
        let (seed_a, key_a) = seed(&conn, "seed_a");
        let (seed_b, key_b) = seed(&conn, "seed_b");
        insert_pending(
            &mut conn,
            &key_a,
            MAINNET,
            &seed_a,
            7,
            0,
            "identity",
            "https://code-1",
        )
        .unwrap();

        let err = insert_pending(
            &mut conn,
            &key_b,
            MAINNET,
            &seed_b,
            8,
            0,
            "identity",
            "https://code-2",
        )
        .unwrap_err();
        assert!(err.to_string().contains("already exists"));

        insert_pending(
            &mut conn,
            &key_b,
            TESTNET,
            &seed_b,
            8,
            0,
            "identity",
            "https://code-3",
        )
        .unwrap();
    }

    #[test]
    fn duplicate_network_seed_ip_index_tuple_is_rejected() {
        let mut conn = conn();
        let (seed_a, key_a) = seed(&conn, "seed_a");
        insert_pending(
            &mut conn,
            &key_a,
            MAINNET,
            &seed_a,
            7,
            0,
            "identity-1",
            "https://code-1",
        )
        .unwrap();

        let err = insert_pending(
            &mut conn,
            &key_a,
            MAINNET,
            &seed_a,
            7,
            0,
            "identity-2",
            "https://code-2",
        )
        .unwrap_err();
        assert!(err.to_string().contains("identity index 0"));
    }

    #[test]
    fn import_recovered_inserts_and_reuses_tuple() {
        let mut conn = conn();
        let (signer_owner_id, dek) = seed(&conn, "seed_a");
        let identity_object = serde_json::json!({"value": {"validTo": "2026-05-15T00:00:00Z"}});

        let (record, inserted) = import_recovered(
            &mut conn,
            &dek,
            RecoveredIdentity {
                network_genesis_hash: MAINNET,
                signer_owner_id: &signer_owner_id,
                ip_identity: 7,
                identity_index: 0,
                label: "identity_1",
                identity_object: &identity_object,
            },
        )
        .unwrap();
        assert!(inserted);
        assert_eq!(record.status, IdentityStatus::Done);

        let (updated, inserted_again) = import_recovered(
            &mut conn,
            &dek,
            RecoveredIdentity {
                network_genesis_hash: MAINNET,
                signer_owner_id: &signer_owner_id,
                ip_identity: 7,
                identity_index: 0,
                label: "identity_other",
                identity_object: &identity_object,
            },
        )
        .unwrap();
        assert!(!inserted_again);
        assert_eq!(updated.id, record.id);
        assert_eq!(updated.label, "identity_1");

        let payload = decrypt_private_payload(&conn, record.id, &dek).unwrap();
        assert_eq!(payload.identity_object(), Some(&identity_object));
    }

    #[test]
    fn next_generated_label_skips_existing_suffixes() {
        let mut conn = conn();
        let (signer_owner_id, dek) = seed(&conn, "seed_a");
        insert_pending(
            &mut conn,
            &dek,
            MAINNET,
            &signer_owner_id,
            7,
            0,
            "identity_1",
            "https://code",
        )
        .unwrap();
        insert_pending(
            &mut conn,
            &dek,
            MAINNET,
            &signer_owner_id,
            7,
            1,
            "identity_2",
            "https://code-2",
        )
        .unwrap();

        assert_eq!(
            next_generated_label(&conn, MAINNET, "identity").unwrap(),
            "identity_3"
        );
    }

    #[test]
    fn ledger_owned_identity_uses_independent_signer_owner_tuple_and_payload_domain() {
        let mut conn = conn();
        let (seed_owner, seed_key) = seed(&conn, "seed_a");
        let (ledger_owner, ledger_key) = ledger_owner(&conn, "ledger_a");

        insert_pending(
            &mut conn,
            &seed_key,
            MAINNET,
            &seed_owner,
            7,
            0,
            "seed-identity",
            "https://seed-code",
        )
        .unwrap();
        let ledger_id = insert_pending(
            &mut conn,
            &ledger_key,
            MAINNET,
            &ledger_owner,
            7,
            0,
            "ledger-identity",
            "https://ledger-code",
        )
        .unwrap();

        let ledger_payload = decrypt_private_payload(&conn, ledger_id, &ledger_key).unwrap();
        assert_eq!(ledger_payload.code_uri(), Some("https://ledger-code"));
        assert!(decrypt_private_payload(&conn, ledger_id, &seed_key).is_err());
        assert_eq!(next_index(&conn, MAINNET, &seed_owner, 7).unwrap(), 1);
        assert_eq!(next_index(&conn, MAINNET, &ledger_owner, 7).unwrap(), 1);
    }

    #[test]
    fn private_payload_is_encrypted_and_decrypts_with_correct_key() {
        let mut conn = conn();
        let (signer_owner_id, key) = seed(&conn, "seed_a");
        let id = insert_pending(
            &mut conn,
            &key,
            MAINNET,
            &signer_owner_id,
            7,
            0,
            "identity",
            "https://code",
        )
        .unwrap();

        let raw: Vec<u8> = conn
            .query_row(
                "SELECT ciphertext FROM identity_private_payloads WHERE identity_id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!String::from_utf8_lossy(&raw).contains("https://code"));

        let payload = decrypt_private_payload(&conn, id, &key).unwrap();
        assert_eq!(payload.code_uri(), Some("https://code"));
        assert!(payload.identity_object().is_none());

        let wrong_key = test_key(9);
        assert!(decrypt_private_payload(&conn, id, &wrong_key).is_err());
    }

    #[test]
    fn aad_mismatch_fails_for_transplanted_payload() {
        let mut conn = conn();
        let (signer_owner_id, key) = seed(&conn, "seed_a");
        let id_1 = insert_pending(
            &mut conn,
            &key,
            MAINNET,
            &signer_owner_id,
            7,
            0,
            "identity-1",
            "https://code-1",
        )
        .unwrap();
        let id_2 = insert_pending(
            &mut conn,
            &key,
            MAINNET,
            &signer_owner_id,
            7,
            1,
            "identity-2",
            "https://code-2",
        )
        .unwrap();

        let (ciphertext, nonce): (Vec<u8>, Vec<u8>) = conn
            .query_row(
                "SELECT ciphertext, nonce FROM identity_private_payloads WHERE identity_id = ?1",
                params![id_1],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        conn.execute(
            "UPDATE identity_private_payloads SET ciphertext = ?1, nonce = ?2 WHERE identity_id = ?3",
            params![ciphertext, nonce, id_2],
        )
        .unwrap();

        assert!(decrypt_private_payload(&conn, id_2, &key).is_err());
    }

    #[test]
    fn list_returns_all_identities() {
        let mut conn = conn();
        let (signer_owner_id, key) = seed(&conn, "seed_a");
        insert_pending(
            &mut conn,
            &key,
            MAINNET,
            &signer_owner_id,
            7,
            0,
            "identity-b",
            "https://code-1",
        )
        .unwrap();
        let (signer_owner_id2, key2) = seed(&conn, "seed_b");
        insert_pending(
            &mut conn,
            &key2,
            TESTNET,
            &signer_owner_id2,
            8,
            0,
            "identity-a",
            "https://code-2",
        )
        .unwrap();

        let labels = list(&conn)
            .unwrap()
            .into_iter()
            .map(|record| record.label)
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            vec!["identity-a".to_owned(), "identity-b".to_owned()]
        );
    }

    #[test]
    fn rename_updates_label_within_network_scope() {
        let mut conn = conn();
        let (signer_owner_id, key) = seed(&conn, "seed_a");
        let record_id = insert_pending(
            &mut conn,
            &key,
            MAINNET,
            &signer_owner_id,
            7,
            0,
            "identity",
            "https://code",
        )
        .unwrap();

        rename(&conn, record_id, "identity-renamed").unwrap();

        assert!(
            find_by_network_and_label(&conn, MAINNET, "identity")
                .unwrap()
                .is_none()
        );
        assert!(
            find_by_network_and_label(&conn, MAINNET, "identity-renamed")
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn rename_rejects_duplicate_label_in_network_scope() {
        let mut conn = conn();
        let (signer_owner_id, key) = seed(&conn, "seed_a");
        let record_a = insert_pending(
            &mut conn,
            &key,
            MAINNET,
            &signer_owner_id,
            7,
            0,
            "identity-a",
            "https://code-a",
        )
        .unwrap();
        insert_pending(
            &mut conn,
            &key,
            MAINNET,
            &signer_owner_id,
            7,
            1,
            "identity-b",
            "https://code-b",
        )
        .unwrap();

        let err = rename(&conn, record_a, "identity-b").unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn status_transitions_to_done_and_provider_error_deletes_identity() {
        let mut conn = conn();
        let (signer_owner_id, key) = seed(&conn, "seed_a");
        let id = insert_pending(
            &mut conn,
            &key,
            MAINNET,
            &signer_owner_id,
            7,
            0,
            "identity",
            "https://code",
        )
        .unwrap();

        set_done(
            &mut conn,
            id,
            &key,
            serde_json::json!({"identityObject": {"name": "Alice"}}),
        )
        .unwrap();
        let record = find_by_network_and_label(&conn, MAINNET, "identity")
            .unwrap()
            .unwrap();
        assert_eq!(record.status, IdentityStatus::Done);
        let payload = decrypt_private_payload(&conn, id, &key).unwrap();
        assert_eq!(payload.code_uri(), None);
        assert_eq!(
            payload.identity_object(),
            Some(&serde_json::json!({"identityObject": {"name": "Alice"}}))
        );

        delete(&conn, id).unwrap();
        assert!(
            find_by_network_and_label(&conn, MAINNET, "identity")
                .unwrap()
                .is_none()
        );
        let payload_count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM identity_private_payloads WHERE identity_id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(payload_count, 0);
    }

    #[test]
    fn removing_seed_cascades_identity_and_private_payload() {
        let mut conn = conn();
        let (signer_owner_id, key) = seed(&conn, "seed_a");
        let id = insert_pending(
            &mut conn,
            &key,
            MAINNET,
            &signer_owner_id,
            7,
            0,
            "identity",
            "https://code",
        )
        .unwrap();

        seeds::remove(&conn, "seed_a").unwrap();

        let identity_count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM identities WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap();
        let payload_count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM identity_private_payloads WHERE identity_id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(identity_count, 0);
        assert_eq!(payload_count, 0);
    }

    #[test]
    fn pruning_network_cascades_private_payloads() {
        let mut conn = conn();
        let (signer_owner_id, key) = seed(&conn, "seed_a");
        let id = insert_pending(
            &mut conn,
            &key,
            MAINNET,
            &signer_owner_id,
            7,
            0,
            "identity",
            "https://code",
        )
        .unwrap();
        insert_pending(
            &mut conn,
            &key,
            TESTNET,
            &signer_owner_id,
            7,
            1,
            "identity-testnet",
            "https://code-2",
        )
        .unwrap();

        assert_eq!(prune_by_network(&conn, MAINNET).unwrap(), 1);
        let identity_count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM identities WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(identity_count, 0);
        assert_eq!(
            distinct_network_genesis_hashes(&conn).unwrap(),
            vec![TESTNET.to_owned()]
        );
        let payload_count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM identity_private_payloads",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(payload_count, 1);
    }
}
