//! Signer-owner storage types and helpers.
//!
//! A signer owner is the wallet-local derivation authority for seed-backed and
//! Ledger-backed identities and derived accounts. The owner also anchors a
//! local password-protected encryption domain used for signer-owned private
//! payloads.

use crate::store::crypto::{
    Argon2Params, KEY_LEN, aead_decrypt, aead_encrypt, derive_kek, generate_dek, object_aad,
    random_salt, zeroizing_array_from_slice,
};
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};
use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;
use zeroize::Zeroizing;

const KDF_ALGORITHM: &str = "argon2id";
const CIPHER_VERSION: u32 = 1;
const SIGNER_OWNER_DEK_KIND: &str = "signer_owner_dek";
const SEED_OWNER_SECRET_KIND: &str = "seed_owner_secret";

/// Canonical Concordium Ledger path used to identify an enrolled Ledger key source.
pub const LEDGER_OWNER_ENROLLMENT_PATH: &str = "m/44'/919'/0'/0'/0'";

/// Supported signer-owner backing kinds.
///
/// A seed signer owner stores local seed secret material encrypted under its
/// signer-owner vault. A Ledger signer owner stores public enrollment metadata
/// and relies on a matching Ledger device for signing operations.
///
/// # Examples
///
/// ```
/// use ccd_wallet_core::store::signer_owners::SignerOwnerKind;
/// assert_eq!(SignerOwnerKind::Seed.as_str(), "seed");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignerOwnerKind {
    /// A signer owner backed by locally stored seed secret material.
    Seed,
    /// A signer owner backed by an enrolled Ledger device.
    Ledger,
}

impl SignerOwnerKind {
    /// Return the database representation for this signer-owner kind.
    ///
    /// # Returns
    /// The stable lowercase value stored in `signer_owners.owner_kind`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_core::store::signer_owners::SignerOwnerKind;
    /// assert_eq!(SignerOwnerKind::Ledger.as_str(), "ledger");
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Seed => "seed",
            Self::Ledger => "ledger",
        }
    }
}

impl FromStr for SignerOwnerKind {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "seed" => Ok(Self::Seed),
            "ledger" => Ok(Self::Ledger),
            other => bail!("unsupported signer owner kind '{other}'"),
        }
    }
}

/// Plaintext metadata for a signer owner.
///
/// Signer owner metadata is intentionally listable without unlocking the local
/// password domain. Private owner data lives in owner-kind detail tables and
/// signer-owned payload tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignerOwnerRecord {
    /// Stable signer-owner id used by child rows.
    pub id: String,
    /// Backing kind for this signer owner.
    pub kind: SignerOwnerKind,
    /// User-facing key-source label.
    pub label: String,
    /// Unix timestamp for creation.
    pub created_at: i64,
    /// Unix timestamp for last metadata update.
    pub updated_at: i64,
}

/// Persisted key-wrapping metadata for a signer-owner vault.
///
/// This internal representation contains the encrypted DEK and KDF metadata
/// needed to unlock a signer-owner password domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignerOwnerVaultRecord {
    /// Signer owner that owns this vault.
    pub signer_owner_id: String,
    /// KDF algorithm name.
    pub kdf_algorithm: String,
    /// Serialized KDF parameters.
    pub kdf_params: Argon2Params,
    /// KDF salt bytes.
    pub salt: Vec<u8>,
    /// DEK encrypted under the password-derived KEK.
    pub encrypted_dek: Vec<u8>,
    /// Nonce used for DEK encryption.
    pub dek_nonce: Vec<u8>,
    /// Cipher format version.
    pub cipher_version: u32,
    /// Unix timestamp for creation.
    pub created_at: i64,
    /// Unix timestamp for last metadata update.
    pub updated_at: i64,
}

/// Seed-kind owner encrypted secret metadata.
///
/// The ciphertext contains the locally stored seed secret encrypted under the
/// owning signer-owner DEK.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedOwnerSecretRecord {
    /// Seed signer owner that owns this secret payload.
    pub signer_owner_id: String,
    /// Cipher format version.
    pub cipher_version: u32,
    /// Encrypted seed secret payload.
    pub payload_ciphertext: Vec<u8>,
    /// Nonce used for seed secret payload encryption.
    pub payload_nonce: Vec<u8>,
}

