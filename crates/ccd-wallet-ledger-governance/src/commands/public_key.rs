//! Public-key retrieval command for the Governance Ledger app.

use crate::{
    apdu::{
        ApduCommand, Instruction, NONE, P1_PUBLIC_KEY_CONFIRM, P1_PUBLIC_KEY_NO_CONFIRM,
        P2_SIGNED_PUBLIC_KEY,
    },
    error::{GovernanceLedgerError, Result},
    transport::GovernanceLedgerTransport,
    types::{PublicKeyRequest, PublicKeyResponse},
};

/// Build the public-key APDU command.
///
/// # Arguments
///
/// * `request` - Public-key request and options.
///
/// # Examples
///
/// ```
/// use ccd_wallet_ledger_governance::{DerivationPath, PublicKeyOptions, PublicKeyRequest};
/// use ccd_wallet_ledger_governance::commands::public_key::build_public_key_command;
/// let request = PublicKeyRequest { path: DerivationPath::new([1]).unwrap(), options: PublicKeyOptions::default() };
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

/// Retrieve a public key from the Governance Ledger app.
///
/// # Arguments
///
/// * `transport` - APDU transport used for exchange.
/// * `request` - Public-key request and options.
///
/// # Errors
///
/// Returns an error if APDU exchange fails or the response is malformed.
pub fn get_public_key<T: GovernanceLedgerTransport>(
    transport: &mut T,
    request: &PublicKeyRequest,
) -> Result<PublicKeyResponse> {
    let reply = transport.exchange(&build_public_key_command(request))?;
    parse_public_key_response(reply.data)
}

/// Parse a public-key command response.
///
/// # Arguments
///
/// * `bytes` - Status-stripped response bytes.
///
/// # Errors
///
/// Returns an error if fewer than 32 public-key bytes are present.
///
/// # Examples
///
/// ```
/// use ccd_wallet_ledger_governance::commands::public_key::parse_public_key_response;
/// let response = parse_public_key_response(vec![7; 32]).unwrap();
/// assert_eq!(response.public_key, [7; 32]);
/// ```
pub fn parse_public_key_response(bytes: Vec<u8>) -> Result<PublicKeyResponse> {
    if bytes.len() < 32 {
        return Err(GovernanceLedgerError::InvalidPublicKeyLength {
            actual_len: bytes.len(),
        });
    }
    let public_key = bytes[..32].try_into().expect("slice length checked");
    let signed_public_key = (bytes.len() > 32).then(|| bytes[32..].to_vec());
    Ok(PublicKeyResponse {
        public_key,
        signed_public_key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DerivationPath, MockTransport, PublicKeyOptions};

    #[test]
    fn public_key_command_uses_path_and_options() {
        let request = PublicKeyRequest {
            path: DerivationPath::new([1]).unwrap(),
            options: PublicKeyOptions {
                confirm_on_device: true,
                signed_key: true,
            },
        };
        let command = build_public_key_command(&request);
        assert_eq!(command.ins, Instruction::GetPublicKey.as_u8());
        assert_eq!(command.p1, P1_PUBLIC_KEY_CONFIRM);
        assert_eq!(command.p2, P2_SIGNED_PUBLIC_KEY);
        assert_eq!(command.data, vec![1, 0, 0, 0, 1]);
    }

    #[test]
    fn public_key_round_trip_with_mock_transport() {
        let mut reply = vec![7; 32];
        reply.extend_from_slice(&[0x90, 0x00]);
        let mut transport = MockTransport::new([reply]);
        let request = PublicKeyRequest {
            path: DerivationPath::new([1]).unwrap(),
            options: PublicKeyOptions::default(),
        };
        let response = get_public_key(&mut transport, &request).unwrap();
        assert_eq!(response.public_key, [7; 32]);
        assert_eq!(transport.commands().len(), 1);
    }
}
