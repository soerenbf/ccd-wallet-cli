//! Public request and response types for Concordium Ledger commands.

use crate::error::{LedgerError, Result};
use std::{fmt, str::FromStr};

const HARDENED_OFFSET: u32 = 0x8000_0000;

/// Concordium Ledger derivation path encoded as BIP32 path indices.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivationPath {
    indices: Vec<u32>,
}

impl DerivationPath {
    /// Construct a derivation path from raw BIP32 indices.
    ///
    /// # Arguments
    ///
    /// * `indices` - BIP32 path indices, including hardened offsets where needed.
    ///
    /// # Errors
    ///
    /// Returns an error if the path has more than 255 components, which cannot be encoded in
    /// the Ledger app path format.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger::DerivationPath;
    /// let path = DerivationPath::new([44 + 0x8000_0000, 919 + 0x8000_0000]).unwrap();
    /// assert_eq!(path.indices().len(), 2);
    /// ```
    pub fn new(indices: impl IntoIterator<Item = u32>) -> Result<Self> {
        let indices = indices.into_iter().collect::<Vec<_>>();
        if indices.len() > u8::MAX as usize {
            return Err(LedgerError::invalid_request(format!(
                "derivation path has {} components; maximum is 255",
                indices.len()
            )));
        }
        Ok(Self { indices })
    }

    /// Construct a canonical hardened Concordium account path.
    ///
    /// The resulting path is `m/44'/coin_type'/idp_index'/identity_index'/credential_index'`.
    ///
    /// # Arguments
    ///
    /// * `coin_type` - Concordium coin type (`919` for mainnet, `1` for testnet-like networks).
    /// * `idp_index` - Identity provider index.
    /// * `identity_index` - Identity index.
    /// * `credential_index` - Credential/account index.
    ///
    /// # Errors
    ///
    /// Returns an error if any index is already hardened and therefore cannot be hardened again.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger::DerivationPath;
    /// let path = DerivationPath::concordium_account(919, 0, 0, 0).unwrap();
    /// assert_eq!(path.indices().len(), 5);
    /// ```
    pub fn concordium_account(
        coin_type: u32,
        idp_index: u32,
        identity_index: u32,
        credential_index: u32,
    ) -> Result<Self> {
        Self::new([
            harden(44)?,
            harden(coin_type)?,
            harden(idp_index)?,
            harden(identity_index)?,
            harden(credential_index)?,
        ])
    }

    /// Return the raw path indices.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger::DerivationPath;
    /// let path: DerivationPath = "m/44'/919'/0'".parse().unwrap();
    /// assert_eq!(path.indices().len(), 3);
    /// ```
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// Serialize the path into the Ledger app path format.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger::DerivationPath;
    /// let path = DerivationPath::new([1, 2]).unwrap();
    /// assert_eq!(path.to_ledger_bytes(), vec![2, 0, 0, 0, 1, 0, 0, 0, 2]);
    /// ```
    pub fn to_ledger_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(1 + self.indices.len() * 4);
        bytes.push(self.indices.len() as u8);
        for index in &self.indices {
            bytes.extend_from_slice(&index.to_be_bytes());
        }
        bytes
    }
}

impl FromStr for DerivationPath {
    type Err = LedgerError;

    fn from_str(value: &str) -> Result<Self> {
        let normalized = value.strip_prefix("m/").unwrap_or(value);
        if normalized.is_empty() {
            return Err(LedgerError::invalid_request("derivation path is empty"));
        }
        let mut indices = Vec::new();
        for component in normalized.split('/') {
            if component.is_empty() {
                return Err(LedgerError::invalid_request(
                    "derivation path contains an empty component",
                ));
            }
            let hardened =
                component.ends_with('\'') || component.ends_with('h') || component.ends_with('H');
            let number = if hardened {
                &component[..component.len() - 1]
            } else {
                component
            };
            let mut index = number.parse::<u32>().map_err(|err| {
                LedgerError::invalid_request(format!(
                    "invalid derivation path component '{component}': {err}"
                ))
            })?;
            if hardened {
                index = harden(index)?;
            }
            indices.push(index);
        }
        Self::new(indices)
    }
}