/// Ledger-kind owner public enrollment metadata.
///
/// Ledger owner details identify the hardware-backed wallet root without
/// storing private signing material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerOwnerDetailsRecord {
    /// Ledger signer owner that owns these details.
    pub signer_owner_id: String,
    /// Full canonical public key returned by the Ledger app at enrollment.
    pub canonical_public_key: Vec<u8>,
    /// Short display fingerprint derived from the canonical public key.
    pub fingerprint: String,
    /// Derivation path used to obtain the canonical public key.
    pub enrollment_path: String,
    /// Optional app name observed during enrollment.
    pub app_name: Option<String>,
    /// Unix timestamp for creation.
    pub created_at: i64,
    /// Unix timestamp for last metadata update.
    pub updated_at: i64,
    /// Optional Unix timestamp for the last successful device match.
    pub last_seen_at: Option<i64>,
}

/// Input metadata for inserting Ledger signer-owner details.
///
/// # Examples
///
/// ```ignore
/// let details = NewLedgerOwnerDetails {
///     signer_owner_id: owner.id.as_str(),
///     canonical_public_key: &[0; 32],
///     fingerprint: "00000000",
///     enrollment_path: "m/44'/919'/0'/0'/0'",
///     app_name: Some("Concordium"),
/// };
/// ```
pub struct NewLedgerOwnerDetails<'a> {
    /// Ledger signer owner id.
    pub signer_owner_id: &'a str,
    /// Full canonical public key returned by the Ledger app.
    pub canonical_public_key: &'a [u8],
    /// Short display fingerprint derived from the canonical public key.
    pub fingerprint: &'a str,
    /// Derivation path used to retrieve the canonical public key.
    pub enrollment_path: &'a str,
    /// Optional Ledger app name observed during enrollment.
    pub app_name: Option<&'a str>,
}

/// Unlocked signer-owner password domain.
///
/// Operations use this value to encrypt or decrypt signer-owned private
/// payloads. Seed-backed flows may additionally decrypt the seed owner secret.
#[derive(Debug)]
pub struct UnlockedSignerOwner {
    /// Plaintext signer-owner metadata.
    pub record: SignerOwnerRecord,
    /// Decrypted owner DEK.
    pub dek: Zeroizing<[u8; KEY_LEN]>,
}

/// Derive the short display fingerprint for a Ledger owner canonical public key.
///
/// The full canonical public key remains the unique storage identity. The
/// fingerprint is only a compact user-facing display value.
///
/// # Arguments
/// * `canonical_public_key` - Full public key returned by the Ledger app at
///   [`LEDGER_OWNER_ENROLLMENT_PATH`].
///
/// # Returns
/// An eight-character lowercase hexadecimal fingerprint.
///
/// # Examples
///
/// ```
/// use ccd_wallet_core::store::signer_owners::ledger_owner_fingerprint;
/// assert_eq!(ledger_owner_fingerprint(&[7; 32]).len(), 8);
/// ```
pub fn ledger_owner_fingerprint(canonical_public_key: &[u8]) -> String {
    let digest = Sha256::digest(canonical_public_key);
    hex::encode(&digest[..4])
}

/// Create a signer owner metadata row.
///
/// This function stores only plaintext signer-owner metadata. Callers that
/// create usable owners are also expected to create the corresponding vault and
/// owner-kind detail row in the same higher-level operation.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `kind` - Backing kind for the signer owner.
/// * `label` - Unique user-facing key-source label.
///
/// # Returns
/// The inserted signer-owner record.
///
/// # Errors
/// Returns an error if the label already exists or SQLite insertion fails.
///
/// # Examples
///
/// ```ignore
/// let owner = signer_owners::create(&conn, SignerOwnerKind::Seed, "main")?;
/// ```
pub fn create(conn: &Connection, kind: SignerOwnerKind, label: &str) -> Result<SignerOwnerRecord> {
    if find_by_label(conn, label)?.is_some() {
        bail!("signer owner label '{label}' already exists");
    }

    let now = now_unix_seconds()?;
    let record = SignerOwnerRecord {
        id: Uuid::new_v4().to_string(),
        kind,
        label: label.to_owned(),
        created_at: now,
        updated_at: now,
    };

    conn.execute(
        "INSERT INTO signer_owners (id, owner_kind, label, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            record.id,
            record.kind.as_str(),
            record.label,
            record.created_at,
            record.updated_at,
        ],
    )
    .with_context(|| format!("failed to insert signer owner '{label}'"))?;

    Ok(record)
}

