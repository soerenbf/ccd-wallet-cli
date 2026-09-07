//! Ledger identity/account construction bridge.
//!
//! This module is the explicit boundary between wallet orchestration and the
//! low-level Concordium Ledger APDU client. It prevents Ledger-backed flows from
//! silently falling back to seed-derived construction or private-key export when
//! a high-level identity/account flow is not yet representable end-to-end.

use anyhow::{Context, Result, bail};
use ccd_wallet_core::{
    account_creation::{
        CredentialConstructionInput, UnsignedCredentialDeployment,
        build_unsigned_credential_deployment,
    },
    store::signer_owners::LedgerOwnerDetailsRecord,
    wallet::{CredId, ExtendedSlip10Node, PrfKey},
};
use ccd_wallet_identity_provider::IdentityIssuanceMaterial;
use ccd_wallet_ledger::{
    ConcordiumLedgerApp, CredentialAttribute, CredentialDeploymentContext,
    CredentialDeploymentSigningRequest, CredentialSigningPayload, DerivationPath,
    ExportPrivateKeyNetwork, ExportPrivateKeyNewRequest, ExportPrivateKeyNewType, LedgerError,
    LedgerTransport, PublicKeyOptions,
};
use concordium_rust_sdk::{
    base::{
        curve_arithmetic::Curve,
        id::{
            constants::{ArCurve, AttributeKind, IpPairing},
            pedersen_commitment::{Randomness as CommitmentRandomness, Value as PedersenValue},
            ps_sig::SigRetrievalRandomness,
            types::{
                AccCredentialInfo, AccountCredential, AccountCredentialMessage,
                AccountOwnershipProof, CredentialDeploymentInfo, CredentialHolderInfo,
                CredentialPublicKeys, GlobalContext, HasAttributeRandomness, IdCredentials,
                IdObjectUseData, IdentityObjectV1, IpInfo,
            },
        },
    },
    common::Serial,
};
use ed25519_dalek::{Signature, VerifyingKey};
use std::{collections::BTreeMap, error::Error as StdError, fmt};
use zeroize::Zeroizing;

const EXPORTED_KEY_LENGTH: usize = 32;
const PURPOSE_IDENTITY_EXPORT_KEY_COUNT: usize = 3;
const PURPOSE_IDENTITY_EXPORT_RESPONSE_LENGTH: usize =
    PURPOSE_IDENTITY_EXPORT_KEY_COUNT * (1 + EXPORTED_KEY_LENGTH);
const PURPOSE_ID_RECOVERY_EXPORT_KEY_COUNT: usize = 2;
const PURPOSE_ID_RECOVERY_EXPORT_RESPONSE_LENGTH: usize =
    PURPOSE_ID_RECOVERY_EXPORT_KEY_COUNT * (1 + EXPORTED_KEY_LENGTH);
const PURPOSE_ACCOUNT_DISCOVERY_EXPORT_KEY_COUNT: usize = 1;
const PURPOSE_ACCOUNT_DISCOVERY_EXPORT_RESPONSE_LENGTH: usize =
    PURPOSE_ACCOUNT_DISCOVERY_EXPORT_KEY_COUNT * (1 + EXPORTED_KEY_LENGTH);
const EXPORTED_EXTENDED_NODE_LENGTH: usize = 64;

struct AccountCreationExportMaterial {
    prf_key: Zeroizing<[u8; EXPORTED_KEY_LENGTH]>,
    id_cred_sec: Zeroizing<[u8; EXPORTED_KEY_LENGTH]>,
    commitment_randomness_root: ExtendedSlip10Node,
}

/// Inputs that identify a Ledger-backed identity issuance construction request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LedgerIdentityIssuanceInput<'a> {
    /// Enrolled Ledger owner details matched against the connected device.
    pub owner_details: &'a LedgerOwnerDetailsRecord,
    /// Network genesis hash for the identity issuance flow.
    pub network_genesis_hash: &'a str,
    /// Network designation for purpose-based Ledger export derivation.
    pub export_network: ExportPrivateKeyNetwork,
    /// Identity provider index.
    pub ip_identity: u32,
    /// Identity index under the Ledger key source.
    pub identity_index: u32,
    /// Whether the caller has completed the explicit secret-export approval flow.
    pub approved_secret_export: bool,
}

/// Private Ledger bridge input including caller-fetched chain context.
pub(crate) struct LedgerAccountCredentialDeploymentInput<'a> {
    /// Network designation for purpose-based Ledger export derivation.
    pub export_network: ExportPrivateKeyNetwork,
    /// Ledger coin type used in the account signing path.
    pub coin_type: u32,
    /// Identity provider information fetched by the caller.
    pub ip_info: &'a IpInfo<IpPairing>,
    /// Anonymity revoker information fetched by the caller.
    pub ar_infos: &'a BTreeMap<
        concordium_rust_sdk::id::types::ArIdentity,
        concordium_rust_sdk::id::types::ArInfo<ArCurve>,
    >,
    /// Global cryptographic context fetched by the caller.
    pub global_context: &'a GlobalContext<ArCurve>,
    /// Issued identity object fetched by the caller.
    pub identity_object: IdentityObjectV1<IpPairing, ArCurve, AttributeKind>,
    /// Identity provider index.
    pub ip_identity: u32,
    /// Identity index under the Ledger key source.
    pub identity_index: u32,
    /// Credential counter for the derived account.
    pub credential_counter: u8,
    /// Whether the caller has completed the explicit secret-export approval flow.
    pub approved_secret_export: bool,
}

