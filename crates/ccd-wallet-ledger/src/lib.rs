//! Low-level Concordium Ledger app protocol client.
//!
//! This crate provides a command-oriented foundation for talking to the Concordium
//! application running on a Ledger hardware wallet. It is intentionally scoped as a
//! protocol client: it translates typed Concordium-oriented request values into APDU
//! command sequences, performs command-specific chunking, and returns raw device
//! outputs such as public keys, signatures, address-verification status, app metadata,
//! and exported byte payloads.
//!
//! Supported command families include public key retrieval, address verification,
//! transfer signing, transfer-with-memo signing, scheduled transfer signing,
//! configure-delegation/baker signing, register-data signing, transfer-to-public
//! signing, deploy-module signing, contract init/update signing, public-info-for-IP
//! signing, credential-deployment signing, update-credentials signing, PLT signing,
//! app-name lookup, and private-key export commands.
//!
//! Non-goals:
//! - no wallet database or account selection,
//! - no password prompts or CLI UX,
//! - no signed transaction assembly,
//! - no node submission or finalization tracking.
//!
//! # Features
//!
//! - `sdk`: enables conversion impls from selected `concordium-rust-sdk` types into
//!   crate-local request/value types.
//! - `hid`: enables the concrete Ledger HID APDU transport adapter.

pub mod apdu;
pub mod commands;
pub mod error;
#[cfg(feature = "sdk")]
pub mod sdk;
pub mod serialization;
pub mod transport;
pub mod types;

pub use apdu::ApduCommand;
pub use error::{LedgerError, Result};
#[cfg(feature = "hid")]
pub use transport::HidTransport;
pub use transport::{LedgerTransport, MockTransport};
pub use types::{
    AccountAddressBytes, ChunkedSigningRequest, ConfigureBakerSigningRequest,
    ContractSigningRequest, CredentialAttribute, CredentialDeploymentContext,
    CredentialDeploymentSigningRequest, CredentialSigningPayload, DeployModuleSigningRequest,
    DerivationPath, ExportPrivateKeyLegacyRequest, ExportPrivateKeyNewRequest,
    ExportPrivateKeyNewType, LegacyVerifyAddressRequest, PublicInfoForIpSigningRequest,
    PublicKeyOptions, PublicKeyRequest, PublicKeyResponse, RawSignature,
    RegisterDataSigningRequest, ScheduledTransferSigningRequest,
    ScheduledTransferWithMemoSigningRequest, TransferToPublicSigningRequest,
    TransferWithMemoSigningRequest, UpdateCredentialEntry, UpdateCredentialsSigningRequest,
    VerifyAddressRequest, harden,
};

use apdu::Instruction;

/// Low-level client for the Concordium Ledger app over an APDU transport.
#[derive(Clone, Debug)]
pub struct ConcordiumLedgerApp<T> {
    transport: T,
}

impl<T> ConcordiumLedgerApp<T> {
    /// Construct a Concordium Ledger app client over a transport.
    ///
    /// # Arguments
    ///
    /// * `transport` - APDU transport implementation used for command exchange.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger::{ConcordiumLedgerApp, MockTransport};
    /// let app = ConcordiumLedgerApp::new(MockTransport::default());
    /// let _transport = app.into_transport();
    /// ```
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    /// Return an immutable reference to the underlying transport.
    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Return a mutable reference to the underlying transport.
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    /// Consume the client and return the underlying transport.
    pub fn into_transport(self) -> T {
        self.transport
    }
}

impl<T: LedgerTransport> ConcordiumLedgerApp<T> {
    /// Retrieve a public key from the Ledger app.
    ///
    /// # Arguments
    ///
    /// * `path` - Derivation path for the requested public key.
    /// * `options` - Device confirmation and signed-key options.
    ///
    /// # Errors
    ///
    /// Returns an error if APDU exchange fails or the response is malformed.
    pub fn get_public_key(
        &mut self,
        path: DerivationPath,
        options: PublicKeyOptions,
    ) -> Result<PublicKeyResponse> {
        commands::public_key::get_public_key(
            &mut self.transport,
            &PublicKeyRequest { path, options },
        )
    }

    /// Verify an address on-device using the current address-verification command shape.
    ///
    /// # Arguments
    ///
    /// * `request` - Current address-verification payload.
    ///
    /// # Errors
    ///
    /// Returns an error if APDU exchange fails or the device rejects verification.
    pub fn verify_address(&mut self, request: &VerifyAddressRequest) -> Result<()> {
        commands::device::verify_address(&mut self.transport, request)
    }

    /// Verify an address on-device using the legacy address-verification command shape.
    ///
    /// # Arguments
    ///
    /// * `request` - Legacy address-verification payload.
    ///
    /// # Errors
    ///
    /// Returns an error if APDU exchange fails or the device rejects verification.
    pub fn verify_address_legacy(&mut self, request: &LegacyVerifyAddressRequest) -> Result<()> {
        commands::device::verify_address_legacy(&mut self.transport, request)
    }