/// List signer owners ordered by label.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
///
/// # Returns
/// Plaintext signer-owner records sorted by label.
///
/// # Errors
/// Returns an error if querying or decoding signer owner rows fails.
///
/// # Examples
///
/// ```ignore
/// let owners = signer_owners::list(&conn)?;
/// ```
pub fn list(conn: &Connection) -> Result<Vec<SignerOwnerRecord>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, owner_kind, label, created_at, updated_at
             FROM signer_owners ORDER BY label ASC",
        )
        .context("failed to prepare signer owner list query")?;
    let rows = stmt
        .query_map([], signer_owner_from_row)
        .context("failed to query signer owners")?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read signer owner rows")
}

/// Find a signer owner by user-facing label.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `label` - Signer-owner label to resolve.
///
/// # Returns
/// `Some(record)` when found, otherwise `None`.
///
/// # Errors
/// Returns an error if the lookup or row decoding fails.
///
/// # Examples
///
/// ```ignore
/// let owner = signer_owners::find_by_label(&conn, "main")?;
/// ```
pub fn find_by_label(conn: &Connection, label: &str) -> Result<Option<SignerOwnerRecord>> {
    conn.query_row(
        "SELECT id, owner_kind, label, created_at, updated_at
         FROM signer_owners WHERE label = ?1",
        params![label],
        signer_owner_from_row,
    )
    .optional()
    .with_context(|| format!("failed to query signer owner '{label}'"))
}

/// Find a signer owner by stable id.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `id` - Stable signer-owner id.
///
/// # Returns
/// `Some(record)` when found, otherwise `None`.
///
/// # Errors
/// Returns an error if the lookup or row decoding fails.
///
/// # Examples
///
/// ```ignore
/// let owner = signer_owners::find_by_id(&conn, owner_id)?;
/// ```
pub fn find_by_id(conn: &Connection, id: &str) -> Result<Option<SignerOwnerRecord>> {
    conn.query_row(
        "SELECT id, owner_kind, label, created_at, updated_at
         FROM signer_owners WHERE id = ?1",
        params![id],
        signer_owner_from_row,
    )
    .optional()
    .with_context(|| format!("failed to query signer owner id '{id}'"))
}

/// Rename a signer owner by label.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `old_label` - Existing signer-owner label.
/// * `new_label` - Replacement signer-owner label.
///
/// # Errors
/// Returns an error if `old_label` is unknown, `new_label` already exists, or
/// the update fails.
///
/// # Examples
///
/// ```ignore
/// signer_owners::rename(&conn, "main", "daily")?;
/// ```
pub fn rename(conn: &Connection, old_label: &str, new_label: &str) -> Result<()> {
    if old_label == new_label {
        return Ok(());
    }
    if find_by_label(conn, new_label)?.is_some() {
        bail!("signer owner label '{new_label}' already exists");
    }

    let now = now_unix_seconds()?;
    let affected = conn
        .execute(
            "UPDATE signer_owners SET label = ?1, updated_at = ?2 WHERE label = ?3",
            params![new_label, now, old_label],
        )
        .with_context(|| format!("failed to rename signer owner '{old_label}' to '{new_label}'"))?;
    if affected == 0 {
        bail!("signer owner '{old_label}' is not configured");
    }
    Ok(())
}

/// Delete a signer owner by label.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `label` - Signer-owner label to remove.
///
/// # Errors
/// Returns an error if the label is unknown or deletion fails.
///
/// # Examples
///
/// ```ignore
/// signer_owners::delete_by_label(&conn, "old-ledger")?;
/// ```
pub fn delete_by_label(conn: &Connection, label: &str) -> Result<()> {
    let affected = conn
        .execute("DELETE FROM signer_owners WHERE label = ?1", params![label])
        .with_context(|| format!("failed to delete signer owner '{label}'"))?;
    if affected == 0 {
        bail!("signer owner '{label}' is not configured");
    }
    Ok(())
}

