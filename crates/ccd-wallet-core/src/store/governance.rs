use crate::store::crypto::{
    Argon2Params, KEY_LEN, aead_decrypt, aead_encrypt, derive_kek, generate_dek, object_aad,
    random_salt, zeroizing_array_from_slice,
};
use anyhow::{Context, Result, bail};
use concordium_rust_sdk::base::base::{UpdateKeyPair, UpdatePublicKey};
use rusqlite::{Connection, OptionalExtension, params};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use zeroize::Zeroizing;

const KDF_ALGORITHM: &str = "argon2id";
const CIPHER_VERSION: u32 = 1;
const GOVERNANCE_VAULT_DEK_KIND: &str = "governance_vault_dek";
const GOVERNANCE_KEY_PAYLOAD_KIND: &str = "governance_key_payload";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernanceKeyRecord {
    pub id: i64,
    pub network_genesis_hash: String,
    pub vault_id: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernanceVaultRecord {
    pub id: String,
    pub network_genesis_hash: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug)]
pub struct UnlockedGovernanceVault {
    pub record: GovernanceVaultRecord,
    pub dek: Zeroizing<[u8; KEY_LEN]>,
}

#[derive(Debug, Clone)]
pub struct DecryptedGovernanceKey {
    pub record: GovernanceKeyRecord,
    pub raw_json: String,
    pub key_pair: UpdateKeyPair,
    pub public_key: UpdatePublicKey,
}

#[derive(Debug)]
struct GovernanceVault {
    record: GovernanceVaultRecord,
    kdf_algorithm: String,
    kdf_params: Argon2Params,
    salt: Vec<u8>,
    encrypted_dek: Vec<u8>,
    dek_nonce: Vec<u8>,
    cipher_version: u32,
}

pub fn governance_vault_exists(conn: &Connection, network_genesis_hash: &str) -> Result<bool> {
    Ok(load_vault(conn, network_genesis_hash)?.is_some())
}

pub fn create_or_unlock_vault(
    conn: &Connection,
    network_genesis_hash: &str,
    password: &str,
) -> Result<UnlockedGovernanceVault> {
    if let Some(vault) = load_vault(conn, network_genesis_hash)? {
        return unlock_vault_record(vault, password);
    }

    let id = Uuid::new_v4().to_string();
    let now = now_unix_seconds()?;
    let params = Argon2Params::default();
    let salt = random_salt();
    let dek = generate_dek();
    let kek = derive_kek(password, &salt, &params)?;
    let dek_aad = object_aad(&id, GOVERNANCE_VAULT_DEK_KIND, CIPHER_VERSION);
    let (encrypted_dek, dek_nonce) = aead_encrypt(&kek, &*dek, &dek_aad)?;
    let kdf_params_json = serde_json::to_string(&params)
        .context("failed to serialise governance vault KDF params")?;

    conn.execute(
        "INSERT INTO governance_key_vaults (
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
        format!("failed to create governance key vault for network '{network_genesis_hash}'")
    })?;

    Ok(UnlockedGovernanceVault {
        record: GovernanceVaultRecord {
            id,
            network_genesis_hash: network_genesis_hash.to_owned(),
            created_at: now,
            updated_at: now,
        },
        dek,
    })
}

pub fn unlock_vault(
    conn: &Connection,
    network_genesis_hash: &str,
    password: &str,
) -> Result<UnlockedGovernanceVault> {
    let vault = load_vault(conn, network_genesis_hash)?.with_context(|| {
        format!("governance key vault for network '{network_genesis_hash}' is not configured")
    })?;
    unlock_vault_record(vault, password)
}

