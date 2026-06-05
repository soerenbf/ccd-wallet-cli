//! Low-level Concordium Governance Ledger app protocol client.
//!
//! This crate provides a command-oriented foundation for talking to the Concordium
//! Governance application running on a Ledger hardware wallet. It translates typed
//! governance-oriented request values into APDU command sequences, performs
//! command-specific staging and chunking, and returns raw device outputs such as public
//! keys and signatures.
//!
//! Supported command families include public key retrieval, higher-level governance key
//! updates, level-2 authorization updates, protocol updates, exchange-rate updates,
//! transaction-fee-distribution updates, GAS rewards updates, foundation-account updates,
//! mint-distribution updates, baker-stake-threshold updates, cooldown/pool/time/timeout
//! parameter updates, consensus parameter updates, add-anonymity-revoker updates,
//! add-identity-provider updates, validator-score-parameter updates, and create-PLT
//! updates.
//!
//! Non-goals:
//! - no wallet database or governance key vault access,
//! - no signer selection or CLI UX,
//! - no signed update instruction or block-item assembly,
//! - no node submission or finalization tracking,
//! - no blind signing for unknown serialized governance payloads.
//!
//! # Features
//!
//! - `hid`: enabled by default, provides concrete HID transport support.
//! - `sdk`: enables conversions from selected `concordium-rust-sdk` governance/update types
//!   into crate-local request/value types.

pub mod apdu;
pub mod commands;
pub mod error;
pub mod serialization;
pub mod transport;
pub mod types;

pub use apdu::ApduCommand;
pub use error::{GovernanceLedgerError, Result};
#[cfg(feature = "hid")]
pub use transport::HidTransport;
pub use transport::{GovernanceLedgerTransport, MockTransport};
pub use types::{
    AccessStructureUpdate, AddAnonymityRevokerRequest, AddIdentityProviderRequest,
    AuthorizationsKeyUpdateType, AuthorizationsUpdateRequest, AuthorizationsVersion,
    BakerStakeThresholdUpdateRequest, BlockEnergyLimitUpdateRequest,
    CooldownParametersUpdateRequest, CreatePltRequest, DerivationPath, DescriptionFields,
    ExchangeRateUpdateRequest, FinalizationCommitteeParametersUpdateRequest, FixedUpdateRequest,
    FoundationAccountUpdateRequest, GasRewardsUpdateRequest, GovernancePublicKeyEntry,
    GovernanceUpdatePrefix, HigherLevelKeyUpdateRequest, HigherLevelKeyUpdateType,
    MinBlockTimeUpdateRequest, MintDistributionUpdateRequest, PoolParametersUpdateRequest,
    ProtocolUpdateRequest, PublicKeyOptions, PublicKeyRequest, PublicKeyResponse, RawSignature,
    TimeParametersUpdateRequest, TimeoutParametersUpdateRequest,
    TransactionFeeDistributionUpdateRequest, UpdateHeaderBytes,
    ValidatorScoreParametersUpdateRequest, harden,
};

use apdu::Instruction;

/// Low-level client for the Concordium Governance Ledger app over an APDU transport.
#[derive(Clone, Debug)]
pub struct GovernanceLedgerApp<T> {
    transport: T,
}

impl<T> GovernanceLedgerApp<T> {
    /// Construct a Governance Ledger app client over a transport.
    ///
    /// # Arguments
    ///
    /// * `transport` - APDU transport implementation used for command exchange.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger_governance::{GovernanceLedgerApp, MockTransport};
    /// let app = GovernanceLedgerApp::new(MockTransport::default());
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

impl<T: GovernanceLedgerTransport> GovernanceLedgerApp<T> {
    /// Retrieve a governance public key from the Governance Ledger app.
    ///
    /// # Arguments
    ///
    /// * `path` - Derivation path for the requested key.
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

    /// Sign an exchange-rate update and return the raw signature.
    pub fn sign_exchange_rate(
        &mut self,
        request: &ExchangeRateUpdateRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_fixed_update(
            &mut self.transport,
            Instruction::UpdateExchangeRate,
            request,
        )
    }

    /// Sign a transaction-fee-distribution update and return the raw signature.
    pub fn sign_transaction_fee_distribution(
        &mut self,
        request: &TransactionFeeDistributionUpdateRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_fixed_update(
            &mut self.transport,
            Instruction::UpdateTransactionFeeDistribution,
            request,
        )
    }

    /// Sign a GAS rewards update and return the raw signature.
    pub fn sign_gas_rewards(&mut self, request: &GasRewardsUpdateRequest) -> Result<RawSignature> {
        commands::signing::sign_fixed_update(
            &mut self.transport,
            Instruction::UpdateGasRewards,
            request,
        )
    }

    /// Sign a foundation-account update and return the raw signature.
    pub fn sign_foundation_account(
        &mut self,
        request: &FoundationAccountUpdateRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_fixed_update(
            &mut self.transport,
            Instruction::UpdateFoundationAccount,
            request,
        )
    }

    /// Sign a mint-distribution update and return the raw signature.
    pub fn sign_mint_distribution(
        &mut self,
        request: &MintDistributionUpdateRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_fixed_update(
            &mut self.transport,
            Instruction::UpdateMintDistribution,
            request,
        )
    }

