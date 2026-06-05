//! Non-transaction Concordium Ledger app command helpers.

use crate::{
    apdu::{
        ApduCommand,
        constants::{
            Instruction, NONE, P1_ACCOUNT_CREATION, P1_ACCOUNT_CREDENTIAL_DISCOVERY,
            P1_CREATION_OF_ZK_PROOF, P1_ID_RECOVERY, P1_IDENTITY_CREDENTIAL_CREATION,
            P1_LEGACY_VERIFY_ADDRESS, P1_VERIFY_ADDRESS,
        },
    },
    error::Result,
    transport::LedgerTransport,
    types::{
        ExportPrivateKeyLegacyRequest, ExportPrivateKeyNewRequest, ExportPrivateKeyNewType,
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

/// Build a new private-key export command.
///
/// # Arguments
///
/// * `request` - New export request.
pub fn build_export_private_key_new_command(request: &ExportPrivateKeyNewRequest) -> ApduCommand {
    ApduCommand::new(
        Instruction::ExportPrivateKeyNew.as_u8(),
        export_private_key_new_p1(request.export_type),
        NONE,
        request.payload.clone(),
    )
}

/// Execute new private-key export and return raw device bytes.
///
/// # Arguments
///
/// * `transport` - APDU transport connected to a Ledger device.
/// * `request` - New export request.
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

fn export_private_key_new_p1(export_type: ExportPrivateKeyNewType) -> u8 {
    match export_type {
        ExportPrivateKeyNewType::IdentityCredentialCreation => P1_IDENTITY_CREDENTIAL_CREATION,
        ExportPrivateKeyNewType::AccountCreation => P1_ACCOUNT_CREATION,
        ExportPrivateKeyNewType::IdRecovery => P1_ID_RECOVERY,
        ExportPrivateKeyNewType::AccountCredentialDiscovery => P1_ACCOUNT_CREDENTIAL_DISCOVERY,
        ExportPrivateKeyNewType::CreationOfZkProof => P1_CREATION_OF_ZK_PROOF,
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
    fn export_private_key_new_maps_export_type_to_p1() {
        let command = build_export_private_key_new_command(&ExportPrivateKeyNewRequest {
            export_type: ExportPrivateKeyNewType::AccountCreation,
            payload: vec![1],
        });
        assert_eq!(command.p1, P1_ACCOUNT_CREATION);
    }
}