/// Delete a signer owner by stable id.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `id` - Stable signer-owner id to remove.
///
/// # Errors
/// Returns an error if the id is unknown or deletion fails.
///
/// # Examples
///
/// ```ignore
/// signer_owners::delete_by_id(&conn, owner_id)?;
/// ```
pub fn delete_by_id(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn
        .execute("DELETE FROM signer_owners WHERE id = ?1", params![id])
        .with_context(|| format!("failed to delete signer owner id '{id}'"))?;
    if affected == 0 {
        bail!("signer owner id '{id}' is not configured");
    }
    Ok(())
}

/// Create a signer-owner vault for an existing signer owner.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `signer_owner_id` - Existing signer-owner id.
/// * `password` - Local password used to protect the owner DEK.
///
/// # Returns
/// The generated owner DEK so callers can immediately store owner-kind secret
/// payloads in the same operation.
///
/// # Errors
/// Returns an error if the signer owner is unknown, the vault already exists,
/// key wrapping fails, or SQLite insertion fails.
///
/// # Examples
///
/// ```ignore
/// let dek = signer_owners::create_vault(&conn, &owner.id, password)?;
/// ```
pub fn create_vault(
    conn: &Connection,
    signer_owner_id: &str,
    password: &str,
) -> Result<Zeroizing<[u8; KEY_LEN]>> {
    if find_by_id(conn, signer_owner_id)?.is_none() {
        bail!("signer owner id '{signer_owner_id}' is not configured");
    }

    let params = Argon2Params::default();
    let salt = random_salt();
    let dek = generate_dek();
    let kek = derive_kek(password, &salt, &params)?;
    let aad = object_aad(signer_owner_id, SIGNER_OWNER_DEK_KIND, CIPHER_VERSION);
    let (encrypted_dek, dek_nonce) = aead_encrypt(&kek, &*dek, &aad)?;
    let kdf_params_json =
        serde_json::to_string(&params).context("failed to serialise KDF params")?;
    let now = now_unix_seconds()?;

    conn.execute(
        "INSERT INTO signer_owner_vaults (
            signer_owner_id, kdf_algorithm, kdf_params_json, salt, encrypted_dek,
            dek_nonce, cipher_version, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            signer_owner_id,
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
    .with_context(|| format!("failed to create signer owner vault for '{signer_owner_id}'"))?;

    Ok(dek)
}

/// Unlock a signer-owner vault by owner label.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `label` - Signer-owner label.
/// * `password` - Local password for the signer owner.
///
/// # Returns
/// An unlocked signer-owner context containing the owner metadata and DEK.
///
/// # Errors
/// Returns an error if the owner or vault is missing, the password is wrong, or
/// decryption fails.
///
/// # Examples
///
/// ```ignore
/// let unlocked = signer_owners::unlock_by_label(&conn, "main", password)?;
/// ```
pub fn unlock_by_label(
    conn: &Connection,
    label: &str,
    password: &str,
) -> Result<UnlockedSignerOwner> {
    let record = find_by_label(conn, label)?
        .with_context(|| format!("signer owner '{label}' is not configured"))?;
    unlock_record(conn, record, password)
}

/// Unlock a signer-owner vault by owner id.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `signer_owner_id` - Stable signer-owner id.
/// * `password` - Local password for the signer owner.
///
/// # Returns
/// An unlocked signer-owner context containing the owner metadata and DEK.
///
/// # Errors
/// Returns an error if the owner or vault is missing, the password is wrong, or
/// decryption fails.
///
/// # Examples
///
/// ```ignore
/// let unlocked = signer_owners::unlock_by_id(&conn, owner_id, password)?;
/// ```
pub fn unlock_by_id(
    conn: &Connection,
    signer_owner_id: &str,
    password: &str,
) -> Result<UnlockedSignerOwner> {
    let record = find_by_id(conn, signer_owner_id)?
        .with_context(|| format!("signer owner id '{signer_owner_id}' is not configured"))?;
    unlock_record(conn, record, password)
}

/// Change a signer-owner local password.
///
/// Password changes re-wrap the existing owner DEK and do not re-encrypt child
/// payloads.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `label` - Signer-owner label.
/// * `old_password` - Current local password.
/// * `new_password` - New local password.
///
/// # Errors
/// Returns an error if the owner cannot be unlocked with `old_password` or the
/// vault update fails.
///
/// # Examples
///
/// ```ignore
/// signer_owners::change_password(&conn, "main", old_password, new_password)?;
/// ```
pub fn change_password(
    conn: &Connection,
    label: &str,
    old_password: &str,
    new_password: &str,
) -> Result<()> {
    let unlocked = unlock_by_label(conn, label, old_password)?;
    let params = Argon2Params::default();
    let salt = random_salt();
    let kek = derive_kek(new_password, &salt, &params)?;
    let aad = object_aad(&unlocked.record.id, SIGNER_OWNER_DEK_KIND, CIPHER_VERSION);
    let (encrypted_dek, dek_nonce) = aead_encrypt(&kek, &*unlocked.dek, &aad)?;
    let kdf_params_json =
        serde_json::to_string(&params).context("failed to serialise KDF params")?;
    let now = now_unix_seconds()?;

    conn.execute(
        "UPDATE signer_owner_vaults
         SET kdf_algorithm = ?1, kdf_params_json = ?2, salt = ?3,
             encrypted_dek = ?4, dek_nonce = ?5, cipher_version = ?6, updated_at = ?7
         WHERE signer_owner_id = ?8",
        params![
            KDF_ALGORITHM,
            kdf_params_json,
            salt.as_slice(),
            encrypted_dek,
            dek_nonce.as_slice(),
            CIPHER_VERSION,
            now,
            unlocked.record.id,
        ],
    )
    .with_context(|| format!("failed to change password for signer owner '{label}'"))?;

    Ok(())
}

/// Store an encrypted seed secret for a seed signer owner.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `signer_owner_id` - Seed signer-owner id.
/// * `owner_dek` - Unlocked signer-owner DEK.
/// * `secret` - Plaintext seed secret bytes.
///
/// # Errors
/// Returns an error if encryption or insertion fails.
///
/// # Examples
///
/// ```ignore
/// signer_owners::insert_seed_secret(&conn, &owner.id, &dek, mnemonic.as_bytes())?;
/// ```
pub fn insert_seed_secret(
    conn: &Connection,
    signer_owner_id: &str,
    owner_dek: &[u8; KEY_LEN],
    secret: &[u8],
) -> Result<()> {
    let aad = object_aad(signer_owner_id, SEED_OWNER_SECRET_KIND, CIPHER_VERSION);
    let (payload_ciphertext, payload_nonce) = aead_encrypt(owner_dek, secret, &aad)?;
    conn.execute(
        "INSERT INTO seed_owner_secrets (
            signer_owner_id, cipher_version, payload_ciphertext, payload_nonce
        ) VALUES (?1, ?2, ?3, ?4)",
        params![
            signer_owner_id,
            CIPHER_VERSION,
            payload_ciphertext,
            payload_nonce.as_slice(),
        ],
    )
    .with_context(|| format!("failed to insert seed owner secret for '{signer_owner_id}'"))?;
    Ok(())
}

