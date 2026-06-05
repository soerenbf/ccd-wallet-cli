//! Public request and response types for Governance Ledger commands.

use crate::error::{GovernanceLedgerError, Result};
use std::{fmt, str::FromStr};

const HARDENED_OFFSET: u32 = 0x8000_0000;

/// Governance Ledger derivation path encoded as BIP32 path indices.
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
    /// Returns an error if the path has more than 255 components.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger_governance::DerivationPath;
    /// let path = DerivationPath::new([44 + 0x8000_0000, 919 + 0x8000_0000]).unwrap();
    /// assert_eq!(path.indices().len(), 2);
    /// ```
    pub fn new(indices: impl IntoIterator<Item = u32>) -> Result<Self> {
        let indices = indices.into_iter().collect::<Vec<_>>();
        if indices.len() > u8::MAX as usize {
            return Err(GovernanceLedgerError::invalid_request(format!(
                "derivation path has {} components; maximum is 255",
                indices.len()
            )));
        }
        Ok(Self { indices })
    }

    /// Return the raw path indices.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger_governance::DerivationPath;
    /// let path: DerivationPath = "m/44'/919'/0'".parse().unwrap();
    /// assert_eq!(path.indices().len(), 3);
    /// ```
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// Serialize the path into the Governance Ledger app path format.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger_governance::DerivationPath;
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
    type Err = GovernanceLedgerError;

    fn from_str(value: &str) -> Result<Self> {
        let normalized = value.strip_prefix("m/").unwrap_or(value);
        if normalized.is_empty() {
            return Err(GovernanceLedgerError::invalid_request(
                "derivation path is empty",
            ));
        }
        let mut indices = Vec::new();
        for component in normalized.split('/') {
            if component.is_empty() {
                return Err(GovernanceLedgerError::invalid_request(
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
                GovernanceLedgerError::invalid_request(format!(
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
/// use ccd_wallet_ledger_governance::harden;
/// assert_eq!(harden(44).unwrap(), 44 + 0x8000_0000);
/// ```
pub fn harden(index: u32) -> Result<u32> {
    if index >= HARDENED_OFFSET {
        return Err(GovernanceLedgerError::invalid_request(format!(
            "derivation path index {index} is too large to harden"
        )));
    }
    Ok(index + HARDENED_OFFSET)
}

/// Options for Governance Ledger public-key retrieval.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PublicKeyOptions {
    /// Whether the Ledger device should ask the user to confirm the public key.
    pub confirm_on_device: bool,
    /// Whether the Ledger app should include a signature over the returned public key.
    pub signed_key: bool,
}

/// Request for retrieving a Governance Ledger public key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicKeyRequest {
    /// Derivation path for the requested key.
    pub path: DerivationPath,
    /// Public-key command options.
    pub options: PublicKeyOptions,
}

/// Public-key response returned by the Governance Ledger app.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicKeyResponse {
    /// Raw 32-byte Ed25519 public key bytes.
    pub public_key: [u8; 32],
    /// Optional device signature over the public key.
    pub signed_public_key: Option<Vec<u8>>,
}

/// Raw 64-byte signature returned by a signing-oriented Governance Ledger command.
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
    /// Returns user-decline for one-byte decline sentinels or invalid-length errors.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger_governance::RawSignature;
    /// let signature = RawSignature::from_response(vec![7; 64]).unwrap();
    /// assert_eq!(signature.0, [7; 64]);
    /// ```
    pub fn from_response(bytes: Vec<u8>) -> Result<Self> {
        if bytes.len() == 1 {
            return Err(GovernanceLedgerError::UserDeclined);
        }
        let actual_len = bytes.len();
        let signature = bytes
            .try_into()
            .map_err(|_| GovernanceLedgerError::InvalidSignatureLength { actual_len })?;
        Ok(Self(signature))
    }
}

/// Serialized 28-byte governance update instruction header consumed by the Governance Ledger app.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UpdateHeaderBytes(pub [u8; 28]);

impl UpdateHeaderBytes {
    /// Construct header bytes from an already serialized governance update header.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The 28 bytes expected by Governance Ledger update commands.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger_governance::UpdateHeaderBytes;
    /// let header = UpdateHeaderBytes::new([0; 28]);
    /// assert_eq!(header.as_ref().len(), 28);
    /// ```
    pub const fn new(bytes: [u8; 28]) -> Self {
        Self(bytes)
    }
}

impl AsRef<[u8]> for UpdateHeaderBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Common initial data shared by governance update signing commands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceUpdatePrefix {
    /// Derivation path for the signing key.
    pub path: DerivationPath,
    /// Serialized update instruction header.
    pub header: UpdateHeaderBytes,
    /// Governance update type tag expected by the device flow.
    pub update_type: u8,
}

impl GovernanceUpdatePrefix {
    /// Serialize path, update header, and update type for an initial APDU packet.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger_governance::{DerivationPath, GovernanceUpdatePrefix, UpdateHeaderBytes};
    /// let prefix = GovernanceUpdatePrefix { path: DerivationPath::new([1]).unwrap(), header: UpdateHeaderBytes::new([0; 28]), update_type: 1 };
    /// assert_eq!(prefix.to_ledger_bytes().len(), 1 + 4 + 28 + 1);
    /// ```
    pub fn to_ledger_bytes(&self) -> Vec<u8> {
        let mut bytes = self.path.to_ledger_bytes();
        bytes.extend_from_slice(self.header.as_ref());
        bytes.push(self.update_type);
        bytes
    }
}

/// Fixed-shape governance update request whose payload is sent in one APDU packet.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixedUpdateRequest {
    /// Common update prefix.
    pub prefix: GovernanceUpdatePrefix,
    /// Serialized update-family-specific payload bytes following the update type.
    pub payload: Vec<u8>,
}

/// Request for signing an exchange-rate update.
pub type ExchangeRateUpdateRequest = FixedUpdateRequest;
/// Request for signing a transaction-fee-distribution update.
pub type TransactionFeeDistributionUpdateRequest = FixedUpdateRequest;
/// Request for signing a GAS rewards update.
pub type GasRewardsUpdateRequest = FixedUpdateRequest;
/// Request for signing a foundation-account update.
pub type FoundationAccountUpdateRequest = FixedUpdateRequest;
/// Request for signing a mint-distribution update.
pub type MintDistributionUpdateRequest = FixedUpdateRequest;
/// Request for signing a baker-stake-threshold update.
pub type BakerStakeThresholdUpdateRequest = FixedUpdateRequest;
/// Request for signing a cooldown-parameters update.
pub type CooldownParametersUpdateRequest = FixedUpdateRequest;
/// Request for signing a pool-parameters update.
pub type PoolParametersUpdateRequest = FixedUpdateRequest;
/// Request for signing a time-parameters update.
pub type TimeParametersUpdateRequest = FixedUpdateRequest;
/// Request for signing a timeout-parameters update.
pub type TimeoutParametersUpdateRequest = FixedUpdateRequest;
/// Request for signing a minimum-block-time update.
pub type MinBlockTimeUpdateRequest = FixedUpdateRequest;
/// Request for signing a block-energy-limit update.
pub type BlockEnergyLimitUpdateRequest = FixedUpdateRequest;
/// Request for signing a finalization-committee-parameters update.
pub type FinalizationCommitteeParametersUpdateRequest = FixedUpdateRequest;
/// Request for signing a validator-score-parameters update.
pub type ValidatorScoreParametersUpdateRequest = FixedUpdateRequest;

/// Request for signing a protocol update.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolUpdateRequest {
    /// Common update prefix.
    pub prefix: GovernanceUpdatePrefix,
    /// Serialized payload length included in the initial packet.
    pub payload_length: u64,
    /// Human-readable protocol update message bytes.
    pub message: Vec<u8>,
    /// Specification URL bytes.
    pub specification_url: Vec<u8>,
    /// 32-byte specification hash.
    pub specification_hash: [u8; 32],
    /// Auxiliary data bytes chunked in the final stage.
    pub auxiliary_data: Vec<u8>,
}

/// Request for signing a create-PLT governance update.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatePltRequest {
    /// Common update prefix.
    pub prefix: GovernanceUpdatePrefix,
    /// Token identifier bytes prefixed by the client as one `u8` length.
    pub token_id: Vec<u8>,
    /// 32-byte token module reference.
    pub token_module: [u8; 32],
    /// Token decimals.
    pub decimals: u8,
    /// Token initialization parameters sent in chunks.
    pub initialization_parameters: Vec<u8>,
}