impl fmt::Display for DerivationPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("m")?;
        for index in &self.indices {
            if index & HARDENED_OFFSET != 0 {
                write!(f, "/{}'", index - HARDENED_OFFSET)?;
            } else {
                write!(f, "/{index}")?;
            }
        }
        Ok(())
    }
}

/// Harden a BIP32 path index.
///
/// # Arguments
///
/// * `index` - Non-hardened index.
///
/// # Errors
///
/// Returns an error if `index` already has the hardened bit set.
///
/// # Examples
///
/// ```
/// use ccd_wallet_ledger::harden;
/// assert_eq!(harden(44).unwrap(), 44 + 0x8000_0000);
/// ```
pub fn harden(index: u32) -> Result<u32> {
    if index >= HARDENED_OFFSET {
        return Err(LedgerError::invalid_request(format!(
            "derivation path index {index} is too large to harden"
        )));
    }
    Ok(index + HARDENED_OFFSET)
}

/// Options for public-key retrieval.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PublicKeyOptions {
    /// Whether the Ledger device should ask the user to confirm the public key.
    pub confirm_on_device: bool,
    /// Whether the Ledger app should include a signature over the returned public key.
    pub signed_key: bool,
}

/// Request for retrieving a Concordium public key from the Ledger app.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicKeyRequest {
    /// Derivation path for the requested key.
    pub path: DerivationPath,
    /// Public-key command options.
    pub options: PublicKeyOptions,
}

/// Public-key response returned by the Ledger app.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicKeyResponse {
    /// Raw 32-byte Ed25519 public key bytes.
    pub public_key: [u8; 32],
    /// Optional device signature over the public key.
    pub signed_public_key: Option<Vec<u8>>,
}

/// Raw 64-byte signature returned by a signing-oriented Ledger command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawSignature(pub [u8; 64]);

impl RawSignature {
    /// Parse a raw response into a 64-byte signature.
    ///
    /// # Arguments
    ///
    /// * `bytes` - Response bytes returned by a signing command.
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError::UserDeclined`] for the one-byte decline sentinel used by
    /// reference clients, or [`LedgerError::InvalidSignatureLength`] for other non-64-byte
    /// responses.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger::RawSignature;
    /// let signature = RawSignature::from_response(vec![7; 64]).unwrap();
    /// assert_eq!(signature.0, [7; 64]);
    /// ```
    pub fn from_response(bytes: Vec<u8>) -> Result<Self> {
        if bytes.len() == 1 {
            return Err(LedgerError::UserDeclined);
        }
        let actual_len = bytes.len();
        let signature = bytes
            .try_into()
            .map_err(|_| LedgerError::InvalidSignatureLength { actual_len })?;
        Ok(Self(signature))
    }
}

/// Request for commands that sign a canonically serialized transaction in generic chunks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChunkedSigningRequest {
    /// Derivation path for the signing key.
    pub path: DerivationPath,
    /// Canonically serialized transaction bytes expected by the Ledger app for this command.
    pub transaction: Vec<u8>,
}

impl ChunkedSigningRequest {
    /// Construct a generic chunked-signing request.
    ///
    /// # Arguments
    ///
    /// * `path` - Derivation path for the signing key.
    /// * `transaction` - Canonically serialized transaction bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if `transaction` is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger::{ChunkedSigningRequest, DerivationPath};
    /// let request = ChunkedSigningRequest::new(DerivationPath::new([1]).unwrap(), vec![2]).unwrap();
    /// assert_eq!(request.transaction, vec![2]);
    /// ```
    pub fn new(path: DerivationPath, transaction: Vec<u8>) -> Result<Self> {
        if transaction.is_empty() {
            return Err(LedgerError::invalid_request(
                "serialized transaction is empty",
            ));
        }
        Ok(Self { path, transaction })
    }
}

