use crate::store::crypto::{KEY_LEN, aead_decrypt, aead_encrypt, object_aad};
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroizing;

const CIPHER_VERSION: u32 = 1;
const ACCOUNT_PRIVATE_PAYLOAD_KIND: &str = "account_private_payload";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRecord {
    pub id: i64,
    pub seed_id: String,
    pub network_genesis_hash: String,
    pub ip_identity: u32,
    pub identity_index: u32,
    pub credential_counter: u32,
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
            label, status, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            pending.seed_id,
            pending.network_genesis_hash,
            pending.ip_identity,
            pending.identity_index,
            pending.credential_counter,
            pending.label,
            AccountStatus::Pending.as_str(),
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

pub fn list(conn: &Connection) -> Result<Vec<AccountRecord>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, seed_id, network_genesis_hash, ip_identity, identity_index,
                    credential_counter, label, status, transaction_hash, created_at, updated_at
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
        "SELECT id, seed_id, network_genesis_hash, ip_identity, identity_index,
                credential_counter, label, status, transaction_hash, created_at, updated_at
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
        "SELECT id, seed_id, network_genesis_hash, ip_identity, identity_index,
                credential_counter, label, status, transaction_hash, created_at, updated_at
         FROM accounts
         WHERE network_genesis_hash = ?1 AND seed_id = ?2 AND ip_identity = ?3
           AND identity_index = ?4 AND credential_counter = ?5",
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
             WHERE network_genesis_hash = ?1 AND seed_id = ?2 AND ip_identity = ?3 AND identity_index = ?4",
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
        "SELECT id, seed_id, network_genesis_hash, ip_identity, identity_index,
                credential_counter, label, status, transaction_hash, created_at, updated_at
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
        "SELECT id, seed_id, network_genesis_hash, ip_identity, identity_index,
                credential_counter, label, status, transaction_hash, created_at, updated_at
         FROM accounts WHERE id = ?1",
        params![id],
        map_account_row,
    )
    .optional()
    .with_context(|| format!("failed to query account {id}"))?
    .with_context(|| format!("account {id} is not configured"))
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

fn map_account_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AccountRecord> {
    let status: String = row.get(7)?;
    Ok(AccountRecord {
        id: row.get(0)?,
        seed_id: row.get(1)?,
        network_genesis_hash: row.get(2)?,
        ip_identity: row.get(3)?,
        identity_index: row.get(4)?,
        credential_counter: row.get(5)?,
        label: row.get(6)?,
        status: AccountStatus::from_str(&status).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(
                7,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    err.to_string(),
                )),
            )
        })?,
        transaction_hash: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
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
}