/// Decrypt a seed signer owner's secret payload.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `signer_owner_id` - Seed signer-owner id.
/// * `owner_dek` - Unlocked signer-owner DEK.
///
/// # Returns
/// Plaintext seed secret bytes wrapped for zeroization on drop.
///
/// # Errors
/// Returns an error if the seed secret row is missing or decryption fails.
///
/// # Examples
///
/// ```ignore
/// let seed = signer_owners::decrypt_seed_secret(&conn, &owner.id, &unlocked.dek)?;
/// ```
pub fn decrypt_seed_secret(
    conn: &Connection,
    signer_owner_id: &str,
    owner_dek: &[u8; KEY_LEN],
) -> Result<Zeroizing<Vec<u8>>> {
    let record = conn
        .query_row(
            "SELECT cipher_version, payload_ciphertext, payload_nonce
             FROM seed_owner_secrets WHERE signer_owner_id = ?1",
            params![signer_owner_id],
            |row| {
                Ok(SeedOwnerSecretRecord {
                    signer_owner_id: signer_owner_id.to_owned(),
                    cipher_version: row.get(0)?,
                    payload_ciphertext: row.get(1)?,
                    payload_nonce: row.get(2)?,
                })
            },
        )
        .optional()
        .with_context(|| format!("failed to query seed owner secret for '{signer_owner_id}'"))?
        .with_context(|| format!("signer owner '{signer_owner_id}' has no seed secret"))?;
    let aad = object_aad(
        signer_owner_id,
        SEED_OWNER_SECRET_KIND,
        record.cipher_version,
    );
    aead_decrypt(
        owner_dek,
        &record.payload_nonce,
        &record.payload_ciphertext,
        &aad,
    )
    .with_context(|| format!("failed to decrypt seed owner secret for '{signer_owner_id}'"))
}