    /// Query the raw app name bytes from the Ledger app.
    ///
    /// # Errors
    ///
    /// Returns an error if APDU exchange fails.
    pub fn get_app_name(&mut self) -> Result<Vec<u8>> {
        commands::device::get_app_name(&mut self.transport)
    }

    /// Export private-key material through the legacy Ledger app command.
    ///
    /// # Arguments
    ///
    /// * `request` - Legacy export mode, type, and payload.
    ///
    /// # Errors
    ///
    /// Returns an error if APDU exchange fails or the device rejects export.
    pub fn export_private_key_legacy(
        &mut self,
        request: &ExportPrivateKeyLegacyRequest,
    ) -> Result<Vec<u8>> {
        commands::device::export_private_key_legacy(&mut self.transport, request)
    }

    /// Export private-key material through the new Ledger app command.
    ///
    /// # Arguments
    ///
    /// * `request` - New export type and payload.
    ///
    /// # Errors
    ///
    /// Returns an error if APDU exchange fails or the device rejects export.
    pub fn export_private_key_new(
        &mut self,
        request: &ExportPrivateKeyNewRequest,
    ) -> Result<Vec<u8>> {
        commands::device::export_private_key_new(&mut self.transport, request)
    }

    /// Sign a simple-transfer transaction and return the raw signature.
    pub fn sign_transfer(&mut self, request: &ChunkedSigningRequest) -> Result<RawSignature> {
        commands::signing::sign_chunked(&mut self.transport, Instruction::SignTransfer, request)
    }

    /// Sign a transfer-with-memo transaction and return the raw signature.
    pub fn sign_transfer_with_memo(
        &mut self,
        request: &TransferWithMemoSigningRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_transfer_with_memo(&mut self.transport, request)
    }

    /// Sign a scheduled transfer transaction and return the raw signature.
    pub fn sign_scheduled_transfer(
        &mut self,
        request: &ScheduledTransferSigningRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_scheduled_transfer(&mut self.transport, request)
    }

    /// Sign a scheduled transfer with memo transaction and return the raw signature.
    pub fn sign_scheduled_transfer_with_memo(
        &mut self,
        request: &ScheduledTransferWithMemoSigningRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_scheduled_transfer_with_memo(&mut self.transport, request)
    }

    /// Sign a configure-delegation transaction and return the raw signature.
    pub fn sign_configure_delegation(
        &mut self,
        request: &ChunkedSigningRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_chunked(
            &mut self.transport,
            Instruction::SignConfigureDelegation,
            request,
        )
    }

    /// Sign a configure-baker transaction and return the raw signature.
    pub fn sign_configure_baker(
        &mut self,
        request: &ConfigureBakerSigningRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_configure_baker(&mut self.transport, request)
    }

    /// Sign a register-data transaction and return the raw signature.
    pub fn sign_register_data(
        &mut self,
        request: &RegisterDataSigningRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_register_data(&mut self.transport, request)
    }

    /// Sign a transfer-to-public transaction and return the raw signature.
    pub fn sign_transfer_to_public(
        &mut self,
        request: &TransferToPublicSigningRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_transfer_to_public(&mut self.transport, request)
    }

    /// Sign a deploy-module transaction and return the raw signature.
    pub fn sign_deploy_module(
        &mut self,
        request: &DeployModuleSigningRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_deploy_module(&mut self.transport, request)
    }

    /// Sign an init-contract transaction and return the raw signature.
    pub fn sign_init_contract(&mut self, request: &ContractSigningRequest) -> Result<RawSignature> {
        commands::signing::sign_init_contract(&mut self.transport, request)
    }

    /// Sign an update-contract transaction and return the raw signature.
    pub fn sign_update_contract(
        &mut self,
        request: &ContractSigningRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_update_contract(&mut self.transport, request)
    }

    /// Sign public information for an identity provider and return the raw signature.
    pub fn sign_public_info_for_ip(
        &mut self,
        request: &PublicInfoForIpSigningRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_public_info_for_ip(&mut self.transport, request)
    }

    /// Sign a credential deployment payload and return the raw signature.
    pub fn sign_credential_deployment(
        &mut self,
        request: &CredentialDeploymentSigningRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_credential_deployment(&mut self.transport, request)
    }

    /// Sign an update-credentials transaction and return the raw signature.
    pub fn sign_update_credentials(
        &mut self,
        request: &UpdateCredentialsSigningRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_update_credentials(&mut self.transport, request)
    }

    /// Sign a protocol-level token transaction and return the raw signature.
    pub fn sign_plt(&mut self, request: &ChunkedSigningRequest) -> Result<RawSignature> {
        commands::signing::sign_chunked(
            &mut self.transport,
            Instruction::SignPltTransaction,
            request,
        )
    }
}