    /// Sign a baker-stake-threshold update and return the raw signature.
    pub fn sign_baker_stake_threshold(
        &mut self,
        request: &BakerStakeThresholdUpdateRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_fixed_update(
            &mut self.transport,
            Instruction::UpdateBakerStakeThreshold,
            request,
        )
    }

    /// Sign a cooldown-parameters update and return the raw signature.
    pub fn sign_cooldown_parameters(
        &mut self,
        request: &CooldownParametersUpdateRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_fixed_update(
            &mut self.transport,
            Instruction::UpdateCooldownParameters,
            request,
        )
    }

    /// Sign a pool-parameters update and return the raw signature.
    pub fn sign_pool_parameters(
        &mut self,
        request: &PoolParametersUpdateRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_fixed_update(
            &mut self.transport,
            Instruction::UpdatePoolParameters,
            request,
        )
    }

    /// Sign a time-parameters update and return the raw signature.
    pub fn sign_time_parameters(
        &mut self,
        request: &TimeParametersUpdateRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_fixed_update(
            &mut self.transport,
            Instruction::UpdateTimeParameters,
            request,
        )
    }

    /// Sign a timeout-parameters update and return the raw signature.
    pub fn sign_timeout_parameters(
        &mut self,
        request: &TimeoutParametersUpdateRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_fixed_update(
            &mut self.transport,
            Instruction::UpdateTimeoutParameters,
            request,
        )
    }

    /// Sign a minimum-block-time update and return the raw signature.
    pub fn sign_min_block_time(
        &mut self,
        request: &MinBlockTimeUpdateRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_fixed_update(
            &mut self.transport,
            Instruction::UpdateMinBlockTime,
            request,
        )
    }

    /// Sign a block-energy-limit update and return the raw signature.
    pub fn sign_block_energy_limit(
        &mut self,
        request: &BlockEnergyLimitUpdateRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_fixed_update(
            &mut self.transport,
            Instruction::UpdateBlockEnergyLimit,
            request,
        )
    }

    /// Sign a finalization-committee-parameters update and return the raw signature.
    pub fn sign_finalization_committee_parameters(
        &mut self,
        request: &FinalizationCommitteeParametersUpdateRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_fixed_update(
            &mut self.transport,
            Instruction::UpdateFinalizationCommitteeParameters,
            request,
        )
    }

    /// Sign a validator-score-parameters update and return the raw signature.
    pub fn sign_validator_score_parameters(
        &mut self,
        request: &ValidatorScoreParametersUpdateRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_fixed_update(
            &mut self.transport,
            Instruction::UpdateValidatorScoreParameters,
            request,
        )
    }

    /// Sign a protocol update and return the raw signature.
    pub fn sign_protocol_update(
        &mut self,
        request: &ProtocolUpdateRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_protocol_update(&mut self.transport, request)
    }

    /// Sign an add-anonymity-revoker update and return the raw signature.
    pub fn sign_add_anonymity_revoker(
        &mut self,
        request: &AddAnonymityRevokerRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_add_anonymity_revoker(&mut self.transport, request)
    }

    /// Sign an add-identity-provider update and return the raw signature.
    pub fn sign_add_identity_provider(
        &mut self,
        request: &AddIdentityProviderRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_add_identity_provider(&mut self.transport, request)
    }

    /// Sign a create-PLT update and return the raw signature.
    pub fn sign_create_plt(&mut self, request: &CreatePltRequest) -> Result<RawSignature> {
        commands::signing::sign_create_plt(&mut self.transport, request)
    }

    /// Sign an update-root-keys flow using root keys and return the raw signature.
    pub fn sign_update_root_keys(
        &mut self,
        request: &HigherLevelKeyUpdateRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_higher_level_keys(
            &mut self.transport,
            Instruction::UpdateRootKeys,
            request,
        )
    }

    /// Sign an update-level-1-keys flow using root keys and return the raw signature.
    pub fn sign_update_level1_keys_with_root_keys(
        &mut self,
        request: &HigherLevelKeyUpdateRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_higher_level_keys(
            &mut self.transport,
            Instruction::UpdateLevel1Keys,
            request,
        )
    }

    /// Sign an update-level-1-keys flow using level-1 keys and return the raw signature.
    pub fn sign_update_level1_keys_with_level1_keys(
        &mut self,
        request: &HigherLevelKeyUpdateRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_higher_level_keys(
            &mut self.transport,
            Instruction::UpdateLevel2KeysLevel1,
            request,
        )
    }

    /// Sign an update-level-2-authorizations flow using root keys and return the raw signature.
    pub fn sign_update_authorizations_with_root_keys(
        &mut self,
        request: &AuthorizationsUpdateRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_authorizations(
            &mut self.transport,
            Instruction::UpdateLevel2KeysRoot,
            request,
        )
    }

    /// Sign an update-level-2-authorizations flow using level-1 keys and return the raw signature.
    pub fn sign_update_authorizations_with_level1_keys(
        &mut self,
        request: &AuthorizationsUpdateRequest,
    ) -> Result<RawSignature> {
        commands::signing::sign_authorizations(
            &mut self.transport,
            Instruction::UpdateLevel2KeysLevel1,
            request,
        )
    }
}