/// Insert Ledger owner details.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `details` - Ledger enrollment metadata to persist.
///
/// # Errors
/// Returns an error if insertion fails, including duplicate canonical public
/// keys.
///
/// # Examples
///
/// ```ignore
/// signer_owners::insert_ledger_details(&conn, details)?;
/// ```
pub fn insert_ledger_details(
    conn: &Connection,
    details: NewLedgerOwnerDetails<'_>,
) -> Result<LedgerOwnerDetailsRecord> {
    let now = now_unix_seconds()?;
    conn.execute(
        "INSERT INTO ledger_owner_details (
            signer_owner_id, canonical_public_key, fingerprint, enrollment_path,
            app_name, created_at, updated_at, last_seen_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)",
        params![
            details.signer_owner_id,
            details.canonical_public_key,
            details.fingerprint,
            details.enrollment_path,
            details.app_name,
            now,
            now,
        ],
    )
    .with_context(|| {
        format!(
            "failed to insert Ledger owner details for '{}'",
            details.signer_owner_id
        )
    })?;

    Ok(LedgerOwnerDetailsRecord {
        signer_owner_id: details.signer_owner_id.to_owned(),
        canonical_public_key: details.canonical_public_key.to_vec(),
        fingerprint: details.fingerprint.to_owned(),
        enrollment_path: details.enrollment_path.to_owned(),
        app_name: details.app_name.map(str::to_owned),
        created_at: now,
        updated_at: now,
        last_seen_at: None,
    })
}

/// Find Ledger owner details by signer owner id.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `signer_owner_id` - Ledger signer-owner id.
///
/// # Returns
/// Ledger details when present.
///
/// # Errors
/// Returns an error if querying or decoding details fails.
///
/// # Examples
///
/// ```ignore
/// let details = signer_owners::find_ledger_details_by_owner_id(&conn, &owner.id)?;
/// ```
pub fn find_ledger_details_by_owner_id(
    conn: &Connection,
    signer_owner_id: &str,
) -> Result<Option<LedgerOwnerDetailsRecord>> {
    conn.query_row(
        "SELECT signer_owner_id, canonical_public_key, fingerprint, enrollment_path,
                app_name, created_at, updated_at, last_seen_at
         FROM ledger_owner_details WHERE signer_owner_id = ?1",
        params![signer_owner_id],
        ledger_details_from_row,
    )
    .optional()
    .with_context(|| format!("failed to query Ledger details for '{signer_owner_id}'"))
}

/// Find Ledger owner details by canonical public key.
///
/// # Arguments
/// * `conn` - Open wallet database connection.
/// * `canonical_public_key` - Full canonical public key returned by the Ledger app.
///
/// # Returns
/// Ledger details when the key is enrolled.
///
/// # Errors
/// Returns an error if querying or decoding details fails.
///
/// # Examples
///
/// ```ignore
/// let details = signer_owners::find_ledger_details_by_canonical_public_key(&conn, key)?;
/// ```
pub fn find_ledger_details_by_canonical_public_key(
    conn: &Connection,
    canonical_public_key: &[u8],
) -> Result<Option<LedgerOwnerDetailsRecord>> {
    conn.query_row(
        "SELECT signer_owner_id, canonical_public_key, fingerprint, enrollment_path,
                app_name, created_at, updated_at, last_seen_at
         FROM ledger_owner_details WHERE canonical_public_key = ?1",
        params![canonical_public_key],
        ledger_details_from_row,
    )
    .optional()
    .context("failed to query Ledger details by canonical public key")
}

