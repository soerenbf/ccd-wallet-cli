//! Seed storage compatibility API backed by seed-kind signer owners.
//!
//! Public seed commands continue to work with seed labels and unlocked seed
//! contexts, while the persisted representation uses the signer-owner model.

use crate::store::{
    crypto::KEY_LEN,
    signer_owners::{self, SignerOwnerKind, SignerOwnerRecord},
};
use anyhow::{Context, Result, bail};
use rusqlite::Connection;
use zeroize::Zeroizing;

/// Plaintext seed metadata exposed to seed commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedRecord {
    /// Stable signer-owner id for the seed.
    pub id: String,
    /// User-facing seed label.
    pub label: String,
    /// Unix timestamp for creation.
    pub created_at: i64,
    /// Unix timestamp for last metadata update.
    pub updated_at: i64,
}

/// Unlocked seed context.
///
/// This value contains both the decrypted seed secret and the signer-owner DEK
/// used for seed-owned identity/account private payloads.
#[derive(Debug)]
pub struct UnlockedSeed {
    /// Plaintext seed metadata.
    pub record: SeedRecord,
    /// Decrypted seed secret bytes.
    pub secret: Zeroizing<Vec<u8>>,
    /// Decrypted signer-owner DEK.
    pub dek: Zeroizing<[u8; KEY_LEN]>,
}

/// Add a seed phrase to the wallet.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `label` - Unique seed/key-source label.
/// * `secret` - Seed phrase or seed secret bytes.
/// * `password` - Local password for the seed signer owner.
///
/// # Returns
/// Plaintext seed metadata for the newly created seed.
///
/// # Errors
/// Returns an error if the label already exists or storage/encryption fails.
///
/// # Examples
///
/// ```ignore
/// let seed = seeds::add(&conn, "main_seed", mnemonic.as_bytes(), password)?;
/// ```
pub fn add(conn: &Connection, label: &str, secret: &[u8], password: &str) -> Result<SeedRecord> {
    if signer_owners::find_by_label(conn, label)?.is_some() {
        bail!("seed label '{label}' already exists");
    }

    let owner = signer_owners::create(conn, SignerOwnerKind::Seed, label)?;
    let dek = signer_owners::create_vault(conn, &owner.id, password)?;
    signer_owners::insert_seed_secret(conn, &owner.id, &dek, secret)?;
    Ok(seed_record_from_owner(owner))
}

/// List configured seeds without unlocking them.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
///
/// # Returns
/// Seed metadata rows sorted by label.
///
/// # Errors
/// Returns an error if querying signer owners fails.
///
/// # Examples
///
/// ```ignore
/// let seeds = seeds::list(&conn)?;
/// ```
pub fn list(conn: &Connection) -> Result<Vec<SeedRecord>> {
    Ok(signer_owners::list(conn)?
        .into_iter()
        .filter(|owner| owner.kind == SignerOwnerKind::Seed)
        .map(seed_record_from_owner)
        .collect())
}

/// Remove a configured seed by label.
///
/// Deleting the seed signer owner cascades to its vault, seed secret, and
/// signer-owned identity/account rows.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `label` - Seed label to delete.
///
/// # Errors
/// Returns an error if the seed label is unknown or deletion fails.
///
/// # Examples
///
/// ```ignore
/// seeds::remove(&conn, "old_seed")?;
/// ```
pub fn remove(conn: &Connection, label: &str) -> Result<()> {
    let seed =
        find_by_label(conn, label)?.with_context(|| format!("seed '{label}' is not configured"))?;
    signer_owners::delete_by_id(conn, &seed.id)
}