/// Prepare Ledger-derived identity issuance material.
///
/// # Arguments
/// * `input` - Ledger owner, network, and identity issuance coordinates.
/// * `app` - Connected Concordium Ledger app client.
///
/// # Errors
/// Returns an error if export approval is missing, APDU exchange fails, the connected app does not
/// support the 5.5.0+ purpose-based identity credential creation export, or the export response is
/// malformed.
///
/// # Examples
///
/// ```ignore
/// let material = ledger_construction::construct_identity_issuance(input, &mut app)?;
/// ```
pub(crate) fn construct_identity_issuance<T: LedgerTransport>(
    input: LedgerIdentityIssuanceInput<'_>,
    app: &mut ConcordiumLedgerApp<T>,
) -> Result<IdentityIssuanceMaterial> {
    if !input.approved_secret_export {
        bail!(
            "Ledger identity issuance for key source '{}' requires explicit secret-export approval; no local identity state was written",
            input.owner_details.fingerprint,
        );
    }

    let exported = export_identity_credential_creation_material(app, &input)?;
    parse_identity_credential_creation_export(&exported)
}

/// Prepare Ledger-derived identity recovery material.
///
/// # Arguments
/// * `input` - Ledger owner, network, and identity recovery coordinates.
/// * `app` - Connected Concordium Ledger app client.
///
/// # Errors
/// Returns an error if export approval is missing, APDU exchange fails, the connected app does not
/// support the 5.5.0+ purpose-based identity recovery export, or the export response is malformed.
///
/// # Examples
///
/// ```ignore
/// let id_cred_sec = ledger_construction::construct_identity_recovery(input, &mut app)?;
/// ```
pub(crate) fn construct_identity_recovery<T: LedgerTransport>(
    input: LedgerIdentityIssuanceInput<'_>,
    app: &mut ConcordiumLedgerApp<T>,
) -> Result<CredId> {
    if !input.approved_secret_export {
        bail!(
            "Ledger identity recovery for key source '{}' requires explicit secret-export approval; no local identity state was written",
            input.owner_details.fingerprint,
        );
    }

    let exported = export_purpose_material(app, &input, ExportPrivateKeyNewType::IdRecovery)
        .map_err(|err| map_identity_export_error(err, &input))?;
    parse_identity_recovery_export(&exported)
}

/// Prepare Ledger-derived account credential discovery material.
///
/// # Arguments
/// * `input` - Ledger owner, network, and identity coordinates for account discovery.
/// * `app` - Connected Concordium Ledger app client.
///
/// # Errors
/// Returns an error if export approval is missing, APDU exchange fails, the connected app does not
/// support the 5.5.0+ purpose-based account-credential discovery export, or the export response is malformed.
///
/// # Examples
///
/// ```ignore
/// let prf_key = ledger_construction::construct_account_credential_discovery(input, &mut app)?;
/// ```
pub(crate) fn construct_account_credential_discovery<T: LedgerTransport>(
    input: LedgerIdentityIssuanceInput<'_>,
    app: &mut ConcordiumLedgerApp<T>,
) -> Result<PrfKey> {
    if !input.approved_secret_export {
        bail!(
            "Ledger account credential discovery for key source '{}' requires explicit secret-export approval; no local account state was written",
            input.owner_details.fingerprint,
        );
    }

    let exported = export_purpose_material(
        app,
        &input,
        ExportPrivateKeyNewType::AccountCredentialDiscovery,
    )
    .map_err(|err| map_identity_export_error(err, &input))?;
    parse_account_credential_discovery_export(&exported)
}

/// Construct and device-sign a Ledger-backed account credential deployment.
pub(crate) fn construct_credential_deployment_with_app<T: LedgerTransport>(
    input: LedgerAccountCredentialDeploymentInput<'_>,
    app: &mut ConcordiumLedgerApp<T>,
) -> Result<AccountCredentialMessage<IpPairing, ArCurve, AttributeKind>> {
    if !input.approved_secret_export {
        bail!("Ledger account creation requires explicit secret-export approval");
    }

    let account_path = DerivationPath::concordium_account(
        input.coin_type,
        input.ip_identity,
        input.identity_index,
        input.credential_counter.into(),
    )?;
    let account_public_key = app
        .get_public_key(account_path.clone(), PublicKeyOptions::default())?
        .public_key;
    let account_export = export_account_creation_material(app, &input)?;
    let recovery_export = export_identity_recovery_material(app, &input)?;
    if account_export.id_cred_sec[..] != recovery_export.id_cred_sec[..] {
        bail!(
            "Ledger account-creation and identity-recovery exports returned different IDCredSec values"
        );
    }
    let verifying_key = VerifyingKey::from_bytes(&account_public_key)
        .context("Ledger returned an invalid Ed25519 account public key")?;
    let mut keys = BTreeMap::new();
    keys.insert(
        concordium_rust_sdk::common::types::KeyIndex(0),
        verifying_key.into(),
    );
    let credential_public_keys = CredentialPublicKeys {
        keys,
        threshold: concordium_rust_sdk::id::types::SignatureThreshold::ONE,
    };
    let id_use_data = IdObjectUseData {
        aci: AccCredentialInfo {
            cred_holder_info: CredentialHolderInfo {
                id_cred: IdCredentials {
                    id_cred_sec: PedersenValue::new(bls_scalar_from_export(
                        &account_export.id_cred_sec,
                    )),
                },
            },
            prf_key: PrfKey::new(bls_scalar_from_export(&account_export.prf_key)),
        },
        randomness: SigRetrievalRandomness::new(bls_scalar_from_export(
            &recovery_export.blinding_randomness,
        )),
    };
    let randomness = LedgerAttributeRandomness(&account_export.commitment_randomness_root);
    let unsigned = build_unsigned_credential_deployment(CredentialConstructionInput {
        ip_info: input.ip_info,
        ar_infos: input.ar_infos,
        global_context: input.global_context,
        identity_object: input.identity_object,
        id_use_data,
        credential_public_keys,
        attribute_randomness: &randomness,
        credential_counter: input.credential_counter,
    })?;
    let request = credential_signing_request(&unsigned, account_path);
    let signature = app.sign_credential_deployment(&request)?;
    finish_credential(unsigned, signature.0)
}