/// Description fields shared by add-anonymity-revoker and add-identity-provider updates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DescriptionFields {
    /// Name bytes.
    pub name: Vec<u8>,
    /// URL bytes.
    pub url: Vec<u8>,
    /// Description bytes.
    pub description: Vec<u8>,
}

/// Request for signing an add-anonymity-revoker update.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddAnonymityRevokerRequest {
    /// Common update prefix.
    pub prefix: GovernanceUpdatePrefix,
    /// Serialized total payload length.
    pub payload_length: u64,
    /// Serialized anonymity revoker info length.
    pub ar_info_length: u32,
    /// Anonymity revoker identity.
    pub ar_identity: u32,
    /// Description fields staged one at a time.
    pub description: DescriptionFields,
    /// 96-byte anonymity revoker public key.
    pub public_key: Vec<u8>,
}

/// Request for signing an add-identity-provider update.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddIdentityProviderRequest {
    /// Common update prefix.
    pub prefix: GovernanceUpdatePrefix,
    /// Serialized total payload length.
    pub payload_length: u64,
    /// Serialized identity provider info length.
    pub ip_info_length: u32,
    /// Identity provider identity.
    pub ip_identity: u32,
    /// Description fields staged one at a time.
    pub description: DescriptionFields,
    /// Serialized verify key bytes.
    pub verify_key: Vec<u8>,
    /// 32-byte CDI verify key.
    pub cdi_verify_key: [u8; 32],
}

