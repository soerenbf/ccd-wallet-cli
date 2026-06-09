//! Ledger identity/account construction bridge.
//!
//! This module is the explicit boundary between wallet orchestration and the
//! low-level Concordium Ledger APDU client. It prevents Ledger-backed flows from
//! silently falling back to seed-derived construction or private-key export when
//! a high-level identity/account flow is not yet representable end-to-end.

use anyhow::{Result, bail};
use ccd_wallet_core::{
    store::signer_owners::LedgerOwnerDetailsRecord,
    wallet::{CredId, PrfKey},
};
use ccd_wallet_identity_provider::IdentityIssuanceMaterial;
use ccd_wallet_ledger::{
    ConcordiumLedgerApp, ExportPrivateKeyNetwork, ExportPrivateKeyNewRequest,
    ExportPrivateKeyNewType, LedgerError, LedgerTransport,
};
use concordium_rust_sdk::base::{
    curve_arithmetic::Curve,
    id::{constants::ArCurve, ps_sig::SigRetrievalRandomness},
};
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

/// Inputs that identify a Ledger-backed credential deployment construction request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LedgerCredentialDeploymentInput<'a> {
    /// Enrolled Ledger owner details matched against the connected device.
    pub owner_details: &'a LedgerOwnerDetailsRecord,
    /// Network genesis hash for the account creation flow.
    pub network_genesis_hash: &'a str,
    /// Identity provider index.
    pub ip_identity: u32,
    /// Identity index under the Ledger key source.
    pub identity_index: u32,
    /// Credential counter for the derived account.
    pub credential_counter: u8,
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

/// Prepare a Ledger-backed account credential deployment.
///
/// # Arguments
/// * `input` - Ledger owner and account derivation coordinates.
///
/// # Errors
/// Returns an unsupported-flow error until the wallet has an end-to-end
/// implementation that can construct and sign credential deployments through
/// supported Ledger app operations without implicit private-key export.
///
/// # Examples
///
/// ```ignore
/// let deployment = ledger_construction::construct_credential_deployment(input)?;
/// ```
pub(crate) fn construct_credential_deployment(
    input: LedgerCredentialDeploymentInput<'_>,
) -> Result<()> {
    bail!(
        "Ledger account creation is not yet supported for key source '{}' on network '{}' (provider {}, identity index {}, credential {}); no transaction was submitted",
        input.owner_details.fingerprint,
        input.network_genesis_hash,
        input.ip_identity,
        input.identity_index,
        input.credential_counter,
    )
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
    let exported = app.export_private_key_new(&ExportPrivateKeyNewRequest {
        export_type,
        network: input.export_network,
        payload: build_identity_issuance_export_payload(input.ip_identity, input.identity_index),
    })?;
    Ok(Zeroizing::new(exported))
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
    let id_cred_sec = bls_scalar_from_export(*chunks.next_key("IDCredSec")?);
    let prf_key = PrfKey::new(bls_scalar_from_export(*chunks.next_key("PRFKey")?));
    let blinding_randomness = SigRetrievalRandomness::new(bls_scalar_from_export(
        *chunks.next_key("signature blinding randomness")?,
    ));
    chunks.finish_with_expected_count(PURPOSE_IDENTITY_EXPORT_KEY_COUNT)?;

    Ok(IdentityIssuanceMaterial {
        id_cred_sec,
        prf_key,
        blinding_randomness,
    })
}

fn parse_identity_recovery_export(bytes: &[u8]) -> Result<CredId> {
    if bytes.len() != PURPOSE_ID_RECOVERY_EXPORT_RESPONSE_LENGTH {
        bail!(
            "Ledger identity recovery export response was {} bytes; expected {} bytes from Concordium Ledger app 5.5.0+ purpose-based identity recovery export",
            bytes.len(),
            PURPOSE_ID_RECOVERY_EXPORT_RESPONSE_LENGTH,
        );
    }

    let mut chunks = PurposeExportChunks::new(bytes);
    let id_cred_sec = bls_scalar_from_export(*chunks.next_key("IDCredSec")?);
    let _blinding_randomness = chunks.next_key("signature blinding randomness")?;
    chunks.finish_with_expected_count(PURPOSE_ID_RECOVERY_EXPORT_KEY_COUNT)?;
    Ok(id_cred_sec)
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
    let prf_key = PrfKey::new(bls_scalar_from_export(*chunks.next_key("PRFKey")?));
    chunks.finish_with_expected_count(PURPOSE_ACCOUNT_DISCOVERY_EXPORT_KEY_COUNT)?;
    Ok(prf_key)
}

fn bls_scalar_from_export(bytes: [u8; EXPORTED_KEY_LENGTH]) -> CredId {
    ArCurve::scalar_from_bytes(bytes)
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
        if self.offset >= self.bytes.len() {
            bail!("Ledger export response is missing {label}");
        }
        let length = self.bytes[self.offset] as usize;
        self.offset += 1;
        if length != EXPORTED_KEY_LENGTH {
            bail!(
                "Ledger export response has {length}-byte {label}; expected {EXPORTED_KEY_LENGTH} bytes (total response length: {} bytes)",
                self.bytes.len(),
            );
        }
        let end = self.offset + EXPORTED_KEY_LENGTH;
        if end > self.bytes.len() {
            bail!("Ledger export response truncated while reading {label}");
        }
        let mut key = Zeroizing::new([0u8; EXPORTED_KEY_LENGTH]);
        key.copy_from_slice(&self.bytes[self.offset..end]);
        self.offset = end;
        self.keys_read += 1;
        Ok(key)
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
    fn account_construction_fails_before_submission_when_unsupported() {
        let details = details();
        let err = construct_credential_deployment(LedgerCredentialDeploymentInput {
            owner_details: &details,
            network_genesis_hash: "net",
            ip_identity: 7,
            identity_index: 0,
            credential_counter: 0,
        })
        .unwrap_err();
        assert!(err.to_string().contains("not yet supported"));
        assert!(err.to_string().contains("no transaction was submitted"));
    }
}
