//! Non-transaction Concordium Ledger app command helpers.

use crate::{
    apdu::{
        ApduCommand,
        constants::{
            Instruction, NONE, P1_ACCOUNT_CREATION, P1_ACCOUNT_CREDENTIAL_DISCOVERY,
            P1_CREATION_OF_ZK_PROOF, P1_ID_RECOVERY, P1_IDENTITY_CREDENTIAL_CREATION,
            P1_LEGACY_VERIFY_ADDRESS, P1_NEW_PATH_LEGACY_PRF_KEY,
            P1_NEW_PATH_LEGACY_PRF_KEY_AND_ID_CRED_SEC, P1_NEW_PATH_LEGACY_PRF_KEY_RECOVERY,
            P1_VERIFY_ADDRESS, P2_NEW_PATH_LEGACY_BLS_KEY, P2_NEW_PATH_LEGACY_SEED,
        },
    },
    error::{LedgerError, Result},
    transport::LedgerTransport,
    types::{
        AppVersion, ExportPrivateKeyLegacyRequest, ExportPrivateKeyNetwork,
        ExportPrivateKeyNewPathLegacyMode, ExportPrivateKeyNewPathLegacyOutput,
        ExportPrivateKeyNewPathLegacyRequest, ExportPrivateKeyNewRequest, ExportPrivateKeyNewType,
        LegacyVerifyAddressRequest, VerifyAddressRequest,
    },
};

/// Build a current address verification command.
///
/// # Arguments
///
/// * `request` - Current address-verification request.
///
/// # Examples
///
/// ```
/// use ccd_wallet_ledger::{VerifyAddressRequest, commands::device::build_verify_address_command};
/// let command = build_verify_address_command(&VerifyAddressRequest { payload: vec![1] });
/// assert_eq!(command.p1, 0x01);
/// ```
pub fn build_verify_address_command(request: &VerifyAddressRequest) -> ApduCommand {
    ApduCommand::new(
        Instruction::VerifyAddress.as_u8(),
        P1_VERIFY_ADDRESS,
        NONE,
        request.payload.clone(),
    )
}

/// Build a legacy address verification command.
///
/// # Arguments
///
/// * `request` - Legacy address-verification request.
///
/// # Examples
///
/// ```
/// use ccd_wallet_ledger::{LegacyVerifyAddressRequest, commands::device::build_verify_address_legacy_command};
/// let command = build_verify_address_legacy_command(&LegacyVerifyAddressRequest { payload: vec![1] });
/// assert_eq!(command.p1, 0x00);
/// ```
pub fn build_verify_address_legacy_command(request: &LegacyVerifyAddressRequest) -> ApduCommand {
    ApduCommand::new(
        Instruction::VerifyAddress.as_u8(),
        P1_LEGACY_VERIFY_ADDRESS,
        NONE,
        request.payload.clone(),
    )
}

/// Execute current address verification.
///
/// # Arguments
///
/// * `transport` - APDU transport connected to a Ledger device.
/// * `request` - Current address-verification request.
///
/// # Errors
///
/// Returns an error if APDU exchange fails or the Ledger app returns a non-success status word.
pub fn verify_address<T: LedgerTransport>(
    transport: &mut T,
    request: &VerifyAddressRequest,
) -> Result<()> {
    transport.exchange(&build_verify_address_command(request))?;
    Ok(())
}

/// Execute legacy address verification.
///
/// # Arguments
///
/// * `transport` - APDU transport connected to a Ledger device.
/// * `request` - Legacy address-verification request.
///
/// # Errors
///
/// Returns an error if APDU exchange fails or the Ledger app returns a non-success status word.
pub fn verify_address_legacy<T: LedgerTransport>(
    transport: &mut T,
    request: &LegacyVerifyAddressRequest,
) -> Result<()> {
    transport.exchange(&build_verify_address_legacy_command(request))?;
    Ok(())
}

/// Build the app-name query command.
///
/// # Examples
///
/// ```
/// use ccd_wallet_ledger::commands::device::build_get_app_name_command;
/// assert_eq!(build_get_app_name_command().ins, 0x21);
/// ```
pub fn build_get_app_name_command() -> ApduCommand {
    ApduCommand::new(Instruction::GetAppName.as_u8(), NONE, NONE, Vec::new())
}

/// Query the app name from the Ledger app.
///
/// # Arguments
///
/// * `transport` - APDU transport connected to a Ledger device.
///
/// # Errors
///
/// Returns an error if APDU exchange fails or the Ledger app returns a non-success status word.
pub fn get_app_name<T: LedgerTransport>(transport: &mut T) -> Result<Vec<u8>> {
    Ok(transport.exchange(&build_get_app_name_command())?.data)
}

