//! Public-key retrieval command for the Concordium Ledger app.

use crate::{
    apdu::{
        ApduCommand,
        constants::{
            Instruction, NONE, P1_PUBLIC_KEY_CONFIRM, P1_PUBLIC_KEY_NO_CONFIRM,
            P2_SIGNED_PUBLIC_KEY,
        },
    },
    error::{LedgerError, Result},
    transport::LedgerTransport,
    types::{PublicKeyRequest, PublicKeyResponse},
};

/// Build the public-key retrieval APDU command.
///
/// # Arguments
///
/// * `request` - Public-key request containing path and device-display options.
///
/// # Examples
///
/// ```
/// use ccd_wallet_ledger::{DerivationPath, PublicKeyOptions, PublicKeyRequest};
/// use ccd_wallet_ledger::commands::public_key::build_public_key_command;
/// let request = PublicKeyRequest {
///     path: DerivationPath::new([1]).unwrap(),
///     options: PublicKeyOptions::default(),
/// };
/// let command = build_public_key_command(&request);
/// assert_eq!(command.ins, 0x01);
/// ```
pub fn build_public_key_command(request: &PublicKeyRequest) -> ApduCommand {
    let p1 = if request.options.confirm_on_device {
        P1_PUBLIC_KEY_CONFIRM
    } else {
        P1_PUBLIC_KEY_NO_CONFIRM
    };
    let p2 = if request.options.signed_key {
        P2_SIGNED_PUBLIC_KEY
    } else {
        NONE
    };
    ApduCommand::new(
        Instruction::GetPublicKey.as_u8(),
        p1,
        p2,
        request.path.to_ledger_bytes(),
    )
}

/// Execute the public-key retrieval command.
///
/// # Arguments
///
/// * `transport` - APDU transport connected to a Ledger device.
/// * `request` - Public-key request containing path and device-display options.
///
/// # Errors
///
/// Returns an error if APDU exchange fails or the response does not contain a 32-byte key.
///
/// # Examples
///
/// ```
/// use ccd_wallet_ledger::{DerivationPath, MockTransport, PublicKeyOptions, PublicKeyRequest};
/// use ccd_wallet_ledger::commands::public_key::get_public_key;
/// let mut reply = vec![7; 32];
/// reply.extend_from_slice(&[0x90, 0x00]);
/// let mut transport = MockTransport::new([reply]);
/// let request = PublicKeyRequest {
///     path: DerivationPath::new([1]).unwrap(),
///     options: PublicKeyOptions::default(),
/// };
/// let response = get_public_key(&mut transport, &request).unwrap();
/// assert_eq!(response.public_key, [7; 32]);
/// ```
pub fn get_public_key<T: LedgerTransport>(
    transport: &mut T,
    request: &PublicKeyRequest,
) -> Result<PublicKeyResponse> {
    let reply = transport.exchange(&build_public_key_command(request))?.data;
    parse_public_key_response(reply, request.options.signed_key)
}

/// Parse public-key response bytes returned by the Ledger app.
///
/// # Arguments
///
/// * `reply` - Status-stripped response bytes.
/// * `signed_key` - Whether a signed public key was requested.
///
/// # Errors
///
/// Returns an error if fewer than 32 bytes are returned.
///
/// # Examples
///
/// ```
/// use ccd_wallet_ledger::commands::public_key::parse_public_key_response;
/// let response = parse_public_key_response(vec![1; 32], false).unwrap();
/// assert_eq!(response.signed_public_key, None);
/// ```
pub fn parse_public_key_response(reply: Vec<u8>, signed_key: bool) -> Result<PublicKeyResponse> {
    if reply.len() < 32 {
        return Err(LedgerError::invalid_request(format!(
            "public-key response is {} bytes; expected at least 32",
            reply.len()
        )));
    }
    let mut public_key = [0u8; 32];
    public_key.copy_from_slice(&reply[..32]);
    let signed_public_key = if signed_key {
        Some(reply[32..].to_vec())
    } else {
        None
    };
    Ok(PublicKeyResponse {
        public_key,
        signed_public_key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DerivationPath, PublicKeyOptions};

    #[test]
    fn public_key_confirmation_p1_matches_ledger_app_protocol() {
        let path = DerivationPath::new([1]).unwrap();
        let no_confirm = build_public_key_command(&PublicKeyRequest {
            path: path.clone(),
            options: PublicKeyOptions {
                confirm_on_device: false,
                signed_key: false,
            },
        });
        let confirm = build_public_key_command(&PublicKeyRequest {
            path,
            options: PublicKeyOptions {
                confirm_on_device: true,
                signed_key: false,
            },
        });

        assert_eq!(no_confirm.p1, P1_PUBLIC_KEY_NO_CONFIRM);
        assert_eq!(no_confirm.p1, 0x01);
        assert_eq!(confirm.p1, P1_PUBLIC_KEY_CONFIRM);
        assert_eq!(confirm.p1, 0x00);
    }
}