struct IdentityRecoveryExportMaterial {
    id_cred_sec: Zeroizing<[u8; EXPORTED_KEY_LENGTH]>,
    blinding_randomness: Zeroizing<[u8; EXPORTED_KEY_LENGTH]>,
}

struct LedgerAttributeRandomness<'a>(&'a ExtendedSlip10Node);
impl HasAttributeRandomness<ArCurve> for LedgerAttributeRandomness<'_> {
    type ErrorType = AttributeRandomnessError;
    fn get_attribute_commitment_randomness(
        &self,
        tag: &concordium_rust_sdk::id::types::AttributeTag,
    ) -> std::result::Result<CommitmentRandomness<ArCurve>, Self::ErrorType> {
        self.0
            .derive_attribute_commitment_randomness(*tag)
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

fn export_account_creation_material<T: LedgerTransport>(
    app: &mut ConcordiumLedgerApp<T>,
    input: &LedgerAccountCredentialDeploymentInput<'_>,
) -> Result<AccountCreationExportMaterial> {
    let exported = app
        .export_private_key_new(&ExportPrivateKeyNewRequest {
            export_type: ExportPrivateKeyNewType::AccountCreation,
            network: input.export_network,
            payload: [
                input.ip_identity.to_be_bytes(),
                input.identity_index.to_be_bytes(),
                u32::from(input.credential_counter).to_be_bytes(),
            ]
            .concat(),
        })
        .map_err(map_account_export_error)?;
    parse_account_creation_export(&exported)
}

fn export_identity_recovery_material<T: LedgerTransport>(
    app: &mut ConcordiumLedgerApp<T>,
    input: &LedgerAccountCredentialDeploymentInput<'_>,
) -> Result<IdentityRecoveryExportMaterial> {
    let exported = app
        .export_private_key_new(&ExportPrivateKeyNewRequest {
            export_type: ExportPrivateKeyNewType::IdRecovery,
            network: input.export_network,
            payload: build_identity_issuance_export_payload(
                input.ip_identity,
                input.identity_index,
            ),
        })
        .map_err(map_account_export_error)?;
    parse_identity_recovery_export_material(&exported)
}

fn map_account_export_error(err: LedgerError) -> anyhow::Error {
    match err {
        LedgerError::Status {
            status: 0x6B03 | 0x6D00,
            ..
        } => {
            anyhow::anyhow!("Ledger account creation requires Concordium Ledger app 5.6.4 or newer")
        }
        other => anyhow::anyhow!(other),
    }
}

fn export_identity_credential_creation_material<T: LedgerTransport>(
    app: &mut ConcordiumLedgerApp<T>,
    input: &LedgerIdentityIssuanceInput<'_>,
) -> Result<Zeroizing<Vec<u8>>> {
    export_purpose_material(
        app,
        input,
        ExportPrivateKeyNewType::IdentityCredentialCreation,
    )
    .map_err(|err| map_identity_export_error(err, input))
}

fn export_purpose_material<T: LedgerTransport>(
    app: &mut ConcordiumLedgerApp<T>,
    input: &LedgerIdentityIssuanceInput<'_>,
    export_type: ExportPrivateKeyNewType,
) -> std::result::Result<Zeroizing<Vec<u8>>, LedgerError> {
    app.export_private_key_new(&ExportPrivateKeyNewRequest {
        export_type,
        network: input.export_network,
        payload: build_identity_issuance_export_payload(input.ip_identity, input.identity_index),
    })
}

fn map_identity_export_error(
    err: LedgerError,
    input: &LedgerIdentityIssuanceInput<'_>,
) -> anyhow::Error {
    match err {
        LedgerError::Status {
            status: 0x6B03 | 0x6D00,
            ..
        } => anyhow::anyhow!(
            "Ledger identity issuance requires Concordium Ledger app 5.5.0 or newer with purpose-based identity export support; update the Concordium Ledger app and retry"
        ),
        other => anyhow::anyhow!(other).context(format!(
            "failed to export Ledger identity issuance material for key source '{}' on network '{}' (provider {}, identity index {})",
            input.owner_details.fingerprint,
            input.network_genesis_hash,
            input.ip_identity,
            input.identity_index,
        )),
    }
}

fn build_identity_issuance_export_payload(ip_identity: u32, identity_index: u32) -> Vec<u8> {
    let mut payload = Vec::with_capacity(8);
    payload.extend_from_slice(&ip_identity.to_be_bytes());
    payload.extend_from_slice(&identity_index.to_be_bytes());
    payload
}

fn parse_identity_credential_creation_export(bytes: &[u8]) -> Result<IdentityIssuanceMaterial> {
    if bytes.len() != PURPOSE_IDENTITY_EXPORT_RESPONSE_LENGTH {
        bail!(
            "Ledger identity export response was {} bytes; expected {} bytes from Concordium Ledger app 5.5.0+ purpose-based identity export",
            bytes.len(),
            PURPOSE_IDENTITY_EXPORT_RESPONSE_LENGTH,
        );
    }

    let mut chunks = PurposeExportChunks::new(bytes);
    let id_cred_sec = chunks.next_key("IDCredSec")?;
    let prf_key = chunks.next_key("PRFKey")?;
    let blinding_randomness = chunks.next_key("signature blinding randomness")?;
    let id_cred_sec = bls_scalar_from_export(&id_cred_sec);
    let prf_key = PrfKey::new(bls_scalar_from_export(&prf_key));
    let blinding_randomness =
        SigRetrievalRandomness::new(bls_scalar_from_export(&blinding_randomness));
    chunks.finish_with_expected_count(PURPOSE_IDENTITY_EXPORT_KEY_COUNT)?;

    Ok(IdentityIssuanceMaterial {
        id_cred_sec,
        prf_key,
        blinding_randomness,
    })
}

fn parse_identity_recovery_export(bytes: &[u8]) -> Result<CredId> {
    let material = parse_identity_recovery_export_material(bytes)?;
    Ok(bls_scalar_from_export(&material.id_cred_sec))
}

fn parse_identity_recovery_export_material(bytes: &[u8]) -> Result<IdentityRecoveryExportMaterial> {
    if bytes.len() != PURPOSE_ID_RECOVERY_EXPORT_RESPONSE_LENGTH {
        bail!(
            "Ledger identity recovery export response was {} bytes; expected {} bytes from Concordium Ledger app 5.6.4+ purpose-based identity recovery export",
            bytes.len(),
            PURPOSE_ID_RECOVERY_EXPORT_RESPONSE_LENGTH
        );
    }
    let mut chunks = PurposeExportChunks::new(bytes);
    let id_cred_sec = chunks.next_key("IDCredSec")?;
    let blinding_randomness = chunks.next_key("signature blinding randomness")?;
    chunks.finish_with_expected_count(PURPOSE_ID_RECOVERY_EXPORT_KEY_COUNT)?;
    Ok(IdentityRecoveryExportMaterial {
        id_cred_sec,
        blinding_randomness,
    })
}

fn parse_account_credential_discovery_export(bytes: &[u8]) -> Result<PrfKey> {
    if bytes.len() != PURPOSE_ACCOUNT_DISCOVERY_EXPORT_RESPONSE_LENGTH {
        bail!(
            "Ledger account credential discovery export response was {} bytes; expected {} bytes from Concordium Ledger app 5.5.0+ purpose-based account credential discovery export",
            bytes.len(),
            PURPOSE_ACCOUNT_DISCOVERY_EXPORT_RESPONSE_LENGTH,
        );
    }

    let mut chunks = PurposeExportChunks::new(bytes);
    let prf_key = chunks.next_key("PRFKey")?;
    let prf_key = PrfKey::new(bls_scalar_from_export(&prf_key));
    chunks.finish_with_expected_count(PURPOSE_ACCOUNT_DISCOVERY_EXPORT_KEY_COUNT)?;
    Ok(prf_key)
}

fn parse_account_creation_export(bytes: &[u8]) -> Result<AccountCreationExportMaterial> {
    let mut chunks = PurposeExportChunks::new(bytes);
    let prf_key = chunks.next_key("PRFKey")?;
    let id_cred_sec = chunks.next_key("IDCredSec")?;
    let commitment_randomness_root =
        ExtendedSlip10Node::from_bytes(chunks.next_extended_node("commitment randomness root")?);
    chunks.finish_with_expected_count(3)?;

    Ok(AccountCreationExportMaterial {
        prf_key,
        id_cred_sec,
        commitment_randomness_root,
    })
}

fn credential_signing_request(
    unsigned: &UnsignedCredentialDeployment,
    path: DerivationPath,
) -> CredentialDeploymentSigningRequest {
    let values = &unsigned.credential.values;
    let verification_keys = values
        .cred_key_info
        .keys
        .iter()
        .map(|(index, key)| {
            let mut entry = Vec::new();
            index.serial(&mut entry);
            key.serial(&mut entry);
            entry
        })
        .collect();
    let mut threshold_credential_id_identity = serialize(&values.cred_key_info.threshold);
    values.cred_id.serial(&mut threshold_credential_id_identity);
    values
        .ip_identity
        .serial(&mut threshold_credential_id_identity);
    values
        .threshold
        .serial(&mut threshold_credential_id_identity);
    threshold_credential_id_identity
        .extend_from_slice(&(values.ar_data.len() as u16).to_be_bytes());
    let ar_identity_data = values
        .ar_data
        .iter()
        .map(|(identity, data)| {
            let mut entry = Vec::new();
            identity.serial(&mut entry);
            data.serial(&mut entry);
            entry
        })
        .collect();
    let mut credential_dates = serialize(&values.policy.valid_to);
    values.policy.created_at.serial(&mut credential_dates);
    credential_dates.extend_from_slice(&(values.policy.policy_vec.len() as u16).to_be_bytes());
    let attributes = values
        .policy
        .policy_vec
        .iter()
        .map(|(tag, value)| CredentialAttribute {
            tag: serialize(tag),
            value: value.as_ref().as_bytes().to_vec(),
        })
        .collect();
    let proofs = serialize(&unsigned.credential.proofs);
    CredentialDeploymentSigningRequest {
        path: path.to_ledger_bytes(),
        credential: CredentialSigningPayload {
            verification_key_count: vec![values.cred_key_info.keys.len() as u8],
            verification_keys,
            threshold_credential_id_identity,
            ar_identity_data,
            credential_dates,
            attributes,
            proof_length: (proofs.len() as u32).to_be_bytes().to_vec(),
            proofs,
        },
        context: CredentialDeploymentContext::New {
            expiry: serialize(&unsigned.message_expiry),
        },
    }
}

fn finish_credential(
    unsigned: UnsignedCredentialDeployment,
    signature: [u8; 64],
) -> Result<AccountCredentialMessage<IpPairing, ArCurve, AttributeKind>> {
    let signature = Signature::from_bytes(&signature);
    let mut signatures = BTreeMap::new();
    signatures.insert(
        concordium_rust_sdk::common::types::KeyIndex(0),
        signature.into(),
    );
    Ok(AccountCredentialMessage {
        message_expiry: unsigned.message_expiry,
        credential: AccountCredential::Normal {
            cdi: CredentialDeploymentInfo {
                values: unsigned.credential.values,
                proofs: concordium_rust_sdk::id::types::CredDeploymentProofs {
                    id_proofs: unsigned.credential.proofs,
                    proof_acc_sk: AccountOwnershipProof { sigs: signatures },
                },
            },
        },
    })
}

fn serialize<T: Serial>(value: &T) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.serial(&mut bytes);
    bytes
}