/// Governance key with scheme identifier and raw 32-byte key material.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernancePublicKeyEntry {
    /// Scheme identifier accepted by the Governance Ledger app.
    pub scheme_id: u8,
    /// Raw public key bytes.
    pub key: [u8; 32],
}

/// Key-update discriminator for higher-level governance key update flows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HigherLevelKeyUpdateType {
    /// Authorize the update with root keys.
    RootKeys,
    /// Authorize the update with level-1 keys.
    Level1Keys,
}

impl HigherLevelKeyUpdateType {
    /// Return the device discriminator for a higher-level key update target.
    ///
    /// # Arguments
    ///
    /// * `update_type` - Governance update type byte (`0x0A` for root, `0x0B` for level 1).
    ///
    /// # Errors
    ///
    /// Returns an error if the signer kind is invalid for the targeted higher-level update flow.
    pub fn to_device_byte(self, update_type: u8) -> Result<u8> {
        match (self, update_type) {
            (Self::RootKeys, 0x0A) => Ok(0x00),
            (Self::RootKeys, 0x0B) => Ok(0x01),
            (Self::Level1Keys, 0x0B) => Ok(0x00),
            (Self::Level1Keys, 0x0A) => Err(GovernanceLedgerError::invalid_request(
                "root-key updates cannot be authorized with level-1 keys",
            )),
            (_, other) => Err(GovernanceLedgerError::invalid_request(format!(
                "unsupported higher-level update type 0x{other:02x} for key-update discriminator"
            ))),
        }
    }
}

/// Higher-level governance key update request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HigherLevelKeyUpdateRequest {
    /// Common update prefix.
    pub prefix: GovernanceUpdatePrefix,
    /// Typed higher-level key-update discriminator.
    pub key_update_type: HigherLevelKeyUpdateType,
    /// New governance keys.
    pub keys: Vec<GovernancePublicKeyEntry>,
    /// New threshold.
    pub threshold: u16,
}