/// Build the app-version query command.
///
/// # Examples
///
/// ```
/// use ccd_wallet_ledger::commands::device::build_get_app_version_command;
/// assert_eq!(build_get_app_version_command().ins, 0x40);
/// ```
pub fn build_get_app_version_command() -> ApduCommand {
    ApduCommand::new(Instruction::GetAppVersion.as_u8(), NONE, NONE, Vec::new())
}

/// Query the Ledger app semantic version.
///
/// # Arguments
/// * `transport` - APDU transport connected to a Ledger device.
///
/// # Errors
/// Returns an error if APDU exchange fails, the app does not support version reporting, or the
/// response is not exactly three bytes.
pub fn get_app_version<T: LedgerTransport>(transport: &mut T) -> Result<AppVersion> {
    parse_app_version_response(transport.exchange(&build_get_app_version_command())?.data)
}

/// Parse an app-version response.
///
/// # Arguments
/// * `bytes` - Status-stripped response bytes.
///
/// # Errors
/// Returns an error if `bytes` is not exactly three bytes.
pub fn parse_app_version_response(bytes: Vec<u8>) -> Result<AppVersion> {
    let [major, minor, patch]: [u8; 3] = bytes.try_into().map_err(|bytes: Vec<u8>| {
        LedgerError::invalid_request(format!(
            "app-version response is {} bytes; expected 3",
            bytes.len()
        ))
    })?;
    Ok(AppVersion::new(major, minor, patch))
}

/// Build a legacy private-key export command.
///
/// # Arguments
///
/// * `request` - Legacy export request.
pub fn build_export_private_key_legacy_command(
    request: &ExportPrivateKeyLegacyRequest,
) -> ApduCommand {
    ApduCommand::new(
        Instruction::ExportPrivateKeyLegacy.as_u8(),
        request.mode,
        request.export_type,
        request.payload.clone(),
    )
}

/// Execute legacy private-key export and return raw device bytes.
///
/// # Arguments
///
/// * `transport` - APDU transport connected to a Ledger device.
/// * `request` - Legacy export request.
///
/// # Errors
///
/// Returns an error if APDU exchange fails or the Ledger app returns a non-success status word.
pub fn export_private_key_legacy<T: LedgerTransport>(
    transport: &mut T,
    request: &ExportPrivateKeyLegacyRequest,
) -> Result<Vec<u8>> {
    Ok(transport
        .exchange(&build_export_private_key_legacy_command(request))?
        .data)
}

/// Build a purpose-based new private-key export command for app 5.5.0 and newer.
///
/// # Arguments
///
/// * `request` - Purpose-based export request.
pub fn build_export_private_key_new_command(request: &ExportPrivateKeyNewRequest) -> ApduCommand {
    ApduCommand::new(
        Instruction::ExportPrivateKeyNew.as_u8(),
        export_private_key_new_p1(request.export_type),
        export_private_key_new_p2(request.network),
        request.payload.clone(),
    )
}

/// Execute purpose-based new private-key export and return raw device bytes.
///
/// # Arguments
///
/// * `transport` - APDU transport connected to a Ledger device.
/// * `request` - Purpose-based export request.
///
/// # Errors
///
/// Returns an error if APDU exchange fails or the Ledger app returns a non-success status word.
pub fn export_private_key_new<T: LedgerTransport>(
    transport: &mut T,
    request: &ExportPrivateKeyNewRequest,
) -> Result<Vec<u8>> {
    Ok(transport
        .exchange(&build_export_private_key_new_command(request))?
        .data)
}

/// Build a legacy new-path private-key export command.
///
/// # Arguments
/// * `request` - Legacy-new-path export mode, output kind, and payload.
pub fn build_export_private_key_new_path_legacy_command(
    request: &ExportPrivateKeyNewPathLegacyRequest,
) -> ApduCommand {
    ApduCommand::new(
        Instruction::ExportPrivateKeyNew.as_u8(),
        export_private_key_new_path_legacy_p1(request.mode),
        export_private_key_new_path_legacy_p2(request.output),
        request.payload.clone(),
    )
}

/// Execute legacy new-path private-key export and return raw device bytes.
///
/// # Arguments
/// * `transport` - APDU transport connected to a Ledger device.
/// * `request` - Legacy-new-path export request.
///
/// # Errors
/// Returns an error if APDU exchange fails or the Ledger app returns a non-success status word.
pub fn export_private_key_new_path_legacy<T: LedgerTransport>(
    transport: &mut T,
    request: &ExportPrivateKeyNewPathLegacyRequest,
) -> Result<Vec<u8>> {
    Ok(transport
        .exchange(&build_export_private_key_new_path_legacy_command(request))?
        .data)
}

