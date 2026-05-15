use crate::wallet::ConcordiumHdWallet;
use anyhow::{Context, Result, bail};
use concordium_rust_sdk::{
    common::types::{KeyIndex, TransactionTime},
    id::{
        account_holder::create_credential,
        constants::{ArCurve, AttributeKind, IpPairing},
        pedersen_commitment::{Randomness as CommitmentRandomness, Value as PedersenValue},
        types::{
            AccCredentialInfo, AccountCredential, AccountCredentialMessage, CredentialData,
            CredentialHolderInfo, GlobalContext, HasAttributeRandomness, IdCredentials,
            IdObjectUseData, IdentityObjectV1, IpContext, IpInfo, Policy,
        },
    },
    smart_contracts::common::SignatureThreshold,
};
use ed25519_dalek::SigningKey;
use either::Either;
use serde_json::Value;
use std::{collections::BTreeMap, error::Error as StdError, fmt};

pub const DEFAULT_CREDENTIAL_MESSAGE_EXPIRY_MINUTES: u32 = 5;

pub struct CredentialDeploymentInput<'a> {
    pub wallet: &'a ConcordiumHdWallet,
    pub ip_info: &'a IpInfo<IpPairing>,
    pub ar_infos: &'a BTreeMap<
        concordium_rust_sdk::id::types::ArIdentity,
        concordium_rust_sdk::id::types::ArInfo<ArCurve>,
    >,
    pub global_context: &'a GlobalContext<ArCurve>,
    pub identity_object: IdentityObjectV1<IpPairing, ArCurve, AttributeKind>,
    pub identity_index: u32,
    pub credential_counter: u8,
}

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

pub fn parse_identity_object(
    token_or_identity_object: &Value,
) -> Result<IdentityObjectV1<IpPairing, ArCurve, AttributeKind>> {
    serde_json::from_value(extract_identity_object(token_or_identity_object)?)
        .context("failed to parse issued identity object")
}

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
    let id_cred_sec = wallet.get_id_cred_sec(ip_identity, identity_index)?;
    let prf_key = wallet.get_prf_key(ip_identity, identity_index)?;
    let randomness = wallet.get_blinding_randomness(ip_identity, identity_index)?;

    let id_use_data = IdObjectUseData {
        aci: AccCredentialInfo {
            cred_holder_info: CredentialHolderInfo {
                id_cred: IdCredentials {
                    id_cred_sec: PedersenValue::new(id_cred_sec),
                },
            },
            prf_key,
        },
        randomness,
    };

    let policy = Policy {
        valid_to: identity_object.alist.valid_to,
        created_at: identity_object.alist.created_at,
        policy_vec: BTreeMap::new(),
        _phantom: Default::default(),
    };
    let credential_data = credential_data(wallet, ip_identity, identity_index, credential_counter)?;
    let attribute_randomness = DerivedAttributeRandomness {
        wallet,
        ip_identity,
        identity_index,
        credential_counter,
    };
    let context = IpContext {
        ip_info,
        ars_infos: ar_infos,
        global_context,
    };
    let message_expiry = TransactionTime::minutes_after(DEFAULT_CREDENTIAL_MESSAGE_EXPIRY_MINUTES);
    let (cdi, _) = create_credential(
        context,
        &identity_object,
        &id_use_data,
        credential_counter,
        policy,
        &credential_data,
        &attribute_randomness,
        &Either::Left(message_expiry),
    )
    .context("failed to build account credential deployment")?;

    Ok(AccountCredentialMessage {
        message_expiry,
        credential: AccountCredential::Normal { cdi },
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

pub fn credential_counter_to_u8(counter: u32) -> Result<u8> {
    counter
        .try_into()
        .with_context(|| format!("credential counter {counter} exceeds Concordium u8 range"))
}

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