/// Rename a configured seed.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `old_label` - Existing seed label.
/// * `new_label` - New unique key-source label.
///
/// # Errors
/// Returns an error if the source seed is unknown, the target label is already
/// used by any key source, or the rename fails.
///
/// # Examples
///
/// ```ignore
/// seeds::rename(&conn, "main_seed", "daily")?;
/// ```
pub fn rename(conn: &Connection, old_label: &str, new_label: &str) -> Result<()> {
    if old_label == new_label {
        return Ok(());
    }
    find_by_label(conn, old_label)?
        .with_context(|| format!("seed '{old_label}' is not configured"))?;
    signer_owners::rename(conn, old_label, new_label)
        .map_err(|err| anyhow::anyhow!(err.to_string().replace("signer owner label", "seed label")))
}

/// Unlock a seed and return only its decrypted secret bytes.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `label` - Seed label.
/// * `password` - Local seed password.
///
/// # Returns
/// Decrypted seed secret bytes.
///
/// # Errors
/// Returns an error if the seed is unknown, the password is wrong, or decryption
/// fails.
///
/// # Examples
///
/// ```ignore
/// let seed_secret = seeds::unlock(&conn, "main_seed", password)?;
/// ```
pub fn unlock(conn: &Connection, label: &str, password: &str) -> Result<Zeroizing<Vec<u8>>> {
    Ok(unlock_context(conn, label, password)?.secret)
}

/// Unlock a seed and return both seed secret and signer-owner DEK.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `label` - Seed label.
/// * `password` - Local seed password.
///
/// # Returns
/// Unlocked seed context.
///
/// # Errors
/// Returns an error if the seed is unknown, the password is wrong, or decryption
/// fails.
///
/// # Examples
///
/// ```ignore
/// let unlocked = seeds::unlock_context(&conn, "main_seed", password)?;
/// ```
pub fn unlock_context(conn: &Connection, label: &str, password: &str) -> Result<UnlockedSeed> {
    let seed =
        find_by_label(conn, label)?.with_context(|| format!("seed '{label}' is not configured"))?;
    let unlocked = signer_owners::unlock_by_id(conn, &seed.id, password)?;
    let secret = signer_owners::decrypt_seed_secret(conn, &seed.id, &unlocked.dek)?;
    Ok(UnlockedSeed {
        record: seed,
        secret,
        dek: unlocked.dek,
    })
}

/// Change a seed password.
///
/// Password changes re-wrap the signer-owner DEK and do not re-encrypt the seed
/// secret payload or child payloads.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `label` - Seed label.
/// * `old_password` - Current local seed password.
/// * `new_password` - New local seed password.
///
/// # Errors
/// Returns an error if the seed is unknown, the old password is wrong, or the
/// vault update fails.
///
/// # Examples
///
/// ```ignore
/// seeds::change_password(&conn, "main_seed", old_password, new_password)?;
/// ```
pub fn change_password(
    conn: &Connection,
    label: &str,
    old_password: &str,
    new_password: &str,
) -> Result<()> {
    find_by_label(conn, label)?.with_context(|| format!("seed '{label}' is not configured"))?;
    signer_owners::change_password(conn, label, old_password, new_password)
}

/// Find a seed by label without unlocking it.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `label` - Seed label to resolve.
///
/// # Returns
/// `Some(seed)` when the label belongs to a seed-kind signer owner, otherwise
/// `None`.
///
/// # Errors
/// Returns an error if the signer-owner lookup fails.
///
/// # Examples
///
/// ```ignore
/// let seed = seeds::find_by_label(&conn, "main_seed")?;
/// ```
pub fn find_by_label(conn: &Connection, label: &str) -> Result<Option<SeedRecord>> {
    Ok(signer_owners::find_by_label(conn, label)?
        .filter(|owner| owner.kind == SignerOwnerKind::Seed)
        .map(seed_record_from_owner))
}