fn unlock_record(
    conn: &Connection,
    record: SignerOwnerRecord,
    password: &str,
) -> Result<UnlockedSignerOwner> {
    let vault = load_vault(conn, &record.id)?;
    if vault.kdf_algorithm != KDF_ALGORITHM {
        bail!(
            "unsupported signer owner vault KDF '{}'",
            vault.kdf_algorithm
        );
    }
    let kek = derive_kek(password, &vault.salt, &vault.kdf_params)?;
    let aad = object_aad(&record.id, SIGNER_OWNER_DEK_KIND, vault.cipher_version);
    let dek = aead_decrypt(&kek, &vault.dek_nonce, &vault.encrypted_dek, &aad)
        .with_context(|| format!("failed to unlock signer owner '{}'", record.label))?;
    Ok(UnlockedSignerOwner {
        record,
        dek: zeroizing_array_from_slice::<KEY_LEN>(&dek, "signer owner DEK")?,
    })
}

fn load_vault(conn: &Connection, signer_owner_id: &str) -> Result<SignerOwnerVaultRecord> {
    conn.query_row(
        "SELECT signer_owner_id, kdf_algorithm, kdf_params_json, salt, encrypted_dek,
                dek_nonce, cipher_version, created_at, updated_at
         FROM signer_owner_vaults WHERE signer_owner_id = ?1",
        params![signer_owner_id],
        |row| {
            let kdf_params_json: String = row.get(2)?;
            let kdf_params =
                serde_json::from_str::<Argon2Params>(&kdf_params_json).map_err(|err| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Text,
                        Box::new(err),
                    )
                })?;
            Ok(SignerOwnerVaultRecord {
                signer_owner_id: row.get(0)?,
                kdf_algorithm: row.get(1)?,
                kdf_params,
                salt: row.get(3)?,
                encrypted_dek: row.get(4)?,
                dek_nonce: row.get(5)?,
                cipher_version: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        },
    )
    .optional()
    .with_context(|| format!("failed to query signer owner vault for '{signer_owner_id}'"))?
    .with_context(|| format!("signer owner '{signer_owner_id}' has no vault"))
}

fn ledger_details_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<LedgerOwnerDetailsRecord> {
    Ok(LedgerOwnerDetailsRecord {
        signer_owner_id: row.get(0)?,
        canonical_public_key: row.get(1)?,
        fingerprint: row.get(2)?,
        enrollment_path: row.get(3)?,
        app_name: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
        last_seen_at: row.get(7)?,
    })
}