pub fn import_key_json(
    conn: &mut Connection,
    vault: &GovernanceVaultRecord,
    vault_dek: &[u8; KEY_LEN],
    raw_json: &str,
) -> Result<GovernanceKeyRecord> {
    let key_pair = parse_key_pair_json(raw_json)?;
    let public_key = UpdatePublicKey::from(&key_pair);

    if decrypted_keys(conn, &vault.network_genesis_hash, vault_dek)?
        .into_iter()
        .any(|existing| existing.public_key == public_key)
    {
        bail!(
            "governance key '{}' already exists for network '{}'",
            public_key_hex(&public_key),
            vault.network_genesis_hash
        );
    }

    let tx = conn
        .transaction()
        .context("failed to start governance key import transaction")?;
    let now = now_unix_seconds()?;
    tx.execute(
        "INSERT INTO governance_keys (network_genesis_hash, vault_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![vault.network_genesis_hash, vault.id, now, now],
    )
    .with_context(|| {
        format!(
            "failed to insert governance key for network '{}'",
            vault.network_genesis_hash
        )
    })?;
    let record = GovernanceKeyRecord {
        id: tx.last_insert_rowid(),
        network_genesis_hash: vault.network_genesis_hash.clone(),
        vault_id: vault.id.clone(),
        created_at: now,
        updated_at: now,
    };
    upsert_payload_in_tx(&tx, &record, vault_dek, raw_json)?;
    tx.commit()
        .context("failed to commit governance key import transaction")?;
    Ok(record)
}

pub fn decrypted_keys(
    conn: &Connection,
    network_genesis_hash: &str,
    vault_dek: &[u8; KEY_LEN],
) -> Result<Vec<DecryptedGovernanceKey>> {
    list(conn)?
        .into_iter()
        .filter(|record| record.network_genesis_hash == network_genesis_hash)
        .map(|record| {
            let raw_json = decrypt_payload(conn, &record, vault_dek)?;
            let key_pair = parse_key_pair_json(&raw_json)?;
            let public_key = UpdatePublicKey::from(&key_pair);
            Ok(DecryptedGovernanceKey {
                record,
                raw_json,
                key_pair,
                public_key,
            })
        })
        .collect()
}

pub fn remove_by_verify_key(
    conn: &Connection,
    network_genesis_hash: &str,
    vault_dek: &[u8; KEY_LEN],
    verify_key: &str,
) -> Result<bool> {
    let normalized = verify_key.trim().to_ascii_lowercase();
    let match_id = decrypted_keys(conn, network_genesis_hash, vault_dek)?
        .into_iter()
        .find(|entry| public_key_hex(&entry.public_key) == normalized)
        .map(|entry| entry.record.id);

    let Some(id) = match_id else {
        return Ok(false);
    };
    conn.execute("DELETE FROM governance_keys WHERE id = ?1", params![id])
        .with_context(|| format!("failed to delete governance key {id}"))?;
    delete_empty_vault(conn, network_genesis_hash)?;
    Ok(true)
}

pub fn remove_all(conn: &Connection, network_genesis_hash: &str) -> Result<usize> {
    let deleted = conn
        .execute(
            "DELETE FROM governance_keys WHERE network_genesis_hash = ?1",
            params![network_genesis_hash],
        )
        .with_context(|| {
            format!("failed to delete governance keys for network '{network_genesis_hash}'")
        })?;
    delete_empty_vault(conn, network_genesis_hash)?;
    Ok(deleted)
}

pub fn prune_by_network(conn: &Connection, network_genesis_hash: &str) -> Result<usize> {
    let deleted = remove_all(conn, network_genesis_hash)?;
    conn.execute(
        "DELETE FROM governance_key_vaults WHERE network_genesis_hash = ?1",
        params![network_genesis_hash],
    )
    .with_context(|| {
        format!("failed to prune governance key vault for network '{network_genesis_hash}'")
    })?;
    Ok(deleted)
}

pub fn distinct_network_genesis_hashes(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT network_genesis_hash FROM governance_keys ORDER BY network_genesis_hash",
        )
        .context("failed to prepare distinct governance network hash query")?;
    let rows = stmt
        .query_map([], |row| row.get(0))
        .context("failed to query distinct governance network hashes")?;
    let mut hashes = rows
        .collect::<rusqlite::Result<Vec<String>>>()
        .context("failed to read governance network hashes")?;
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT network_genesis_hash FROM governance_key_vaults ORDER BY network_genesis_hash",
        )
        .context("failed to prepare distinct governance vault network hash query")?;
    let rows = stmt
        .query_map([], |row| row.get(0))
        .context("failed to query distinct governance vault network hashes")?;
    hashes.extend(
        rows.collect::<rusqlite::Result<Vec<String>>>()
            .context("failed to read governance vault network hashes")?,
    );
    hashes.sort();
    hashes.dedup();
    Ok(hashes)
}