/// Request for transfer-with-memo signing using the Ledger app's staged flow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferWithMemoSigningRequest {
    /// Serialized path-prefixed header, recipient address, transaction kind, and memo-length bytes.
    pub header_address_memo_length: Vec<u8>,
    /// Serialized memo bytes without its length prefix.
    pub memo: Vec<u8>,
    /// Serialized amount bytes.
    pub amount: Vec<u8>,
}

/// Request for transfer-with-schedule signing using the Ledger app's staged flow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduledTransferSigningRequest {
    /// Serialized path-prefixed header, recipient address, transaction kind, and schedule length.
    pub header_address_schedule_length: Vec<u8>,
    /// Serialized schedule pair bytes.
    pub schedule: Vec<u8>,
}

/// Request for transfer-with-schedule-and-memo signing using the Ledger app's staged flow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduledTransferWithMemoSigningRequest {
    /// Serialized path-prefixed header, recipient address, transaction kind, schedule length, and memo length.
    pub header_address_schedule_memo_length: Vec<u8>,
    /// Serialized memo bytes without its length prefix.
    pub memo: Vec<u8>,
    /// Serialized schedule pair bytes.
    pub schedule: Vec<u8>,
}

/// Request for configure-baker signing using the Ledger app's staged flow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigureBakerSigningRequest {
    /// Serialized path-prefixed header, transaction kind, and update bitmap bytes.
    pub header_kind_and_bitmap: Vec<u8>,
    /// Serialized stake/restake/open-for-delegation/election/signature key stage bytes.
    pub first_batch: Vec<u8>,
    /// Serialized aggregation key bytes.
    pub aggregation_keys: Vec<u8>,
    /// Serialized metadata URL length bytes.
    pub url_length: Vec<u8>,
    /// Serialized metadata URL bytes.
    pub url: Vec<u8>,
    /// Serialized commission-fee bytes.
    pub commission_fee: Vec<u8>,
    /// Serialized suspended flag bytes.
    pub suspended: Vec<u8>,
}

/// Request for register-data signing using the Ledger app's staged flow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisterDataSigningRequest {
    /// Serialized path-prefixed header, transaction kind, and data length bytes.
    pub header: Vec<u8>,
    /// Serialized data bytes without the length prefix.
    pub data: Vec<u8>,
}

/// Request for transfer-to-public signing using the Ledger app's staged flow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferToPublicSigningRequest {
    /// Serialized path-prefixed header and transaction kind bytes.
    pub header: Vec<u8>,
    /// Serialized remaining amount, transfer amount, recipient, index, and proof-length bytes.
    pub amount_recipient_proofs_length: Vec<u8>,
    /// Serialized proof bytes.
    pub proofs: Vec<u8>,
}

/// Request for deploy-module signing using the Ledger app's staged flow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeployModuleSigningRequest {
    /// Derivation path for the signing key.
    pub path: DerivationPath,
    /// Serialized transaction header, kind, module version, and source length bytes.
    pub header_and_version: Vec<u8>,
    /// Serialized module source bytes.
    pub source: Vec<u8>,
}

/// Request for init-contract or update-contract signing using staged name and parameter uploads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractSigningRequest {
    /// Derivation path for the signing key.
    pub path: DerivationPath,
    /// Serialized transaction header and fixed-size contract payload prefix bytes.
    pub header_and_data: Vec<u8>,
    /// Contract init or receive name bytes, without the two-byte length prefix.
    pub name: Vec<u8>,
    /// Contract parameter bytes, without the two-byte length prefix.
    pub parameter: Vec<u8>,
}

/// Request for public-info-for-IP signing using the Ledger app's staged flow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicInfoForIpSigningRequest {
    /// Serialized path, id credential public value, registration ID, and public-key count bytes.
    pub initial: Vec<u8>,
    /// Serialized key index, scheme, and verification-key entries.
    pub keys: Vec<Vec<u8>>,
    /// Serialized signature threshold bytes.
    pub threshold: Vec<u8>,
}

