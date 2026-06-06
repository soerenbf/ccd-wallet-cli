//! APDU constants for the Concordium Ledger app protocol.

/// Concordium Ledger app CLA byte.
pub const LEDGER_CLA: u8 = 0xE0;

/// Empty P1 or P2 parameter value.
pub const NONE: u8 = 0x00;

/// P2 marker indicating more chunks follow.
pub const P2_MORE: u8 = 0x80;

/// P2 marker indicating this is the final chunk.
pub const P2_LAST: u8 = 0x00;

/// Maximum data payload accepted by the Ledger APDU envelope used by the JS reference client.
pub const MAX_APDU_PAYLOAD_SIZE: usize = 255;

/// P1 value for legacy address verification.
pub const P1_LEGACY_VERIFY_ADDRESS: u8 = 0x00;
/// P1 value for current address verification.
pub const P1_VERIFY_ADDRESS: u8 = 0x01;

/// P1 value for public-key retrieval with device confirmation.
pub const P1_PUBLIC_KEY_CONFIRM: u8 = 0x00;
/// P1 value for public-key retrieval without device confirmation.
pub const P1_PUBLIC_KEY_NO_CONFIRM: u8 = 0x01;
/// P2 value requesting that the device signs the returned public key.
pub const P2_SIGNED_PUBLIC_KEY: u8 = 0x01;

/// P1 value for the first generic chunk.
pub const P1_FIRST_CHUNK: u8 = 0x00;
/// P1 value for the initial staged packet.
pub const P1_INITIAL_PACKET: u8 = 0x00;
/// P1 value for transfer-with-memo initial header/address/memo-length packet.
pub const P1_INITIAL_WITH_MEMO: u8 = 0x01;
/// P1 value for transfer-with-schedule-and-memo initial packet.
pub const P1_INITIAL_WITH_MEMO_SCHEDULE: u8 = 0x02;
/// P1 value for transfer-with-schedule-and-memo memo stage.
pub const P1_MEMO_SCHEDULE: u8 = 0x03;
/// P1 value for scheduled transfer pair uploads.
pub const P1_SCHEDULED_TRANSFER_PAIRS: u8 = 0x01;
/// P1 value used for memo bytes.
pub const P1_MEMO: u8 = 0x02;
/// P1 value used for amount bytes.
pub const P1_AMOUNT: u8 = 0x03;
/// P1 value used for register-data and PLT operation data chunks.
pub const P1_DATA: u8 = 0x01;
/// P1 value used for proof chunks.
pub const P1_PROOF: u8 = 0x02;
/// P1 value used for shielded transfer remaining amount/recipient/proof length.
pub const P1_REMAINING_AMOUNT: u8 = 0x01;

/// P1 value used for configure-baker first batch.
pub const P1_FIRST_BATCH: u8 = 0x01;
/// P1 value used for configure-baker aggregation key.
pub const P1_AGGREGATION_KEY: u8 = 0x02;
/// P1 value used for configure-baker URL length.
pub const P1_URL_LENGTH: u8 = 0x03;
/// P1 value used for configure-baker URL bytes.
pub const P1_URL: u8 = 0x04;
/// P1 value used for configure-baker commission-fee bytes.
pub const P1_COMMISSION_FEE: u8 = 0x05;
/// P1 value used for configure-baker suspended flag.
pub const P1_SUSPENDED: u8 = 0x06;

/// P1 value used for module source chunks in deploy-module signing.
pub const P1_SOURCE: u8 = 0x01;
/// P1 value used for contract name chunks in init/update contract signing.
pub const P1_NAME: u8 = 0x01;
/// P1 value used for contract parameter chunks in init/update contract signing.
pub const P1_PARAM: u8 = 0x02;

/// P2 value for the initial update-credentials stage.
pub const P2_CREDENTIAL_INITIAL: u8 = 0x00;
/// P2 value for an update-credentials credential-index stage.
pub const P2_CREDENTIAL_CREDENTIAL_INDEX: u8 = 0x01;
/// P2 value for update-credentials staged credential fields.
pub const P2_CREDENTIAL_CREDENTIAL: u8 = 0x02;
/// P2 value for update-credentials removed-credential count.
pub const P2_CREDENTIAL_ID_COUNT: u8 = 0x03;
/// P2 value for update-credentials removed credential identifiers.
pub const P2_CREDENTIAL_ID: u8 = 0x04;
/// P2 value for update-credentials account threshold.
pub const P2_THRESHOLD: u8 = 0x05;