/// Level-2 authorizations version selector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorizationsVersion {
    /// Authorizations V0.
    V0,
    /// Authorizations V1.
    V1,
    /// Authorizations V2.
    V2,
}

impl AuthorizationsVersion {
    /// Return the P2 selector used by the Governance Ledger app.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger_governance::AuthorizationsVersion;
    /// assert_eq!(AuthorizationsVersion::V2.p2(), 2);
    /// ```
    pub const fn p2(self) -> u8 {
        match self {
            Self::V0 => 0,
            Self::V1 => 1,
            Self::V2 => 2,
        }
    }
}

/// Key-update discriminator for level-2 authorizations update flows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorizationsKeyUpdateType {
    /// Authorize the level-2 update with root keys.
    RootKeys,
    /// Authorize the level-2 update with level-1 keys.
    Level1Keys,
}

impl AuthorizationsKeyUpdateType {
    /// Return the device discriminator byte for the authorizations version.
    ///
    /// # Arguments
    ///
    /// * `version` - Authorizations version targeted by the update.
    pub const fn to_device_byte(self, version: AuthorizationsVersion) -> u8 {
        match (self, version) {
            (Self::Level1Keys, AuthorizationsVersion::V0) => 0x01,
            (Self::RootKeys, AuthorizationsVersion::V0) => 0x02,
            (Self::Level1Keys, AuthorizationsVersion::V1) => 0x02,
            (Self::RootKeys, AuthorizationsVersion::V1) => 0x03,
            (Self::Level1Keys, AuthorizationsVersion::V2) => 0x03,
            (Self::RootKeys, AuthorizationsVersion::V2) => 0x04,
        }
    }
}

/// Access structure used in a level-2 authorizations update.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccessStructureUpdate {
    /// Key indices authorized for the structure.
    pub key_indices: Vec<u16>,
    /// Threshold for the structure.
    pub threshold: u16,
}

/// Level-2 authorizations update request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizationsUpdateRequest {
    /// Common update prefix.
    pub prefix: GovernanceUpdatePrefix,
    /// Typed authorizations key-update discriminator.
    pub key_update_type: AuthorizationsKeyUpdateType,
    /// Authorizations format targeted by the update.
    pub version: AuthorizationsVersion,
    /// New level-2 governance keys.
    pub keys: Vec<GovernancePublicKeyEntry>,
    /// Access structures in device order.
    pub access_structures: Vec<AccessStructureUpdate>,
}

#[cfg(feature = "sdk")]
mod sdk_conversions {
    use super::*;
    use concordium_rust_sdk::{base::updates, common::Serial, id};

    impl<P: Serial> From<(GovernanceUpdatePrefix, P)> for FixedUpdateRequest {
        fn from((prefix, payload): (GovernanceUpdatePrefix, P)) -> Self {
            let mut payload_bytes = Vec::new();
            payload.serial(&mut payload_bytes);
            Self {
                prefix,
                payload: payload_bytes,
            }
        }
    }

    impl From<(GovernanceUpdatePrefix, updates::ProtocolUpdate)> for ProtocolUpdateRequest {
        fn from((prefix, payload): (GovernanceUpdatePrefix, updates::ProtocolUpdate)) -> Self {
            let mut serialized = Vec::new();
            payload.serial(&mut serialized);
            let payload_length = u64::from_be_bytes(
                serialized[..8]
                    .try_into()
                    .expect("protocol payload has length prefix"),
            );
            Self {
                prefix,
                payload_length,
                message: payload.message.into_bytes(),
                specification_url: payload.specification_url.into_bytes(),
                specification_hash: serialize_fixed_32(payload.specification_hash),
                auxiliary_data: payload.specification_auxiliary_data,
            }
        }
    }

