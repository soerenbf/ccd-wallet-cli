//! Identity-provider request construction and HTTP support.
//!
//! This crate owns Concordium identity issuance request construction together
//! with provider metadata, polling, and callback helpers used by the CLI.

pub mod callback;
pub mod client;

use anyhow::{Result, bail};
use ccd_wallet_core::wallet::{ConcordiumHdWallet, CredId, PrfKey};
use concordium_rust_sdk::base::{
    common::{VERSION_0, Versioned},
    id::{
        account_holder::{generate_id_recovery_request, generate_pio_v1},
        constants::{ArCurve, IpPairing},
        pedersen_commitment::Value as PedersenValue,
        ps_sig::SigRetrievalRandomness,
        secret_sharing::Threshold,
        types::{
            AccCredentialInfo, ArIdentity, ArInfo, CredentialHolderInfo, GlobalContext,
            IdCredentials, IdObjectUseData, IdRecoveryRequest, IpContext, IpInfo,
            PreIdentityObjectV1,
        },
    },
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
struct IdentityObjectRequestV1 {
    #[serde(rename = "idObjectRequest")]
    id_object_request: Versioned<PreIdentityObjectV1<IpPairing, ArCurve>>,
}

#[derive(Serialize)]
struct IdentityRecoveryRequestV1 {
    #[serde(rename = "idRecoveryRequest")]
    id_recovery_request: Versioned<IdRecoveryRequest<ArCurve>>,
}

/// Derived material required to construct an identity issuance request.
pub struct IdentityIssuanceMaterial {
    /// Identity credential secret used in the pre-identity object.
    pub id_cred_sec: CredId,
    /// PRF key used for identity and account credential derivations.
    pub prf_key: PrfKey,
    /// Blinding randomness used for identity provider signature retrieval.
    pub blinding_randomness: SigRetrievalRandomness<IpPairing>,
}

impl IdentityIssuanceMaterial {
    /// Derive identity issuance material from a seed-backed wallet.
    ///
    /// # Arguments
    ///
    /// * `wallet` - Seed-backed Concordium HD wallet.
    /// * `identity_provider_index` - Identity provider index.
    /// * `identity_index` - Identity index under the selected key source.
    ///
    /// # Errors
    ///
    /// Returns an error if any seed derivation fails.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let material = IdentityIssuanceMaterial::from_wallet(&wallet, 0, 0)?;
    /// ```
    pub fn from_wallet(
        wallet: &ConcordiumHdWallet,
        identity_provider_index: u32,
        identity_index: u32,
    ) -> Result<Self> {
        Ok(Self {
            id_cred_sec: wallet.get_id_cred_sec(identity_provider_index, identity_index)?,
            prf_key: wallet.get_prf_key(identity_provider_index, identity_index)?,
            blinding_randomness: wallet
                .get_blinding_randomness(identity_provider_index, identity_index)?,
        })
    }
}

/// Input for constructing an identity issuance request from prepared material.
pub struct IdentityRequestInput<'a> {
    /// Identity provider selected for issuance.
    pub ip_info: &'a IpInfo<IpPairing>,
    /// Anonymity revokers available on the selected network.
    pub ar_infos: &'a BTreeMap<ArIdentity, ArInfo<ArCurve>>,
    /// Chain cryptographic parameters.
    pub global_context: &'a GlobalContext<ArCurve>,
    /// Prepared identity issuance material.
    pub material: IdentityIssuanceMaterial,
}

