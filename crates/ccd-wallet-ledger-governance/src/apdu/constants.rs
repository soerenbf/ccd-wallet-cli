//! APDU constants for the Concordium Governance Ledger app protocol.

/// Concordium Governance Ledger app CLA byte.
pub const LEDGER_CLA: u8 = 0xE0;

/// Empty P1 or P2 parameter value.
pub const NONE: u8 = 0x00;

/// Maximum data payload accepted by the Ledger APDU envelope.
pub const MAX_APDU_PAYLOAD_SIZE: usize = 255;

/// P1 value for public-key retrieval with device confirmation.
pub const P1_PUBLIC_KEY_CONFIRM: u8 = 0x00;
/// P1 value for public-key retrieval without device confirmation.
pub const P1_PUBLIC_KEY_NO_CONFIRM: u8 = 0x01;
/// P2 value requesting that the device signs the returned public key.
pub const P2_SIGNED_PUBLIC_KEY: u8 = 0x01;

/// P1 value for an initial governance update packet.
pub const P1_INITIAL: u8 = 0x00;
/// P1 value used for length packets in staged text/description flows.
pub const P1_LENGTH: u8 = 0x01;
/// P1 value used for staged bytes following a length packet.
pub const P1_BYTES: u8 = 0x02;

/// Concordium Governance Ledger app instruction byte.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Instruction {
    /// Retrieve a public key.
    GetPublicKey,
    /// Sign an exchange-rate update.
    UpdateExchangeRate,
    /// Sign a protocol update.
    UpdateProtocol,
    /// Sign a transaction-fee-distribution update.
    UpdateTransactionFeeDistribution,
    /// Sign a GAS rewards update.
    UpdateGasRewards,
    /// Sign a foundation-account update.
    UpdateFoundationAccount,
    /// Sign a mint-distribution update.
    UpdateMintDistribution,
    /// Sign a baker-stake-threshold update.
    UpdateBakerStakeThreshold,
    /// Sign a root-key update.
    UpdateRootKeys,
    /// Sign a level-1-key update.
    UpdateLevel1Keys,
    /// Sign a level-2 authorization update with root keys.
    UpdateLevel2KeysRoot,
    /// Sign a level-2 authorization update with level-1 keys.
    UpdateLevel2KeysLevel1,
    /// Sign an add-anonymity-revoker update.
    AddAnonymityRevoker,
    /// Sign an add-identity-provider update.
    AddIdentityProvider,
    /// Sign a cooldown-parameters update.
    UpdateCooldownParameters,
    /// Sign a pool-parameters update.
    UpdatePoolParameters,
    /// Sign a time-parameters update.
    UpdateTimeParameters,
    /// Sign a timeout-parameters update.
    UpdateTimeoutParameters,
    /// Sign a minimum-block-time update.
    UpdateMinBlockTime,
    /// Sign a block-energy-limit update.
    UpdateBlockEnergyLimit,
    /// Sign a finalization-committee-parameters update.
    UpdateFinalizationCommitteeParameters,
    /// Sign a validator-score-parameters update.
    UpdateValidatorScoreParameters,
    /// Sign a create-PLT update.
    CreatePlt,
}

impl Instruction {
    /// Return the raw instruction byte used in APDU commands.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger_governance::apdu::Instruction;
    /// assert_eq!(Instruction::GetPublicKey.as_u8(), 0x01);
    /// ```
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::GetPublicKey => 0x01,
            Self::UpdateExchangeRate => 0x06,
            Self::UpdateProtocol => 0x21,
            Self::UpdateTransactionFeeDistribution => 0x22,
            Self::UpdateGasRewards => 0x23,
            Self::UpdateFoundationAccount => 0x24,
            Self::UpdateMintDistribution => 0x25,
            Self::UpdateBakerStakeThreshold => 0x27,
            Self::UpdateRootKeys => 0x28,
            Self::UpdateLevel1Keys => 0x29,
            Self::UpdateLevel2KeysRoot => 0x2A,
            Self::UpdateLevel2KeysLevel1 => 0x2B,
            Self::AddAnonymityRevoker => 0x2C,
            Self::AddIdentityProvider => 0x2D,
            Self::UpdateCooldownParameters => 0x40,
            Self::UpdatePoolParameters => 0x41,
            Self::UpdateTimeParameters => 0x42,
            Self::UpdateTimeoutParameters => 0x43,
            Self::UpdateMinBlockTime => 0x44,
            Self::UpdateBlockEnergyLimit => 0x45,
            Self::UpdateFinalizationCommitteeParameters => 0x46,
            Self::UpdateValidatorScoreParameters => 0x47,
            Self::CreatePlt => 0x48,
        }
    }
}