    impl
        From<(
            GovernanceUpdatePrefix,
            id::types::ArInfo<id::constants::ArCurve>,
        )> for AddAnonymityRevokerRequest
    {
        fn from(
            (prefix, payload): (
                GovernanceUpdatePrefix,
                id::types::ArInfo<id::constants::ArCurve>,
            ),
        ) -> Self {
            let mut inner = Vec::new();
            payload.serial(&mut inner);
            let mut public_key = Vec::new();
            payload.ar_public_key.serial(&mut public_key);
            let ar_info_length = inner.len() as u32;
            Self {
                prefix,
                payload_length: u64::from(4 + ar_info_length),
                ar_info_length,
                ar_identity: payload.ar_identity.into(),
                description: description_from_sdk(payload.ar_description),
                public_key,
            }
        }
    }

    impl
        From<(
            GovernanceUpdatePrefix,
            id::types::IpInfo<id::constants::IpPairing>,
        )> for AddIdentityProviderRequest
    {
        fn from(
            (prefix, payload): (
                GovernanceUpdatePrefix,
                id::types::IpInfo<id::constants::IpPairing>,
            ),
        ) -> Self {
            let mut inner = Vec::new();
            payload.serial(&mut inner);
            let mut verify_key = Vec::new();
            payload.ip_verify_key.serial(&mut verify_key);
            let mut cdi_verify_key = Vec::new();
            payload.ip_cdi_verify_key.serial(&mut cdi_verify_key);
            let ip_info_length = inner.len() as u32;
            Self {
                prefix,
                payload_length: u64::from(4 + ip_info_length),
                ip_info_length,
                ip_identity: payload.ip_identity.0,
                description: description_from_sdk(payload.ip_description),
                verify_key,
                cdi_verify_key: cdi_verify_key
                    .try_into()
                    .expect("SDK CDI verify key serializes to 32 bytes"),
            }
        }
    }

    impl From<(GovernanceUpdatePrefix, updates::CreatePlt)> for CreatePltRequest {
        fn from((prefix, payload): (GovernanceUpdatePrefix, updates::CreatePlt)) -> Self {
            Self {
                prefix,
                token_id: payload.token_id.as_ref().as_bytes().to_vec(),
                token_module: serialize_fixed_32(payload.token_module),
                decimals: payload.decimals,
                initialization_parameters: payload.initialization_parameters.as_ref().to_vec(),
            }
        }
    }

    impl<Kind>
        From<(
            GovernanceUpdatePrefix,
            HigherLevelKeyUpdateType,
            updates::HigherLevelAccessStructure<Kind>,
        )> for HigherLevelKeyUpdateRequest
    {
        fn from(
            (prefix, key_update_type, payload): (
                GovernanceUpdatePrefix,
                HigherLevelKeyUpdateType,
                updates::HigherLevelAccessStructure<Kind>,
            ),
        ) -> Self {
            Self {
                prefix,
                key_update_type,
                keys: payload.keys.into_iter().map(Into::into).collect(),
                threshold: payload.threshold.into(),
            }
        }
    }

    impl
        From<(
            GovernanceUpdatePrefix,
            AuthorizationsKeyUpdateType,
            AuthorizationsVersion,
            updates::AuthorizationsV0,
        )> for AuthorizationsUpdateRequest
    {
        fn from(
            (prefix, key_update_type, version, payload): (
                GovernanceUpdatePrefix,
                AuthorizationsKeyUpdateType,
                AuthorizationsVersion,
                updates::AuthorizationsV0,
            ),
        ) -> Self {
            let access_structures = vec![
                payload.emergency.clone(),
                payload.protocol.clone(),
                payload.election_difficulty.clone(),
                payload.euro_per_energy.clone(),
                payload.micro_gtu_per_euro.clone(),
                payload.foundation_account.clone(),
                payload.mint_distribution.clone(),
                payload.transaction_fee_distribution.clone(),
                payload.param_gas_rewards.clone(),
                payload.pool_parameters.clone(),
                payload.add_anonymity_revoker.clone(),
                payload.add_identity_provider.clone(),
            ];
            AuthorizationsUpdateRequest {
                prefix,
                key_update_type,
                version,
                keys: payload.keys.into_iter().map(Into::into).collect(),
                access_structures: access_structures
                    .into_iter()
                    .map(access_structure_from_sdk)
                    .collect(),
            }
        }
    }