/// P1 value used for credential verification-key count.
pub const P1_VERIFICATION_KEY_LENGTH: u8 = 0x0A;
/// P1 value used for credential verification-key fields.
pub const P1_VERIFICATION_KEY: u8 = 0x01;
/// P1 value used for credential signature threshold fields.
pub const P1_SIGNATURE_THRESHOLD: u8 = 0x02;
/// P1 value used for credential anonymity-revoker identity fields.
pub const P1_AR_IDENTITY: u8 = 0x03;
/// P1 value used for credential validity dates.
pub const P1_CREDENTIAL_DATES: u8 = 0x04;
/// P1 value used for credential attribute tag/value-length pair.
pub const P1_ATTRIBUTE_TAG: u8 = 0x05;
/// P1 value used for credential attribute value bytes.
pub const P1_ATTRIBUTE_VALUE: u8 = 0x06;
/// P1 value used for credential proof length.
pub const P1_LENGTH_OF_PROOFS: u8 = 0x07;
/// P1 value used for credential proof chunks.
pub const P1_PROOFS: u8 = 0x08;
/// P1 value used for new/existing credential deployment discriminator and context.
pub const P1_NEW_OR_EXISTING: u8 = 0x09;

/// P1 value for purpose-based identity credential creation export.
pub const P1_IDENTITY_CREDENTIAL_CREATION: u8 = 0x00;
/// P1 value for purpose-based account-creation key export.
pub const P1_ACCOUNT_CREATION: u8 = 0x01;
/// P1 value for purpose-based identity-recovery key export.
pub const P1_ID_RECOVERY: u8 = 0x02;
/// P1 value for purpose-based account-credential discovery key export.
pub const P1_ACCOUNT_CREDENTIAL_DISCOVERY: u8 = 0x03;
/// P1 value for purpose-based zero-knowledge proof creation key export.
pub const P1_CREATION_OF_ZK_PROOF: u8 = 0x04;

/// P1 value for legacy new-path PRF-key export.
pub const P1_NEW_PATH_LEGACY_PRF_KEY: u8 = 0x00;
/// P1 value for legacy new-path PRF-key recovery-display export.
pub const P1_NEW_PATH_LEGACY_PRF_KEY_RECOVERY: u8 = 0x01;
/// P1 value for legacy new-path PRF-key plus IDCredSec export.
pub const P1_NEW_PATH_LEGACY_PRF_KEY_AND_ID_CRED_SEC: u8 = 0x02;
/// P2 value for legacy new-path seed export.
pub const P2_NEW_PATH_LEGACY_SEED: u8 = 0x01;
/// P2 value for legacy new-path BLS-key export.
pub const P2_NEW_PATH_LEGACY_BLS_KEY: u8 = 0x02;

/// Concordium Ledger app instruction byte.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Instruction {
    /// Verify an address on-device.
    VerifyAddress,
    /// Retrieve a public key.
    GetPublicKey,
    /// Sign a simple transfer transaction.
    SignTransfer,
    /// Sign a scheduled transfer transaction.
    SignTransferSchedule,
    /// Sign a credential deployment transaction.
    SignCredentialDeployment,
    /// Export private key material via the legacy Ledger app command.
    ExportPrivateKeyLegacy,
    /// Sign a deploy-module transaction.
    SignDeployModule,
    /// Sign an init-contract transaction.
    SignInitContract,
    /// Sign an update-contract transaction.
    SignUpdateContract,
    /// Sign a transfer-to-public transaction.
    SignTransferToPublic,
    /// Sign a configure-delegation transaction.
    SignConfigureDelegation,
    /// Sign a configure-baker transaction.
    SignConfigureBaker,
    /// Sign a public information for identity-provider payload.
    SignPublicInfoForIp,
    /// Query the app name.
    GetAppName,
    /// Sign an update-credentials transaction.
    SignUpdateCredentials,
    /// Sign a transfer-with-memo transaction.
    SignTransferMemo,
    /// Sign a transfer-with-schedule-and-memo transaction.
    SignTransferScheduleAndMemo,
    /// Sign a register-data transaction.
    SignRegisterData,
    /// Export private key material via the new Ledger app command.
    ExportPrivateKeyNew,
    /// Sign a protocol-level token transaction.
    SignPltTransaction,
    /// Query the Ledger app version.
    GetAppVersion,
}

impl Instruction {
    /// Return the raw instruction byte used in APDU commands.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger::apdu::Instruction;
    /// assert_eq!(Instruction::GetPublicKey.as_u8(), 0x01);
    /// ```
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::VerifyAddress => 0x00,
            Self::GetPublicKey => 0x01,
            Self::SignTransfer => 0x02,
            Self::SignTransferSchedule => 0x03,
            Self::SignCredentialDeployment => 0x04,
            Self::ExportPrivateKeyLegacy => 0x05,
            Self::SignDeployModule => 0x06,
            Self::SignInitContract => 0x07,
            Self::SignUpdateContract => 0x08,
            Self::SignTransferToPublic => 0x12,
            Self::SignConfigureDelegation => 0x17,
            Self::SignConfigureBaker => 0x18,
            Self::SignPublicInfoForIp => 0x20,
            Self::GetAppName => 0x21,
            Self::SignUpdateCredentials => 0x31,
            Self::SignTransferMemo => 0x32,
            Self::SignTransferScheduleAndMemo => 0x34,
            Self::SignRegisterData => 0x35,
            Self::ExportPrivateKeyNew => 0x37,
            Self::SignPltTransaction => 0x38,
            Self::GetAppVersion => 0x40,
        }
    }
}
