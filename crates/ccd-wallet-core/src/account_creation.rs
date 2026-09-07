//! Construction of account credential deployments from explicit cryptographic inputs.

use crate::wallet::ConcordiumHdWallet;
use anyhow::{Context, Result, bail};
use concordium_rust_sdk::{
    common::types::{KeyIndex, TransactionTime},
    id::{
        account_holder::create_unsigned_credential,
        constants::{ArCurve, AttributeKind, IpPairing},
        pedersen_commitment::{Randomness as CommitmentRandomness, Value as PedersenValue},
        types::{
            AccCredentialInfo, AccountCredential, AccountCredentialMessage, AccountOwnershipProof,
            CredDeploymentProofs, CredentialData, CredentialDataWithSigning,
            CredentialDeploymentInfo, CredentialHolderInfo, CredentialPublicKeys, GlobalContext,
            HasAttributeRandomness, IdCredentials, IdObjectUseData, IdentityObjectV1, IpContext,
            IpInfo, Policy, PublicCredentialData, UnsignedCredentialDeploymentInfo,
        },
    },
    smart_contracts::common::SignatureThreshold,
};
use ed25519_dalek::SigningKey;
use either::Either;
use serde_json::Value;
use std::{collections::BTreeMap, error::Error as StdError, fmt};

/// Default number of minutes for which a credential deployment remains valid.
pub const DEFAULT_CREDENTIAL_MESSAGE_EXPIRY_MINUTES: u32 = 5;

/// Seed-backed inputs for constructing and signing an account credential deployment.
///
/// This adapter preserves the existing seed-derived account-creation flow. New credential sources
/// should use [`CredentialConstructionInput`] to supply their explicit cryptographic material.
pub struct CredentialDeploymentInput<'a> {
    /// Wallet from which the seed-backed material is derived.
    pub wallet: &'a ConcordiumHdWallet,
    /// Identity provider information for the credential.
    pub ip_info: &'a IpInfo<IpPairing>,
    /// Anonymity revoker information used to construct the proofs.
    pub ar_infos: &'a BTreeMap<
        concordium_rust_sdk::id::types::ArIdentity,
        concordium_rust_sdk::id::types::ArInfo<ArCurve>,
    >,
    /// Global cryptographic context for the target network.
    pub global_context: &'a GlobalContext<ArCurve>,
    /// Issued identity object from which the credential policy is derived.
    pub identity_object: IdentityObjectV1<IpPairing, ArCurve, AttributeKind>,
    /// Identity index under the seed wallet.
    pub identity_index: u32,
    /// Credential counter for the account being created.
    pub credential_counter: u8,
}

/// Explicit inputs used to construct an unsigned account credential deployment.
///
/// The caller provides identity credential data, account credential public keys, and attribute
/// commitment-randomness retrieval separately. This keeps proof construction independent of the
/// source of account signing keys.
pub struct CredentialConstructionInput<'a, R> {
    /// Identity provider information for the credential.
    pub ip_info: &'a IpInfo<IpPairing>,
    /// Anonymity revoker information used to construct the proofs.
    pub ar_infos: &'a BTreeMap<
        concordium_rust_sdk::id::types::ArIdentity,
        concordium_rust_sdk::id::types::ArInfo<ArCurve>,
    >,
    /// Global cryptographic context for the target network.
    pub global_context: &'a GlobalContext<ArCurve>,
    /// Issued identity object from which the credential policy is derived.
    pub identity_object: IdentityObjectV1<IpPairing, ArCurve, AttributeKind>,
    /// ID credential secret, PRF key, and signature-retrieval randomness.
    pub id_use_data: IdObjectUseData<IpPairing, ArCurve>,
    /// Public account keys and signature threshold embedded in the credential.
    pub credential_public_keys: CredentialPublicKeys,
    /// Source of attribute commitment randomness.
    pub attribute_randomness: &'a R,
    /// Credential counter for the account being created.
    pub credential_counter: u8,
}

/// An unsigned credential deployment ready for account-ownership signing.
pub struct UnsignedCredentialDeployment {
    /// Expiry included in the credential deployment message and signing payload.
    pub message_expiry: TransactionTime,
    /// Credential values and identity proofs awaiting account-ownership signatures.
    pub credential: UnsignedCredentialDeploymentInfo<IpPairing, ArCurve, AttributeKind>,
}