    impl
        From<(
            GovernanceUpdatePrefix,
            AuthorizationsKeyUpdateType,
            AuthorizationsVersion,
            updates::AuthorizationsV1,
        )> for AuthorizationsUpdateRequest
    {
        fn from(
            (prefix, key_update_type, version, payload): (
                GovernanceUpdatePrefix,
                AuthorizationsKeyUpdateType,
                AuthorizationsVersion,
                updates::AuthorizationsV1,
            ),
        ) -> Self {
            let mut access_structures = vec![
                payload.v0.emergency.clone(),
                payload.v0.protocol.clone(),
                payload.v0.election_difficulty.clone(),
                payload.v0.euro_per_energy.clone(),
                payload.v0.micro_gtu_per_euro.clone(),
                payload.v0.foundation_account.clone(),
                payload.v0.mint_distribution.clone(),
                payload.v0.transaction_fee_distribution.clone(),
                payload.v0.param_gas_rewards.clone(),
                payload.v0.pool_parameters.clone(),
                payload.v0.add_anonymity_revoker.clone(),
                payload.v0.add_identity_provider.clone(),
                payload.cooldown_parameters.clone(),
                payload.time_parameters.clone(),
            ];
            if let Some(create_plt) = payload.create_plt.clone() {
                access_structures.push(create_plt);
            }
            AuthorizationsUpdateRequest {
                prefix,
                key_update_type,
                version,
                keys: payload.v0.keys.into_iter().map(Into::into).collect(),
                access_structures: access_structures
                    .into_iter()
                    .map(access_structure_from_sdk)
                    .collect(),
            }
        }
    }

    fn access_structure_from_sdk(value: updates::AccessStructure) -> AccessStructureUpdate {
        AccessStructureUpdate {
            key_indices: value
                .authorized_keys
                .into_iter()
                .map(|index| index.index)
                .collect(),
            threshold: value.threshold.into(),
        }
    }

    fn description_from_sdk(value: id::types::Description) -> DescriptionFields {
        DescriptionFields {
            name: value.name.into_bytes(),
            url: value.url.into_bytes(),
            description: value.description.into_bytes(),
        }
    }

    fn serialize_fixed_32(value: impl Serial) -> [u8; 32] {
        let mut bytes = Vec::new();
        value.serial(&mut bytes);
        bytes.try_into().expect("SDK value serializes to 32 bytes")
    }
}

#[cfg(feature = "sdk")]
impl TryFrom<concordium_rust_sdk::base::updates::UpdateHeader> for UpdateHeaderBytes {
    type Error = GovernanceLedgerError;

    fn try_from(value: concordium_rust_sdk::base::updates::UpdateHeader) -> Result<Self> {
        use concordium_rust_sdk::common::Serial;

        let mut bytes = Vec::new();
        value.serial(&mut bytes);
        let actual_len = bytes.len();
        let header = bytes.try_into().map_err(|_| {
            GovernanceLedgerError::invalid_request(format!(
                "serialized SDK update header must be 28 bytes, got {actual_len}"
            ))
        })?;
        Ok(Self(header))
    }
}

#[cfg(feature = "sdk")]
impl From<concordium_rust_sdk::base::base::UpdatePublicKey> for GovernancePublicKeyEntry {
    fn from(value: concordium_rust_sdk::base::base::UpdatePublicKey) -> Self {
        let json = serde_json::to_value(value).unwrap_or_default();
        let key = json
            .get("verifyKey")
            .and_then(serde_json::Value::as_str)
            .and_then(|hex| hex::decode(hex).ok())
            .and_then(|bytes| bytes.try_into().ok())
            .unwrap_or([0; 32]);
        Self { scheme_id: 0, key }
    }
}