fn signer_owner_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SignerOwnerRecord> {
    let kind: String = row.get(1)?;
    let kind = SignerOwnerKind::from_str(&kind).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                err.to_string(),
            )),
        )
    })?;
    Ok(SignerOwnerRecord {
        id: row.get(0)?,
        kind,
        label: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

fn now_unix_seconds() -> Result<i64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?
        .as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::migrations;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    #[test]
    fn ledger_owner_fingerprint_is_short_hash_prefix() {
        let fingerprint = ledger_owner_fingerprint(&[7; 32]);
        assert_eq!(fingerprint.len(), 8);
        assert_eq!(fingerprint, ledger_owner_fingerprint(&[7; 32]));
        assert_ne!(fingerprint, ledger_owner_fingerprint(&[8; 32]));
        assert_eq!(LEDGER_OWNER_ENROLLMENT_PATH, "m/44'/919'/0'/0'/0'");
    }

    #[test]
    fn create_list_find_rename_and_delete_owner() {
        let conn = conn();
        let owner = create(&conn, SignerOwnerKind::Seed, "main").unwrap();
        assert_eq!(owner.kind, SignerOwnerKind::Seed);
        assert_eq!(owner.label, "main");

        let by_label = find_by_label(&conn, "main").unwrap().unwrap();
        assert_eq!(by_label.id, owner.id);
        let by_id = find_by_id(&conn, &owner.id).unwrap().unwrap();
        assert_eq!(by_id.label, "main");
        assert_eq!(list(&conn).unwrap().len(), 1);

        let duplicate = create(&conn, SignerOwnerKind::Ledger, "main").unwrap_err();
        assert!(duplicate.to_string().contains("already exists"));

        rename(&conn, "main", "daily").unwrap();
        assert!(find_by_label(&conn, "main").unwrap().is_none());
        assert!(find_by_label(&conn, "daily").unwrap().is_some());

        delete_by_label(&conn, "daily").unwrap();
        assert!(list(&conn).unwrap().is_empty());
    }

    #[test]
    fn vault_unlock_wrong_password_and_password_change() {
        let conn = conn();
        let owner = create(&conn, SignerOwnerKind::Seed, "main").unwrap();
        let original_dek = create_vault(&conn, &owner.id, "old-password").unwrap();

        let unlocked = unlock_by_label(&conn, "main", "old-password").unwrap();
        assert_eq!(unlocked.record.id, owner.id);
        assert_eq!(&*unlocked.dek, &*original_dek);
        assert!(unlock_by_label(&conn, "main", "wrong-password").is_err());

        change_password(&conn, "main", "old-password", "new-password").unwrap();
        assert!(unlock_by_label(&conn, "main", "old-password").is_err());
        let unlocked = unlock_by_label(&conn, "main", "new-password").unwrap();
        assert_eq!(&*unlocked.dek, &*original_dek);
    }

    #[test]
    fn signer_owner_vaults_are_independent_domains() {
        let conn = conn();
        let seed = create(&conn, SignerOwnerKind::Seed, "seed").unwrap();
        let ledger = create(&conn, SignerOwnerKind::Ledger, "ledger").unwrap();
        let seed_dek = create_vault(&conn, &seed.id, "seed-password").unwrap();
        let ledger_dek = create_vault(&conn, &ledger.id, "ledger-password").unwrap();

        assert_ne!(&*seed_dek, &*ledger_dek);
        assert!(unlock_by_id(&conn, &seed.id, "ledger-password").is_err());
        assert!(unlock_by_id(&conn, &ledger.id, "seed-password").is_err());
    }

    #[test]
    fn seed_secret_round_trip_and_aad_binding() {
        let conn = conn();
        let owner = create(&conn, SignerOwnerKind::Seed, "main").unwrap();
        let dek = create_vault(&conn, &owner.id, "password").unwrap();
        insert_seed_secret(&conn, &owner.id, &dek, b"seed secret").unwrap();

        let secret = decrypt_seed_secret(&conn, &owner.id, &dek).unwrap();
        assert_eq!(&**secret, b"seed secret");

        let other = create(&conn, SignerOwnerKind::Seed, "other").unwrap();
        let other_dek = create_vault(&conn, &other.id, "password").unwrap();
        assert!(decrypt_seed_secret(&conn, &owner.id, &other_dek).is_err());
    }

    #[test]
    fn ledger_details_insert_lookup_and_duplicate_key_rejection() {
        let conn = conn();
        let owner = create(&conn, SignerOwnerKind::Ledger, "ledger").unwrap();
        create_vault(&conn, &owner.id, "password").unwrap();
        let key = [7u8; 32];
        let details = insert_ledger_details(
            &conn,
            NewLedgerOwnerDetails {
                signer_owner_id: &owner.id,
                canonical_public_key: &key,
                fingerprint: "07070707",
                enrollment_path: "m/44'/919'/0'/0'/0'",
                app_name: Some("Concordium"),
            },
        )
        .unwrap();
        assert_eq!(details.canonical_public_key, key);

        let by_owner = find_ledger_details_by_owner_id(&conn, &owner.id)
            .unwrap()
            .unwrap();
        assert_eq!(by_owner.fingerprint, "07070707");
        let by_key = find_ledger_details_by_canonical_public_key(&conn, &key)
            .unwrap()
            .unwrap();
        assert_eq!(by_key.signer_owner_id, owner.id);

        let second = create(&conn, SignerOwnerKind::Ledger, "ledger-2").unwrap();
        create_vault(&conn, &second.id, "password").unwrap();
        assert!(
            insert_ledger_details(
                &conn,
                NewLedgerOwnerDetails {
                    signer_owner_id: &second.id,
                    canonical_public_key: &key,
                    fingerprint: "07070707",
                    enrollment_path: "m/44'/919'/0'/0'/0'",
                    app_name: None,
                },
            )
            .is_err()
        );
    }

    #[test]
    fn deleting_owner_cascades_vault_and_kind_details() {
        let conn = conn();
        let owner = create(&conn, SignerOwnerKind::Seed, "main").unwrap();
        let dek = create_vault(&conn, &owner.id, "password").unwrap();
        insert_seed_secret(&conn, &owner.id, &dek, b"seed secret").unwrap();

        delete_by_id(&conn, &owner.id).unwrap();
        for table in ["signer_owner_vaults", "seed_owner_secrets"] {
            let count: u32 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 0);
        }
    }
}