pub fn list(conn: &Connection) -> Result<Vec<GovernanceKeyRecord>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, network_genesis_hash, vault_id, created_at, updated_at FROM governance_keys ORDER BY id",
        )
        .context("failed to prepare governance key list query")?;
    let rows = stmt
        .query_map([], |row| {
            Ok(GovernanceKeyRecord {
                id: row.get(0)?,
                network_genesis_hash: row.get(1)?,
                vault_id: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })
        .context("failed to query governance keys")?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read governance keys")
}

fn load_vault(conn: &Connection, network_genesis_hash: &str) -> Result<Option<GovernanceVault>> {
    conn.query_row(
        "SELECT id, network_genesis_hash, kdf_algorithm, kdf_params_json, salt,
                encrypted_dek, dek_nonce, cipher_version, created_at, updated_at
         FROM governance_key_vaults WHERE network_genesis_hash = ?1",
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
            Ok(GovernanceVault {
                record: GovernanceVaultRecord {
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
        format!("failed to query governance key vault for network '{network_genesis_hash}'")
    })
}

fn unlock_vault_record(vault: GovernanceVault, password: &str) -> Result<UnlockedGovernanceVault> {
    if vault.kdf_algorithm != KDF_ALGORITHM {
        bail!("unsupported KDF algorithm: {}", vault.kdf_algorithm);
    }
    let kek = derive_kek(password, &vault.salt, &vault.kdf_params)?;
    let dek_aad = object_aad(
        &vault.record.id,
        GOVERNANCE_VAULT_DEK_KIND,
        vault.cipher_version,
    );
    let dek = aead_decrypt(&kek, &vault.dek_nonce, &vault.encrypted_dek, &dek_aad)
        .context("failed to decrypt governance vault data encryption key")?;
    Ok(UnlockedGovernanceVault {
        record: vault.record,
        dek: zeroizing_array_from_slice::<KEY_LEN>(&dek, "governance vault DEK")?,
    })
}

fn decrypt_payload(
    conn: &Connection,
    record: &GovernanceKeyRecord,
    vault_dek: &[u8; KEY_LEN],
) -> Result<String> {
    let (cipher_version, ciphertext, nonce): (u32, Vec<u8>, Vec<u8>) = conn
        .query_row(
            "SELECT cipher_version, ciphertext, nonce FROM governance_key_payloads WHERE governance_key_id = ?1",
            params![record.id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .with_context(|| format!("failed to query governance key payload for key {}", record.id))?
        .with_context(|| format!("governance key {} has no payload", record.id))?;
    let aad = payload_aad(record, cipher_version);
    let plaintext = aead_decrypt(vault_dek, &nonce, &ciphertext, &aad).with_context(|| {
        format!(
            "failed to decrypt governance key payload for key {}",
            record.id
        )
    })?;
    String::from_utf8(plaintext.to_vec()).with_context(|| {
        format!(
            "governance key payload for key {} is not valid UTF-8",
            record.id
        )
    })
}

fn upsert_payload_in_tx(
    tx: &rusqlite::Transaction<'_>,
    record: &GovernanceKeyRecord,
    vault_dek: &[u8; KEY_LEN],
    raw_json: &str,
) -> Result<()> {
    let aad = payload_aad(record, CIPHER_VERSION);
    let (ciphertext, nonce) =
        aead_encrypt(vault_dek, raw_json.as_bytes(), &aad).with_context(|| {
            format!(
                "failed to encrypt governance key payload for key {}",
                record.id
            )
        })?;
    tx.execute(
        "INSERT INTO governance_key_payloads (governance_key_id, cipher_version, ciphertext, nonce)
         VALUES (?1, ?2, ?3, ?4)",
        params![record.id, CIPHER_VERSION, ciphertext, nonce.as_slice()],
    )
    .with_context(|| {
        format!(
            "failed to store governance key payload for key {}",
            record.id
        )
    })?;
    Ok(())
}

fn payload_aad(record: &GovernanceKeyRecord, cipher_version: u32) -> Vec<u8> {
    object_aad(
        &format!(
            "{}:{}:{}",
            record.id, record.network_genesis_hash, record.vault_id
        ),
        GOVERNANCE_KEY_PAYLOAD_KIND,
        cipher_version,
    )
}

pub fn parse_key_pair_json(raw_json: &str) -> Result<UpdateKeyPair> {
    serde_json::from_str(raw_json).context("failed to parse governance key JSON")
}

pub fn public_key_hex(public_key: &UpdatePublicKey) -> String {
    match serde_json::to_value(&public_key.public)
        .expect("serializing governance public key should succeed")
    {
        serde_json::Value::Object(mut value) => value
            .remove("verifyKey")
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .expect("serialized governance public key must contain verifyKey"),
        other => panic!("unexpected serialized governance public key format: {other:?}"),
    }
}

fn delete_empty_vault(conn: &Connection, network_genesis_hash: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM governance_key_vaults
         WHERE network_genesis_hash = ?1
           AND NOT EXISTS (
             SELECT 1 FROM governance_keys k
             WHERE k.vault_id = governance_key_vaults.id
           )",
        params![network_genesis_hash],
    )
    .with_context(|| {
        format!("failed to clean empty governance key vault for network '{network_genesis_hash}'")
    })?;
    Ok(())
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
    use crate::store::migrations;
    use rand::thread_rng;
    use rusqlite::Connection;

    const MAINNET: &str = "mainnet-hash";

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    fn key_json() -> String {
        serde_json::to_string(&UpdateKeyPair::generate(&mut thread_rng())).unwrap()
    }

    #[test]
    fn governance_vault_round_trip_and_wrong_password() {
        let conn = conn();
        let unlocked = create_or_unlock_vault(&conn, MAINNET, "password").unwrap();
        let reused = create_or_unlock_vault(&conn, MAINNET, "password").unwrap();
        assert_eq!(unlocked.record.id, reused.record.id);
        assert!(unlock_vault(&conn, MAINNET, "wrong").is_err());
    }

    #[test]
    fn imported_governance_key_is_encrypted_and_deduplicated() {
        let mut conn = conn();
        let unlocked = create_or_unlock_vault(&conn, MAINNET, "password").unwrap();
        let raw = key_json();
        let record = import_key_json(&mut conn, &unlocked.record, &unlocked.dek, &raw).unwrap();
        let decrypted = decrypted_keys(&conn, MAINNET, &unlocked.dek).unwrap();
        assert_eq!(decrypted.len(), 1);
        assert_eq!(decrypted[0].record.id, record.id);
        assert_eq!(decrypted[0].raw_json, raw);
        let err = import_key_json(&mut conn, &unlocked.record, &unlocked.dek, &raw).unwrap_err();
        assert!(err.to_string().contains("already exists"));
        let raw_count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM governance_key_payloads WHERE CAST(ciphertext AS TEXT) LIKE '%signKey%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(raw_count, 0);
    }

    #[test]
    fn remove_by_verify_key_and_remove_all_cleanup_vault() {
        let mut conn = conn();
        let unlocked = create_or_unlock_vault(&conn, MAINNET, "password").unwrap();
        let raw = key_json();
        let parsed = parse_key_pair_json(&raw).unwrap();
        let verify_key = public_key_hex(&UpdatePublicKey::from(&parsed));
        import_key_json(&mut conn, &unlocked.record, &unlocked.dek, &raw).unwrap();
        assert!(remove_by_verify_key(&conn, MAINNET, &unlocked.dek, &verify_key).unwrap());
        assert!(!governance_vault_exists(&conn, MAINNET).unwrap());

        let unlocked = create_or_unlock_vault(&conn, MAINNET, "password").unwrap();
        import_key_json(&mut conn, &unlocked.record, &unlocked.dek, &key_json()).unwrap();
        import_key_json(&mut conn, &unlocked.record, &unlocked.dek, &key_json()).unwrap();
        assert_eq!(remove_all(&conn, MAINNET).unwrap(), 2);
        assert!(!governance_vault_exists(&conn, MAINNET).unwrap());
    }

    #[test]
    fn prune_by_network_removes_keys_and_vault() {
        let mut conn = conn();
        let unlocked = create_or_unlock_vault(&conn, MAINNET, "password").unwrap();
        import_key_json(&mut conn, &unlocked.record, &unlocked.dek, &key_json()).unwrap();
        assert_eq!(prune_by_network(&conn, MAINNET).unwrap(), 1);
        assert!(!governance_vault_exists(&conn, MAINNET).unwrap());
        assert!(distinct_network_genesis_hashes(&conn).unwrap().is_empty());
    }
}