/// Extract the unversioned identity object from a provider token or identity object.
///
/// # Arguments
///
/// * `token_or_identity_object` - Provider response or already extracted identity object.
///
/// # Returns
///
/// The identity object suitable for [`parse_identity_object`].
pub fn extract_identity_object(token_or_identity_object: &Value) -> Result<Value> {
    let identity_object = token_or_identity_object
        .get("identityObject")
        .unwrap_or(token_or_identity_object);

    if let Some(versioned_value) = identity_object.get("value")
        && identity_object.get("v").is_some()
    {
        return Ok(versioned_value.clone());
    }

    Ok(identity_object.clone())
}

/// Parse an issued identity object from a provider token or identity object.
///
/// # Arguments
///
/// * `token_or_identity_object` - Provider response or already extracted identity object.
///
/// # Errors
///
/// Returns an error if the extracted value is not a valid issued identity object.
pub fn parse_identity_object(
    token_or_identity_object: &Value,
) -> Result<IdentityObjectV1<IpPairing, ArCurve, AttributeKind>> {
    serde_json::from_value(extract_identity_object(token_or_identity_object)?)
        .context("failed to parse issued identity object")
}

/// Build and sign a credential deployment from seed-derived material.
///
/// # Arguments
///
/// * `input` - Seed wallet and account-creation context.
///
/// # Errors
///
/// Returns an error if seed derivation or credential proof construction fails.
pub fn build_credential_deployment(
    input: CredentialDeploymentInput<'_>,
) -> Result<AccountCredentialMessage<IpPairing, ArCurve, AttributeKind>> {
    let CredentialDeploymentInput {
        wallet,
        ip_info,
        ar_infos,
        global_context,
        identity_object,
        identity_index,
        credential_counter,
    } = input;
    let ip_identity = ip_info.ip_identity.0;
    let id_use_data = id_use_data(wallet, ip_identity, identity_index)?;
    let credential_data = credential_data(wallet, ip_identity, identity_index, credential_counter)?;
    let attribute_randomness = DerivedAttributeRandomness {
        wallet,
        ip_identity,
        identity_index,
        credential_counter,
    };
    let unsigned = build_unsigned_credential_deployment(CredentialConstructionInput {
        ip_info,
        ar_infos,
        global_context,
        identity_object,
        id_use_data,
        credential_public_keys: credential_data.get_cred_key_info(),
        attribute_randomness: &attribute_randomness,
        credential_counter,
    })?;
    let signatures =
        credential_data.sign(&Either::Left(unsigned.message_expiry), &unsigned.credential);

    Ok(AccountCredentialMessage {
        message_expiry: unsigned.message_expiry,
        credential: AccountCredential::Normal {
            cdi: CredentialDeploymentInfo {
                values: unsigned.credential.values,
                proofs: CredDeploymentProofs {
                    id_proofs: unsigned.credential.proofs,
                    proof_acc_sk: AccountOwnershipProof { sigs: signatures },
                },
            },
        },
    })
}

/// Build an unsigned credential deployment from explicit cryptographic inputs.
///
/// # Arguments
///
/// * `input` - Identity credential data, account public keys, randomness source, and credential
///   context.
///
/// # Returns
///
/// The credential values and identity proofs together with the expiry that must be used for the
/// account-ownership signature.
///
/// # Errors
///
/// Returns an error if attribute-randomness retrieval or credential proof construction fails.
pub fn build_unsigned_credential_deployment<R>(
    input: CredentialConstructionInput<'_, R>,
) -> Result<UnsignedCredentialDeployment>
where
    R: HasAttributeRandomness<ArCurve>,
{
    let CredentialConstructionInput {
        ip_info,
        ar_infos,
        global_context,
        identity_object,
        id_use_data,
        credential_public_keys,
        attribute_randomness,
        credential_counter,
    } = input;
    let policy = Policy {
        valid_to: identity_object.alist.valid_to,
        created_at: identity_object.alist.created_at,
        policy_vec: BTreeMap::new(),
        _phantom: Default::default(),
    };
    let message_expiry = TransactionTime::minutes_after(DEFAULT_CREDENTIAL_MESSAGE_EXPIRY_MINUTES);
    let (credential, _) = create_unsigned_credential(
        IpContext {
            ip_info,
            ars_infos: ar_infos,
            global_context,
        },
        &identity_object,
        &id_use_data,
        credential_counter,
        policy,
        credential_public_keys,
        None,
        attribute_randomness,
    )
    .context("failed to build unsigned account credential deployment")?;

    Ok(UnsignedCredentialDeployment {
        message_expiry,
        credential,
    })
}