fn export_private_key_new_p1(export_type: ExportPrivateKeyNewType) -> u8 {
    match export_type {
        ExportPrivateKeyNewType::IdentityCredentialCreation => P1_IDENTITY_CREDENTIAL_CREATION,
        ExportPrivateKeyNewType::AccountCreation => P1_ACCOUNT_CREATION,
        ExportPrivateKeyNewType::IdRecovery => P1_ID_RECOVERY,
        ExportPrivateKeyNewType::AccountCredentialDiscovery => P1_ACCOUNT_CREDENTIAL_DISCOVERY,
        ExportPrivateKeyNewType::CreationOfZkProof => P1_CREATION_OF_ZK_PROOF,
    }
}

fn export_private_key_new_p2(network: ExportPrivateKeyNetwork) -> u8 {
    match network {
        ExportPrivateKeyNetwork::Mainnet => 0x00,
        ExportPrivateKeyNetwork::Testnet => 0x01,
    }
}

fn export_private_key_new_path_legacy_p1(mode: ExportPrivateKeyNewPathLegacyMode) -> u8 {
    match mode {
        ExportPrivateKeyNewPathLegacyMode::PrfKey => P1_NEW_PATH_LEGACY_PRF_KEY,
        ExportPrivateKeyNewPathLegacyMode::PrfKeyRecovery => P1_NEW_PATH_LEGACY_PRF_KEY_RECOVERY,
        ExportPrivateKeyNewPathLegacyMode::PrfKeyAndIdCredSec => {
            P1_NEW_PATH_LEGACY_PRF_KEY_AND_ID_CRED_SEC
        }
    }
}

fn export_private_key_new_path_legacy_p2(output: ExportPrivateKeyNewPathLegacyOutput) -> u8 {
    match output {
        ExportPrivateKeyNewPathLegacyOutput::Seed => P2_NEW_PATH_LEGACY_SEED,
        ExportPrivateKeyNewPathLegacyOutput::BlsKey => P2_NEW_PATH_LEGACY_BLS_KEY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockTransport;

    fn ok_reply(data: impl Into<Vec<u8>>) -> Vec<u8> {
        let mut data = data.into();
        data.extend_from_slice(&[0x90, 0x00]);
        data
    }

    #[test]
    fn current_and_legacy_verify_address_use_separate_p1_values() {
        assert_eq!(
            build_verify_address_command(&VerifyAddressRequest { payload: vec![] }).p1,
            P1_VERIFY_ADDRESS
        );
        assert_eq!(
            build_verify_address_legacy_command(&LegacyVerifyAddressRequest { payload: vec![] }).p1,
            P1_LEGACY_VERIFY_ADDRESS
        );
    }

    #[test]
    fn get_app_name_returns_raw_bytes() {
        let mut transport = MockTransport::new([ok_reply(b"Concordium".to_vec())]);
        assert_eq!(get_app_name(&mut transport).unwrap(), b"Concordium");
        assert_eq!(transport.commands()[0].ins, Instruction::GetAppName.as_u8());
    }

    #[test]
    fn get_app_version_parses_three_byte_response() {
        let mut transport = MockTransport::new([ok_reply(vec![5, 4, 1])]);
        assert_eq!(
            get_app_version(&mut transport).unwrap(),
            AppVersion::new(5, 4, 1)
        );
        assert_eq!(
            transport.commands()[0].ins,
            Instruction::GetAppVersion.as_u8()
        );
    }

    #[test]
    fn purpose_based_export_private_key_new_uses_p1_and_network_p2() {
        let command = build_export_private_key_new_command(&ExportPrivateKeyNewRequest {
            export_type: ExportPrivateKeyNewType::AccountCreation,
            network: ExportPrivateKeyNetwork::Testnet,
            payload: vec![1],
        });
        assert_eq!(command.p1, P1_ACCOUNT_CREATION);
        assert_eq!(command.p2, 0x01);
    }

    #[test]
    fn legacy_new_path_export_uses_mode_as_p1_and_output_as_p2() {
        let command = build_export_private_key_new_path_legacy_command(
            &ExportPrivateKeyNewPathLegacyRequest {
                mode: ExportPrivateKeyNewPathLegacyMode::PrfKeyAndIdCredSec,
                output: ExportPrivateKeyNewPathLegacyOutput::BlsKey,
                payload: vec![1],
            },
        );
        assert_eq!(command.p1, P1_NEW_PATH_LEGACY_PRF_KEY_AND_ID_CRED_SEC);
        assert_eq!(command.p2, P2_NEW_PATH_LEGACY_BLS_KEY);
    }
}