/// One staged credential payload used by credential-deployment and update-credentials flows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CredentialSigningPayload {
    /// Serialized number of verification keys.
    pub verification_key_count: Vec<u8>,
    /// Serialized key index, scheme, and verification-key fields.
    pub key_index_scheme_public_key: Vec<u8>,
    /// Serialized signature threshold, credential ID, identity provider ID, and AR-data count.
    pub threshold_credential_id_identity: Vec<u8>,
    /// Serialized anonymity revoker identity and encrypted ID credential public share fields.
    pub ar_identity: Vec<u8>,
    /// Serialized valid-to, created-at, and revealed-attribute count fields.
    pub credential_dates: Vec<u8>,
    /// Serialized revealed attributes.
    pub attributes: Vec<CredentialAttribute>,
    /// Serialized proof-length field.
    pub proof_length: Vec<u8>,
    /// Serialized proof bytes.
    pub proofs: Vec<u8>,
}

/// Revealed credential attribute staged for Ledger upload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CredentialAttribute {
    /// Serialized attribute tag byte.
    pub tag: Vec<u8>,
    /// Serialized attribute value bytes.
    pub value: Vec<u8>,
}

/// New or existing account context for credential-deployment signing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CredentialDeploymentContext {
    /// New account deployment with serialized expiry bytes.
    New { expiry: Vec<u8> },
    /// Existing account credential with raw account-address bytes.
    Existing { account_address: Vec<u8> },
}

/// Request for credential-deployment signing using the Ledger app's staged flow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CredentialDeploymentSigningRequest {
    /// Serialized derivation path bytes.
    pub path: Vec<u8>,
    /// Staged credential payload fields.
    pub credential: CredentialSigningPayload,
    /// Whether the credential is for a new or existing account, including its context bytes.
    pub context: CredentialDeploymentContext,
}

/// One credential entry in an update-credentials transaction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateCredentialEntry {
    /// Serialized credential index bytes.
    pub credential_index: Vec<u8>,
    /// Staged credential payload fields.
    pub credential: CredentialSigningPayload,
}

/// Request for update-credentials signing using the Ledger app's staged flow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateCredentialsSigningRequest {
    /// Serialized path-prefixed header, transaction kind, and new-credential count bytes.
    pub header_kind_and_index_length: Vec<u8>,
    /// New credential entries to upload.
    pub new_credentials: Vec<UpdateCredentialEntry>,
    /// Serialized removed-credential ID count bytes.
    pub credential_id_count: Vec<u8>,
    /// Serialized removed credential IDs.
    pub credential_ids: Vec<Vec<u8>>,
    /// Serialized resulting account threshold bytes.
    pub threshold: Vec<u8>,
}

/// Request for current address verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifyAddressRequest {
    /// Serialized address-verification payload bytes.
    pub payload: Vec<u8>,
}

/// Request for legacy address verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyVerifyAddressRequest {
    /// Serialized legacy address-verification payload bytes.
    pub payload: Vec<u8>,
}

/// Request for legacy private-key export.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportPrivateKeyLegacyRequest {
    /// Display/export mode passed as APDU P1.
    pub mode: u8,
    /// Export type passed as APDU P2.
    pub export_type: u8,
    /// Serialized identity payload bytes.
    pub payload: Vec<u8>,
}

/// New private-key export type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportPrivateKeyNewType {
    /// Identity credential creation key material.
    IdentityCredentialCreation,
    /// Account creation key material.
    AccountCreation,
    /// Identity recovery key material.
    IdRecovery,
    /// Account credential discovery key material.
    AccountCredentialDiscovery,
    /// Zero-knowledge proof creation key material.
    CreationOfZkProof,
}

/// Request for new private-key export.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportPrivateKeyNewRequest {
    /// Export type used to select APDU P1.
    pub export_type: ExportPrivateKeyNewType,
    /// Serialized export payload bytes.
    pub payload: Vec<u8>,
}

/// Raw 32-byte account address bytes used by optional SDK conversions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccountAddressBytes(pub [u8; 32]);

#[cfg(feature = "sdk")]
impl From<concordium_rust_sdk::common::types::AccountAddress> for AccountAddressBytes {
    fn from(value: concordium_rust_sdk::common::types::AccountAddress) -> Self {
        Self(value.0)
    }
}