/// Build a Concordium v1 identity issuance request from prepared material.
///
/// # Arguments
///
/// * `input` - Identity provider context, anonymity revokers, chain cryptographic
///   parameters, and derived issuance material.
///
/// # Errors
///
/// Returns an error if no anonymity revokers are supplied, if the revocation
/// threshold cannot be constructed, if pre-identity object generation fails, or
/// if JSON serialization fails.
///
/// # Examples
///
/// ```ignore
/// let request_json = build_request_from_material(IdentityRequestInput {
///     ip_info,
///     ar_infos,
///     global_context,
///     material,
/// })?;
/// ```
pub fn build_request_from_material(input: IdentityRequestInput<'_>) -> Result<String> {
    let IdentityRequestInput {
        ip_info,
        ar_infos,
        global_context,
        material,
    } = input;

    if ar_infos.is_empty() {
        bail!("identity request requires at least one anonymity revoker");
    }

    let threshold = Threshold::try_new(ar_infos.len() as u8)?;
    let context = IpContext {
        ip_info,
        ars_infos: ar_infos,
        global_context,
    };
    let id_use_data = IdObjectUseData {
        aci: AccCredentialInfo {
            cred_holder_info: CredentialHolderInfo {
                id_cred: IdCredentials {
                    id_cred_sec: PedersenValue::new(material.id_cred_sec),
                },
            },
            prf_key: material.prf_key,
        },
        randomness: material.blinding_randomness,
    };

    let (pio, _) = generate_pio_v1(&context, threshold, &id_use_data)
        .ok_or_else(|| anyhow::anyhow!("generating the pre-identity object failed"))?;

    Ok(serde_json::to_string(&IdentityObjectRequestV1 {
        id_object_request: Versioned::new(VERSION_0, pio),
    })?)
}

/// Build a Concordium v1 identity issuance request from a seed-backed wallet.
///
/// # Arguments
///
/// * `wallet` - Seed-backed Concordium HD wallet.
/// * `ip_info` - Identity provider selected for issuance.
/// * `ar_infos` - Anonymity revokers available on the selected network.
/// * `global_context` - Chain cryptographic parameters.
/// * `identity_index` - Identity index under the selected key source.
///
/// # Errors
///
/// Returns an error if seed derivation, request construction, or JSON
/// serialization fails.
///
/// # Examples
///
/// ```ignore
/// let request_json = build_request(&wallet, ip_info, &ar_infos, &global_context, 0)?;
/// ```
pub fn build_request(
    wallet: &ConcordiumHdWallet,
    ip_info: &IpInfo<IpPairing>,
    ar_infos: &BTreeMap<ArIdentity, ArInfo<ArCurve>>,
    global_context: &GlobalContext<ArCurve>,
    identity_index: u32,
) -> Result<String> {
    build_request_from_material(IdentityRequestInput {
        ip_info,
        ar_infos,
        global_context,
        material: IdentityIssuanceMaterial::from_wallet(
            wallet,
            ip_info.ip_identity.0,
            identity_index,
        )?,
    })
}

/// Build a Concordium v1 identity recovery request from a seed-backed wallet.
///
/// # Arguments
///
/// * `wallet` - Seed-backed Concordium HD wallet.
/// * `ip_info` - Identity provider selected for recovery.
/// * `global_context` - Chain cryptographic parameters.
/// * `identity_index` - Identity index under the selected key source.
/// * `timestamp` - Unix timestamp included in the recovery request.
///
/// # Errors
///
/// Returns an error if seed derivation, recovery request construction, or JSON
/// serialization fails.
///
/// # Examples
///
/// ```ignore
/// let request_json = build_recovery_request(&wallet, ip_info, &global_context, 0, timestamp)?;
/// ```
pub fn build_recovery_request(
    wallet: &ConcordiumHdWallet,
    ip_info: &IpInfo<IpPairing>,
    global_context: &GlobalContext<ArCurve>,
    identity_index: u32,
    timestamp: u64,
) -> Result<String> {
    let id_cred_sec =
        PedersenValue::new(wallet.get_id_cred_sec(ip_info.ip_identity.0, identity_index)?);
    let request = generate_id_recovery_request(ip_info, global_context, &id_cred_sec, timestamp)
        .ok_or_else(|| anyhow::anyhow!("generating the identity recovery request failed"))?;

    Ok(serde_json::to_string(&IdentityRecoveryRequestV1 {
        id_recovery_request: Versioned::new(VERSION_0, request),
    })?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccd_wallet_core::wallet::Net;

    const SEED_PHRASE: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn derives_material_from_wallet() {
        let wallet = ConcordiumHdWallet::from_seed_phrase(SEED_PHRASE, Net::Testnet).unwrap();
        let material = IdentityIssuanceMaterial::from_wallet(&wallet, 0, 0).unwrap();

        assert_eq!(material.id_cred_sec, wallet.get_id_cred_sec(0, 0).unwrap());
    }
}