fn bls_scalar_from_export(bytes: &[u8; EXPORTED_KEY_LENGTH]) -> CredId {
    ArCurve::scalar_from_bytes(*bytes)
}

struct PurposeExportChunks<'a> {
    bytes: &'a [u8],
    offset: usize,
    keys_read: usize,
}

impl<'a> PurposeExportChunks<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            offset: 0,
            keys_read: 0,
        }
    }

    fn next_key(&mut self, label: &str) -> Result<Zeroizing<[u8; EXPORTED_KEY_LENGTH]>> {
        self.next_bytes(label)
    }

    fn next_extended_node(
        &mut self,
        label: &str,
    ) -> Result<Zeroizing<[u8; EXPORTED_EXTENDED_NODE_LENGTH]>> {
        self.next_bytes(label)
    }

    fn next_bytes<const N: usize>(&mut self, label: &str) -> Result<Zeroizing<[u8; N]>> {
        if self.offset >= self.bytes.len() {
            bail!("Ledger export response is missing {label}");
        }
        let length = self.bytes[self.offset] as usize;
        self.offset += 1;
        if length != N {
            bail!(
                "Ledger export response has {length}-byte {label}; expected {N} bytes (total response length: {} bytes)",
                self.bytes.len(),
            );
        }
        let end = self.offset + N;
        if end > self.bytes.len() {
            bail!("Ledger export response truncated while reading {label}");
        }
        let mut value = Zeroizing::new([0u8; N]);
        value.copy_from_slice(&self.bytes[self.offset..end]);
        self.offset = end;
        self.keys_read += 1;
        Ok(value)
    }

    fn finish_with_expected_count(self, expected: usize) -> Result<()> {
        if self.keys_read != expected {
            bail!(
                "Ledger export response contained {} keys; expected {}",
                self.keys_read,
                expected,
            );
        }
        if self.offset != self.bytes.len() {
            bail!(
                "Ledger export response has {} unexpected trailing bytes",
                self.bytes.len() - self.offset,
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccd_wallet_ledger::{MockTransport, apdu::Instruction};
    use concordium_rust_sdk::common::Versioned;
    use ed25519_dalek::SigningKey;

    fn details() -> LedgerOwnerDetailsRecord {
        LedgerOwnerDetailsRecord {
            signer_owner_id: "owner".to_owned(),
            canonical_public_key: vec![7; 32],
            fingerprint: "07070707".to_owned(),
            enrollment_path: "m/44'/919'/0'/0'/0'".to_owned(),
            app_name: Some("Concordium".to_owned()),
            created_at: 1,
            updated_at: 1,
            last_seen_at: None,
        }
    }

    fn ok_reply(data: impl Into<Vec<u8>>) -> Vec<u8> {
        let mut data = data.into();
        data.extend_from_slice(&[0x90, 0x00]);
        data
    }

    fn exported_key_material(values: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in values {
            bytes.push(EXPORTED_KEY_LENGTH as u8);
            bytes.extend_from_slice(&[*value; EXPORTED_KEY_LENGTH]);
        }
        bytes
    }

    fn exported_identity_material() -> Vec<u8> {
        exported_key_material(&[1, 2, 3])
    }

    fn exported_account_creation_material() -> Vec<u8> {
        let mut bytes = exported_key_material(&[1, 2]);
        bytes.push(EXPORTED_EXTENDED_NODE_LENGTH as u8);
        bytes.extend(3..=66);
        bytes
    }

    fn exported_account_creation_material_for_credential() -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [
            "57ae5c7c108bf3eeecb34bc79a390c4d4662cefab2d95316cbdb8e68fa1632b8",
            "7392eb0b4840c8a6f9314e99a8dd3e2c3663a1e615d8820851e3abd2965fab18",
        ] {
            bytes.push(EXPORTED_KEY_LENGTH as u8);
            bytes.extend(hex::decode(value).unwrap());
        }
        bytes.push(EXPORTED_EXTENDED_NODE_LENGTH as u8);
        bytes.extend(3..=66);
        bytes
    }

    fn exported_recovery_material_for_credential() -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [
            "7392eb0b4840c8a6f9314e99a8dd3e2c3663a1e615d8820851e3abd2965fab18",
            "575851a4e0558d589a57544a4a9f5ad1bd8467126c1b6767d32f633ea03380e6",
        ] {
            bytes.push(EXPORTED_KEY_LENGTH as u8);
            bytes.extend(hex::decode(value).unwrap());
        }
        bytes
    }

    type CredentialTestContext = (
        IpInfo<IpPairing>,
        BTreeMap<
            concordium_rust_sdk::id::types::ArIdentity,
            concordium_rust_sdk::id::types::ArInfo<ArCurve>,
        >,
        GlobalContext<ArCurve>,
        IdentityObjectV1<IpPairing, ArCurve, AttributeKind>,
    );

    fn construct_test_credential(
        app: &mut ConcordiumLedgerApp<MockTransport>,
    ) -> Result<AccountCredentialMessage<IpPairing, ArCurve, AttributeKind>> {
        let (ip_info, ar_infos, global_context, identity_object) = credential_test_context();
        construct_credential_deployment_with_app(
            LedgerAccountCredentialDeploymentInput {
                export_network: ExportPrivateKeyNetwork::Testnet,
                coin_type: 919,
                ip_info: &ip_info,
                ar_infos: &ar_infos,
                global_context: &global_context,
                identity_object,
                ip_identity: 7,
                identity_index: 9,
                credential_counter: 0,
                approved_secret_export: true,
            },
            app,
        )
    }

    fn credential_test_context() -> CredentialTestContext {
        let ip_info = serde_json::from_str::<Versioned<IpInfo<IpPairing>>>(include_str!(
            "testdata/ledger_account_creation/ip_info.json"
        ))
        .unwrap()
        .value;
        let ar_infos = serde_json::from_str::<
            Versioned<
                BTreeMap<
                    concordium_rust_sdk::id::types::ArIdentity,
                    concordium_rust_sdk::id::types::ArInfo<ArCurve>,
                >,
            >,
        >(include_str!(
            "testdata/ledger_account_creation/ars_infos.json"
        ))
        .unwrap()
        .value;
        let global_context = serde_json::from_str::<Versioned<GlobalContext<ArCurve>>>(
            include_str!("testdata/ledger_account_creation/global.json"),
        )
        .unwrap()
        .value;
        let identity_object =
            serde_json::from_str::<Versioned<IdentityObjectV1<IpPairing, ArCurve, AttributeKind>>>(
                include_str!("testdata/ledger_account_creation/identity-object.json"),
            )
            .unwrap()
            .value;
        (ip_info, ar_infos, global_context, identity_object)
    }

    #[test]
    fn identity_construction_requires_explicit_export_approval_before_apdu() {
        let details = details();
        let mut app = ConcordiumLedgerApp::new(MockTransport::default());
        let err = match construct_identity_issuance(
            LedgerIdentityIssuanceInput {
                owner_details: &details,
                network_genesis_hash: "net",
                export_network: ExportPrivateKeyNetwork::Mainnet,
                ip_identity: 7,
                identity_index: 0,
                approved_secret_export: false,
            },
            &mut app,
        ) {
            Ok(_) => panic!("identity construction unexpectedly succeeded"),
            Err(err) => err,
        };

        assert!(
            err.to_string()
                .contains("requires explicit secret-export approval")
        );
        assert!(app.transport().commands().is_empty());
    }

    #[test]
    fn identity_construction_exports_purpose_material_with_network_designation() {
        let details = details();
        let mut app =
            ConcordiumLedgerApp::new(MockTransport::new([ok_reply(exported_identity_material())]));

        construct_identity_issuance(
            LedgerIdentityIssuanceInput {
                owner_details: &details,
                network_genesis_hash: "net",
                export_network: ExportPrivateKeyNetwork::Testnet,
                ip_identity: 7,
                identity_index: 9,
                approved_secret_export: true,
            },
            &mut app,
        )
        .unwrap();

        let commands = app.transport().commands();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].ins, Instruction::ExportPrivateKeyNew.as_u8());
        assert_eq!(commands[0].p1, 0x00);
        assert_eq!(commands[0].p2, 0x01);
        assert_eq!(
            commands[0].data,
            [7u32.to_be_bytes(), 9u32.to_be_bytes()].concat()
        );
    }

    #[test]
    fn identity_recovery_uses_id_recovery_purpose() {
        let details = details();
        let mut app =
            ConcordiumLedgerApp::new(MockTransport::new([ok_reply(exported_key_material(&[
                1, 2,
            ]))]));

        construct_identity_recovery(
            LedgerIdentityIssuanceInput {
                owner_details: &details,
                network_genesis_hash: "net",
                export_network: ExportPrivateKeyNetwork::Testnet,
                ip_identity: 7,
                identity_index: 9,
                approved_secret_export: true,
            },
            &mut app,
        )
        .unwrap();

        let commands = app.transport().commands();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].ins, Instruction::ExportPrivateKeyNew.as_u8());
        assert_eq!(commands[0].p1, 0x02);
        assert_eq!(commands[0].p2, 0x01);
        assert_eq!(
            commands[0].data,
            [7u32.to_be_bytes(), 9u32.to_be_bytes()].concat()
        );
    }

    #[test]
    fn account_discovery_uses_account_credential_discovery_purpose() {
        let details = details();
        let mut app =
            ConcordiumLedgerApp::new(MockTransport::new([ok_reply(exported_key_material(&[3]))]));

        construct_account_credential_discovery(
            LedgerIdentityIssuanceInput {
                owner_details: &details,
                network_genesis_hash: "net",
                export_network: ExportPrivateKeyNetwork::Testnet,
                ip_identity: 7,
                identity_index: 9,
                approved_secret_export: true,
            },
            &mut app,
        )
        .unwrap();

        let commands = app.transport().commands();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].ins, Instruction::ExportPrivateKeyNew.as_u8());
        assert_eq!(commands[0].p1, 0x03);
        assert_eq!(commands[0].p2, 0x01);
        assert_eq!(
            commands[0].data,
            [7u32.to_be_bytes(), 9u32.to_be_bytes()].concat()
        );
    }

    #[test]
    fn unsupported_export_protocol_reports_update_message() {
        let details = details();
        let mut app = ConcordiumLedgerApp::new(MockTransport::new([vec![0x6B, 0x03]]));

        let err = match construct_identity_issuance(
            LedgerIdentityIssuanceInput {
                owner_details: &details,
                network_genesis_hash: "net",
                export_network: ExportPrivateKeyNetwork::Testnet,
                ip_identity: 7,
                identity_index: 9,
                approved_secret_export: true,
            },
            &mut app,
        ) {
            Ok(_) => panic!("identity construction unexpectedly succeeded"),
            Err(err) => err,
        };

        assert!(err.to_string().contains("5.5.0 or newer"));
        assert_eq!(app.transport().commands().len(), 1);
        assert_eq!(
            app.transport().commands()[0].ins,
            Instruction::ExportPrivateKeyNew.as_u8()
        );
    }

    #[test]
    fn purpose_identity_export_is_parsed() {
        parse_identity_credential_creation_export(&exported_identity_material()).unwrap();
    }

    #[test]
    fn malformed_identity_export_is_rejected() {
        let err = match parse_identity_credential_creation_export(&[31, 0]) {
            Ok(_) => panic!("malformed export unexpectedly parsed"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("expected 99 bytes"));
    }

    #[test]
    fn legacy_raw_response_is_rejected_for_identity_issuance() {
        let err = match parse_identity_credential_creation_export(&[7; 64]) {
            Ok(_) => panic!("legacy raw export unexpectedly parsed"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("expected 99 bytes"));
    }

    #[test]
    fn account_creation_export_is_parsed_with_exact_framing() {
        let material =
            parse_account_creation_export(&exported_account_creation_material()).unwrap();
        assert_eq!(*material.prf_key, [1; EXPORTED_KEY_LENGTH]);
        assert_eq!(*material.id_cred_sec, [2; EXPORTED_KEY_LENGTH]);
        assert!(
            material
                .commitment_randomness_root
                .derive_hardened_child(0)
                .is_ok()
        );
    }

    #[test]
    fn truncated_account_creation_export_is_rejected() {
        let mut bytes = exported_account_creation_material();
        bytes.pop();
        let err = match parse_account_creation_export(&bytes) {
            Ok(_) => panic!("truncated export unexpectedly parsed"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("truncated"));
    }

    #[test]
    fn wrong_length_account_creation_export_is_rejected() {
        let mut bytes = exported_account_creation_material();
        bytes[0] = 31;
        let err = match parse_account_creation_export(&bytes) {
            Ok(_) => panic!("wrong-length export unexpectedly parsed"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("expected 32 bytes"));
    }

    #[test]
    fn trailing_account_creation_export_bytes_are_rejected() {
        let mut bytes = exported_account_creation_material();
        bytes.push(0);
        let err = match parse_account_creation_export(&bytes) {
            Ok(_) => panic!("export with trailing bytes unexpectedly parsed"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("unexpected trailing bytes"));
    }

    #[test]
    fn unsupported_account_export_fails_before_recovery_export_or_signing() {
        let signing_key = SigningKey::from_bytes(&[42; 32]);
        let mut app = ConcordiumLedgerApp::new(MockTransport::new([
            ok_reply(signing_key.verifying_key().to_bytes()),
            vec![0x6B, 0x03],
        ]));

        let err = construct_test_credential(&mut app).unwrap_err();

        assert!(err.to_string().contains("5.6.4 or newer"));
        let commands = app.transport().commands();
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].ins, Instruction::GetPublicKey.as_u8());
        assert_eq!(commands[1].ins, Instruction::ExportPrivateKeyNew.as_u8());
    }

    #[test]
    fn malformed_account_export_fails_before_recovery_export_or_signing() {
        let signing_key = SigningKey::from_bytes(&[42; 32]);
        let mut app = ConcordiumLedgerApp::new(MockTransport::new([
            ok_reply(signing_key.verifying_key().to_bytes()),
            ok_reply([31, 0]),
        ]));

        let err = construct_test_credential(&mut app).unwrap_err();

        assert!(err.to_string().contains("expected 32 bytes"));
        assert_eq!(app.transport().commands().len(), 2);
    }

    #[test]
    fn signing_error_returns_no_credential_message() {
        let signing_key = SigningKey::from_bytes(&[42; 32]);
        let mut app = ConcordiumLedgerApp::new(MockTransport::new([
            ok_reply(signing_key.verifying_key().to_bytes()),
            ok_reply(exported_account_creation_material_for_credential()),
            ok_reply(exported_recovery_material_for_credential()),
        ]));

        assert!(construct_test_credential(&mut app).is_err());
        assert!(
            app.transport()
                .commands()
                .iter()
                .any(|command| command.ins == Instruction::SignCredentialDeployment.as_u8())
        );
    }

    #[test]
    fn account_construction_rejects_mismatched_id_cred_sec_before_public_key_or_signing() {
        let (ip_info, ar_infos, global_context, identity_object) = credential_test_context();
        let mut mismatched_recovery = exported_recovery_material_for_credential();
        mismatched_recovery[1] ^= 1;
        let signing_key = SigningKey::from_bytes(&[42; 32]);
        let mut app = ConcordiumLedgerApp::new(MockTransport::new([
            ok_reply(signing_key.verifying_key().to_bytes()),
            ok_reply(exported_account_creation_material_for_credential()),
            ok_reply(mismatched_recovery),
        ]));

        let err = construct_credential_deployment_with_app(
            LedgerAccountCredentialDeploymentInput {
                export_network: ExportPrivateKeyNetwork::Testnet,
                coin_type: 919,
                ip_info: &ip_info,
                ar_infos: &ar_infos,
                global_context: &global_context,
                identity_object,
                ip_identity: 7,
                identity_index: 9,
                credential_counter: 0,
                approved_secret_export: true,
            },
            &mut app,
        )
        .unwrap_err();

        assert!(err.to_string().contains("different IDCredSec"));
        let commands = app.transport().commands();
        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0].ins, Instruction::GetPublicKey.as_u8());
        assert!(
            commands[1..]
                .iter()
                .all(|command| command.ins == Instruction::ExportPrivateKeyNew.as_u8())
        );
        assert_eq!(commands[1].p1, 0x01);
        assert_eq!(commands[2].p1, 0x02);
    }

    #[test]
    fn account_construction_stages_credential_signing_and_assembles_device_signature() {
        let (ip_info, ar_infos, global_context, identity_object) = credential_test_context();
        let signing_key = SigningKey::from_bytes(&[42; 32]);
        let signature = [9; 64];
        let mut replies = vec![
            ok_reply(signing_key.verifying_key().to_bytes()),
            ok_reply(exported_account_creation_material_for_credential()),
            ok_reply(exported_recovery_material_for_credential()),
        ];
        replies.extend((0..100).map(|_| ok_reply(signature)));
        let mut app = ConcordiumLedgerApp::new(MockTransport::new(replies));

        let message = construct_credential_deployment_with_app(
            LedgerAccountCredentialDeploymentInput {
                export_network: ExportPrivateKeyNetwork::Testnet,
                coin_type: 919,
                ip_info: &ip_info,
                ar_infos: &ar_infos,
                global_context: &global_context,
                identity_object,
                ip_identity: 7,
                identity_index: 9,
                credential_counter: 0,
                approved_secret_export: true,
            },
            &mut app,
        )
        .unwrap();

        let AccountCredential::Normal { cdi } = message.credential else {
            panic!("expected a normal credential deployment");
        };
        assert_eq!(
            cdi.proofs
                .proof_acc_sk
                .sigs
                .get(&concordium_rust_sdk::common::types::KeyIndex(0))
                .unwrap()
                .to_bytes(),
            signature
        );

        let commands = app.transport().commands();
        assert_eq!(commands[0].ins, Instruction::GetPublicKey.as_u8());
        assert_eq!(commands[1].ins, Instruction::ExportPrivateKeyNew.as_u8());
        assert_eq!(commands[1].p1, 0x01);
        assert_eq!(commands[1].p2, 0x01);
        assert_eq!(
            commands[1].data,
            [7u32.to_be_bytes(), 9u32.to_be_bytes(), 0u32.to_be_bytes()].concat()
        );
        assert_eq!(commands[2].ins, Instruction::ExportPrivateKeyNew.as_u8());
        assert_eq!(commands[2].p1, 0x02);
        let signing_commands = &commands[3..];
        assert!(signing_commands.len() > 1);
        assert!(
            signing_commands
                .iter()
                .all(|command| command.ins == Instruction::SignCredentialDeployment.as_u8())
        );
        assert_eq!(signing_commands[0].p1, 0x00);

        let threshold = signing_commands
            .iter()
            .find(|command| command.p1 == 0x02)
            .unwrap();
        assert_eq!(threshold.data.len(), 56);
        let mut expected_threshold_suffix = serialize(&cdi.values.threshold);
        expected_threshold_suffix.extend_from_slice(&3u16.to_be_bytes());
        assert!(threshold.data.ends_with(&expected_threshold_suffix));

        let verification_keys = signing_commands
            .iter()
            .filter(|command| command.p1 == 0x01)
            .collect::<Vec<_>>();
        assert_eq!(verification_keys.len(), cdi.values.cred_key_info.keys.len());

        let ar_data = signing_commands
            .iter()
            .filter(|command| command.p1 == 0x03)
            .collect::<Vec<_>>();
        assert_eq!(ar_data.len(), 3);
        assert!(ar_data.iter().all(|command| command.data.len() == 100));

        assert_eq!(signing_commands.last().unwrap().p1, 0x09);
        assert_eq!(signing_commands.last().unwrap().data[0], 0);
    }
}