fn seed_record_from_owner(owner: SignerOwnerRecord) -> SeedRecord {
    SeedRecord {
        id: owner.id,
        label: owner.label,
        created_at: owner.created_at,
        updated_at: owner.updated_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{migrations, signer_owners};
    use rusqlite::params;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
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
    fn duplicate_label_is_rejected_across_key_source_kinds() {
        let conn = conn();
        add(&conn, "main_seed", b"seed secret", "password").unwrap();

        let err = add(&conn, "main_seed", b"other", "password").unwrap_err();
        assert!(err.to_string().contains("already exists"));

        let owner = signer_owners::create(&conn, SignerOwnerKind::Ledger, "ledger").unwrap();
        signer_owners::create_vault(&conn, &owner.id, "password").unwrap();
        let err = add(&conn, "ledger", b"other", "password").unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn find_by_label_returns_only_seed_metadata_without_unlocking() {
        let conn = conn();
        let seed = add(&conn, "main_seed", b"seed secret", "password").unwrap();
        let ledger = signer_owners::create(&conn, SignerOwnerKind::Ledger, "ledger").unwrap();
        signer_owners::create_vault(&conn, &ledger.id, "password").unwrap();

        assert_eq!(find_by_label(&conn, "main_seed").unwrap(), Some(seed));
        assert_eq!(find_by_label(&conn, "ledger").unwrap(), None);
        assert_eq!(find_by_label(&conn, "missing").unwrap(), None);
    }

    #[test]
    fn remove_deletes_existing_seed_and_cascades_vault_and_secret() {
        let conn = conn();
        add(&conn, "main_seed", b"seed secret", "password").unwrap();

        remove(&conn, "main_seed").unwrap();

        assert!(find_by_label(&conn, "main_seed").unwrap().is_none());
        for table in ["signer_owner_vaults", "seed_owner_secrets"] {
            let count: u32 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 0);
        }
    }

    #[test]
    fn remove_unknown_or_non_seed_label_errors() {
        let conn = conn();
        let ledger = signer_owners::create(&conn, SignerOwnerKind::Ledger, "ledger").unwrap();
        signer_owners::create_vault(&conn, &ledger.id, "password").unwrap();

        let missing = remove(&conn, "missing").unwrap_err();
        assert!(
            missing
                .to_string()
                .contains("seed 'missing' is not configured")
        );

        let non_seed = remove(&conn, "ledger").unwrap_err();
        assert!(
            non_seed
                .to_string()
                .contains("seed 'ledger' is not configured")
        );
        assert!(
            signer_owners::find_by_label(&conn, "ledger")
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn rename_updates_label_but_preserves_id() {
        let conn = conn();
        let seed = add(&conn, "main_seed", b"seed secret", "password").unwrap();

        rename(&conn, "main_seed", "daily").unwrap();

        let renamed = find_by_label(&conn, "daily").unwrap().unwrap();
        assert_eq!(renamed.id, seed.id);
        assert!(find_by_label(&conn, "main_seed").unwrap().is_none());
    }

    #[test]
    fn rename_rejects_duplicate_target_label_across_key_sources() {
        let conn = conn();
        add(&conn, "seed_a", b"a", "password").unwrap();
        add(&conn, "seed_b", b"b", "password").unwrap();

        let err = rename(&conn, "seed_a", "seed_b").unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn change_password_rewraps_dek_without_changing_seed_payload() {
        let conn = conn();
        let seed = add(&conn, "main_seed", b"seed secret", "old password").unwrap();

        let before_payload: Vec<u8> = conn
            .query_row(
                "SELECT payload_ciphertext FROM seed_owner_secrets WHERE signer_owner_id = ?1",
                params![seed.id],
                |row| row.get(0),
            )
            .unwrap();

        change_password(&conn, "main_seed", "old password", "new password").unwrap();

        assert!(unlock(&conn, "main_seed", "old password").is_err());
        let unlocked = unlock(&conn, "main_seed", "new password").unwrap();
        assert_eq!(&**unlocked, b"seed secret");

        let after_payload: Vec<u8> = conn
            .query_row(
                "SELECT payload_ciphertext FROM seed_owner_secrets WHERE signer_owner_id = ?1",
                params![seed.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(before_payload, after_payload);
    }
}
