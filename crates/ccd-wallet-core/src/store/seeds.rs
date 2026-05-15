#![allow(dead_code)]

use crate::store::crypto::{
    Argon2Params, KEY_LEN, aead_decrypt, aead_encrypt, derive_kek, generate_dek, object_aad,
    random_salt, zeroizing_array_from_slice,
};
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use zeroize::Zeroizing;

const KDF_ALGORITHM: &str = "argon2id";
const CIPHER_VERSION: u32 = 1;
const SEED_KIND: &str = "seed";
const SEED_DEK_KIND: &str = "seed_dek";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedRecord {
    pub id: String,
    pub label: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug)]
pub struct UnlockedSeed {
    pub record: SeedRecord,
    pub secret: Zeroizing<Vec<u8>>,
    pub dek: Zeroizing<[u8; KEY_LEN]>,
}

#[derive(Debug)]
struct SeedVault {
    kdf_algorithm: String,
    kdf_params: Argon2Params,
    salt: Vec<u8>,
    encrypted_dek: Vec<u8>,
    dek_nonce: Vec<u8>,
    cipher_version: u32,
    payload_ciphertext: Vec<u8>,
    payload_nonce: Vec<u8>,
}

pub fn add(conn: &Connection, label: &str, secret: &[u8], password: &str) -> Result<SeedRecord> {
    if find_by_label(conn, label)?.is_some() {
        bail!("seed label '{label}' already exists");
    }

    let id = Uuid::new_v4().to_string();
    let now = now_unix_seconds()?;
    let record = SeedRecord {
        id: id.clone(),
        label: label.to_owned(),
        created_at: now,
        updated_at: now,
    };

    let params = Argon2Params::default();
    let salt = random_salt();
    let dek = generate_dek();
    let kek = derive_kek(password, &salt, &params)?;

    let dek_aad = object_aad(&id, SEED_DEK_KIND, CIPHER_VERSION);
    let (encrypted_dek, dek_nonce) = aead_encrypt(&kek, &*dek, &dek_aad)?;

    let payload_aad = object_aad(&id, SEED_KIND, CIPHER_VERSION);
    let (payload_ciphertext, payload_nonce) = aead_encrypt(&dek, secret, &payload_aad)?;

    let kdf_params_json =
        serde_json::to_string(&params).context("failed to serialise KDF params")?;

    conn.execute(
        "INSERT INTO seeds (id, label, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
        params![
            record.id,
            record.label,
            record.created_at,
            record.updated_at
        ],
    )
    .with_context(|| format!("failed to insert seed '{}': metadata", label))?;

    conn.execute(
        "INSERT INTO seed_vaults (
            seed_id, kdf_algorithm, kdf_params_json, salt, encrypted_dek, dek_nonce,
            cipher_version, payload_ciphertext, payload_nonce
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            id,
            KDF_ALGORITHM,
            kdf_params_json,
            salt.as_slice(),
            encrypted_dek,
            dek_nonce.as_slice(),
            CIPHER_VERSION,
            payload_ciphertext,
            payload_nonce.as_slice(),
        ],
    )
    .with_context(|| format!("failed to insert seed '{}': vault", label))?;

    Ok(record)
}