fn id_use_data(
    wallet: &ConcordiumHdWallet,
    ip_identity: u32,
    identity_index: u32,
) -> Result<IdObjectUseData<IpPairing, ArCurve>> {
    Ok(IdObjectUseData {
        aci: AccCredentialInfo {
            cred_holder_info: CredentialHolderInfo {
                id_cred: IdCredentials {
                    id_cred_sec: PedersenValue::new(
                        wallet.get_id_cred_sec(ip_identity, identity_index)?,
                    ),
                },
            },
            prf_key: wallet.get_prf_key(ip_identity, identity_index)?,
        },
        randomness: wallet.get_blinding_randomness(ip_identity, identity_index)?,
    })
}

fn credential_data(
    wallet: &ConcordiumHdWallet,
    ip_identity: u32,
    identity_index: u32,
    credential_counter: u8,
) -> Result<CredentialData> {
    let secret =
        wallet.get_account_signing_key(ip_identity, identity_index, credential_counter.into())?;
    let mut keys = BTreeMap::new();
    keys.insert(KeyIndex(0), SigningKey::from_bytes(&secret).into());

    Ok(CredentialData {
        keys,
        threshold: SignatureThreshold::ONE,
    })
}

struct DerivedAttributeRandomness<'a> {
    wallet: &'a ConcordiumHdWallet,
    ip_identity: u32,
    identity_index: u32,
    credential_counter: u8,
}

impl HasAttributeRandomness<ArCurve> for DerivedAttributeRandomness<'_> {
    type ErrorType = AttributeRandomnessError;

    fn get_attribute_commitment_randomness(
        &self,
        attribute_tag: &concordium_rust_sdk::id::types::AttributeTag,
    ) -> std::result::Result<CommitmentRandomness<ArCurve>, Self::ErrorType> {
        self.wallet
            .get_attribute_commitment_randomness(
                self.ip_identity,
                self.identity_index,
                self.credential_counter.into(),
                *attribute_tag,
            )
            .map_err(|err| AttributeRandomnessError(err.to_string()))
    }
}

#[derive(Debug)]
struct AttributeRandomnessError(String);

impl fmt::Display for AttributeRandomnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl StdError for AttributeRandomnessError {}

/// Convert a stored credential counter to the protocol's one-byte representation.
///
/// # Arguments
///
/// * `counter` - Stored credential counter.
///
/// # Errors
///
/// Returns an error if `counter` exceeds the protocol's `u8` range.
pub fn credential_counter_to_u8(counter: u32) -> Result<u8> {
    counter
        .try_into()
        .with_context(|| format!("credential counter {counter} exceeds Concordium u8 range"))
}

/// Ensure identity expiry metadata is present and later than the supplied timestamp.
///
/// # Arguments
///
/// * `expires_at` - Optional Unix expiry timestamp from identity metadata.
/// * `now` - Unix timestamp against which expiry is checked.
///
/// # Errors
///
/// Returns an error if expiry metadata is missing or the identity has expired.
pub fn ensure_not_expired(expires_at: Option<i64>, now: i64) -> Result<()> {
    let expires_at = expires_at.context("identity has no expiry metadata")?;
    if expires_at <= now {
        bail!("identity is expired");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_identity_object_from_provider_token() {
        let token = json!({
            "identityObject": {
                "preIdentityObject": {},
                "attributeList": {},
                "signature": "sig"
            }
        });

        assert_eq!(
            extract_identity_object(&token).unwrap(),
            json!({
                "preIdentityObject": {},
                "attributeList": {},
                "signature": "sig"
            })
        );
    }

    #[test]
    fn unwraps_versioned_identity_object_from_provider_token() {
        let token = json!({
            "identityObject": {
                "v": 0,
                "value": {
                    "preIdentityObject": {},
                    "attributeList": {},
                    "signature": "sig"
                }
            }
        });

        assert_eq!(
            extract_identity_object(&token).unwrap(),
            json!({
                "preIdentityObject": {},
                "attributeList": {},
                "signature": "sig"
            })
        );
    }

    #[test]
    fn treats_unwrapped_value_as_identity_object() {
        let identity_object = json!({ "value": "identity" });

        assert_eq!(
            extract_identity_object(&identity_object).unwrap(),
            identity_object
        );
    }

    #[test]
    fn rejects_credential_counter_outside_protocol_range() {
        assert_eq!(credential_counter_to_u8(255).unwrap(), 255);
        let err = credential_counter_to_u8(256).unwrap_err();
        assert!(err.to_string().contains("exceeds Concordium u8 range"));
    }

    #[test]
    fn validates_identity_expiry() {
        ensure_not_expired(Some(200), 100).unwrap();
        assert!(
            ensure_not_expired(Some(100), 100)
                .unwrap_err()
                .to_string()
                .contains("expired")
        );
        assert!(
            ensure_not_expired(None, 100)
                .unwrap_err()
                .to_string()
                .contains("no expiry")
        );
    }
}