pub fn list(conn: &Connection) -> Result<Vec<SeedRecord>> {
    let mut stmt = conn
        .prepare("SELECT id, label, created_at, updated_at FROM seeds ORDER BY label ASC")
        .context("failed to prepare seed list query")?;

    let rows = stmt
        .query_map([], |row| {
            Ok(SeedRecord {
                id: row.get(0)?,
                label: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })
        .context("failed to query seeds")?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read seed rows")
}

pub fn remove(conn: &Connection, label: &str) -> Result<()> {
    let affected = conn
        .execute("DELETE FROM seeds WHERE label = ?1", params![label])
        .with_context(|| format!("failed to remove seed '{label}'"))?;

    if affected == 0 {
        bail!("seed '{label}' is not configured");
    }

    Ok(())
}

pub fn unlock(conn: &Connection, label: &str, password: &str) -> Result<Zeroizing<Vec<u8>>> {
    Ok(unlock_context(conn, label, password)?.secret)
}

pub fn unlock_context(conn: &Connection, label: &str, password: &str) -> Result<UnlockedSeed> {
    let (record, vault) = load_seed_and_vault(conn, label)?;
    let dek = unlock_dek(&record.id, password, &vault)?;
    let payload_aad = object_aad(&record.id, SEED_KIND, vault.cipher_version);

    let secret = aead_decrypt(
        &dek,
        &vault.payload_nonce,
        &vault.payload_ciphertext,
        &payload_aad,
    )
    .with_context(|| format!("failed to unlock seed '{label}'"))?;

    Ok(UnlockedSeed {
        record,
        secret,
        dek,
    })
}

pub fn change_password(
    conn: &Connection,
    label: &str,
    old_password: &str,
    new_password: &str,
) -> Result<()> {
    let (record, vault) = load_seed_and_vault(conn, label)?;
    let dek = unlock_dek(&record.id, old_password, &vault)?;

    let params = Argon2Params::default();
    let salt = random_salt();
    let new_kek = derive_kek(new_password, &salt, &params)?;
    let dek_aad = object_aad(&record.id, SEED_DEK_KIND, CIPHER_VERSION);
    let (encrypted_dek, dek_nonce) = aead_encrypt(&new_kek, &*dek, &dek_aad)?;
    let kdf_params_json =
        serde_json::to_string(&params).context("failed to serialise KDF params")?;

    conn.execute(
        "UPDATE seed_vaults
         SET kdf_algorithm = ?1, kdf_params_json = ?2, salt = ?3,
             encrypted_dek = ?4, dek_nonce = ?5, cipher_version = ?6
         WHERE seed_id = ?7",
        params![
            KDF_ALGORITHM,
            kdf_params_json,
            salt.as_slice(),
            encrypted_dek,
            dek_nonce.as_slice(),
            CIPHER_VERSION,
            record.id,
        ],
    )
    .with_context(|| format!("failed to change password for seed '{label}'"))?;

    Ok(())
}

fn unlock_dek(
    seed_id: &str,
    password: &str,
    vault: &SeedVault,
) -> Result<Zeroizing<[u8; KEY_LEN]>> {
    if vault.kdf_algorithm != KDF_ALGORITHM {
        bail!("unsupported KDF algorithm: {}", vault.kdf_algorithm);
    }

    let kek = derive_kek(password, &vault.salt, &vault.kdf_params)?;
    let dek_aad = object_aad(seed_id, SEED_DEK_KIND, vault.cipher_version);
    let dek = aead_decrypt(&kek, &vault.dek_nonce, &vault.encrypted_dek, &dek_aad)
        .context("failed to decrypt seed data encryption key")?;

    zeroizing_array_from_slice::<KEY_LEN>(&dek, "DEK")
}

pub fn find_by_label(conn: &Connection, label: &str) -> Result<Option<SeedRecord>> {
    conn.query_row(
        "SELECT id, label, created_at, updated_at FROM seeds WHERE label = ?1",
        params![label],
        |row| {
            Ok(SeedRecord {
                id: row.get(0)?,
                label: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        },
    )
    .optional()
    .with_context(|| format!("failed to query seed '{label}'"))
}

fn load_seed_and_vault(conn: &Connection, label: &str) -> Result<(SeedRecord, SeedVault)> {
    conn.query_row(
        "SELECT
            s.id, s.label, s.created_at, s.updated_at,
            v.kdf_algorithm, v.kdf_params_json, v.salt, v.encrypted_dek, v.dek_nonce,
            v.cipher_version, v.payload_ciphertext, v.payload_nonce
         FROM seeds s
         JOIN seed_vaults v ON v.seed_id = s.id
         WHERE s.label = ?1",
        params![label],
        |row| {
            let kdf_params_json: String = row.get(5)?;
            let kdf_params = serde_json::from_str(&kdf_params_json).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
            })?;

            Ok((
                SeedRecord {
                    id: row.get(0)?,
                    label: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                },
                SeedVault {
                    kdf_algorithm: row.get(4)?,
                    kdf_params,
                    salt: row.get(6)?,
                    encrypted_dek: row.get(7)?,
                    dek_nonce: row.get(8)?,
                    cipher_version: row.get::<_, u32>(9)?,
                    payload_ciphertext: row.get(10)?,
                    payload_nonce: row.get(11)?,
                },
            ))
        },
    )
    .optional()
    .with_context(|| format!("failed to load seed '{label}'"))?
    .with_context(|| format!("seed '{label}' is not configured"))
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

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    #[test]
    fn add_and_unlock_round_trip() {
        let conn = conn();
        add(&conn, "main_seed", b"seed secret", "password").unwrap();

        let unlocked = unlock(&conn, "main_seed", "password").unwrap();
        assert_eq!(&**unlocked, b"seed secret");

        assert!(unlock(&conn, "main_seed", "wrong").is_err());
    }

    #[test]
    fn unlock_context_exposes_secret_and_dek_after_password_verification() {
        let conn = conn();
        let seed = add(&conn, "main_seed", b"seed secret", "password").unwrap();

        let unlocked = unlock_context(&conn, "main_seed", "password").unwrap();
        assert_eq!(unlocked.record, seed);
        assert_eq!(&*unlocked.secret, b"seed secret");
        assert_ne!(&*unlocked.dek, &[0u8; KEY_LEN]);

        assert!(unlock_context(&conn, "main_seed", "wrong").is_err());
    }

    #[test]
    fn list_does_not_require_password() {
        let conn = conn();
        let seed = add(&conn, "main_seed", b"seed secret", "password").unwrap();

        assert_eq!(list(&conn).unwrap(), vec![seed]);
    }

    #[test]
    fn duplicate_label_is_rejected() {
        let conn = conn();
        add(&conn, "main_seed", b"seed secret", "password").unwrap();

        let err = add(&conn, "main_seed", b"other", "password").unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn find_by_label_returns_metadata_without_unlocking() {
        let conn = conn();
        let seed = add(&conn, "main_seed", b"seed secret", "password").unwrap();

        assert_eq!(find_by_label(&conn, "main_seed").unwrap(), Some(seed));
        assert_eq!(find_by_label(&conn, "missing").unwrap(), None);
    }

    #[test]
    fn remove_deletes_existing_seed_and_cascades_vault() {
        let conn = conn();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        add(&conn, "main_seed", b"seed secret", "password").unwrap();

        remove(&conn, "main_seed").unwrap();

        assert!(find_by_label(&conn, "main_seed").unwrap().is_none());
        let vault_count: u32 = conn
            .query_row("SELECT COUNT(*) FROM seed_vaults", [], |row| row.get(0))
            .unwrap();
        assert_eq!(vault_count, 0);
    }

    #[test]
    fn remove_unknown_seed_errors() {
        let conn = conn();

        let err = remove(&conn, "missing").unwrap_err();

        assert!(err.to_string().contains("seed 'missing' is not configured"));
    }

    #[test]
    fn change_password_reencrypts_dek() {
        let conn = conn();
        add(&conn, "main_seed", b"seed secret", "old password").unwrap();

        let before_payload: Vec<u8> = conn
            .query_row(
                "SELECT payload_ciphertext FROM seed_vaults JOIN seeds ON seeds.id = seed_vaults.seed_id WHERE seeds.label = ?1",
                params!["main_seed"],
                |row| row.get(0),
            )
            .unwrap();

        change_password(&conn, "main_seed", "old password", "new password").unwrap();

        assert!(unlock(&conn, "main_seed", "old password").is_err());
        let unlocked = unlock(&conn, "main_seed", "new password").unwrap();
        assert_eq!(&**unlocked, b"seed secret");

        let after_payload: Vec<u8> = conn
            .query_row(
                "SELECT payload_ciphertext FROM seed_vaults JOIN seeds ON seeds.id = seed_vaults.seed_id WHERE seeds.label = ?1",
                params!["main_seed"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(before_payload, after_payload);
    }
}
