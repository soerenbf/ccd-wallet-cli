//! Governance Ledger APDU status-word handling.

use crate::error::{GovernanceLedgerError, Result};

/// Successful Ledger APDU status word.
pub const STATUS_OK: u16 = 0x9000;

/// User rejected the operation on-device.
pub const STATUS_CONDITIONS_NOT_SATISFIED: u16 = 0x6985;

/// Parsed Ledger APDU status word.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StatusWord(pub u16);

impl StatusWord {
    /// Return `true` if the status word indicates success.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger_governance::apdu::StatusWord;
    /// assert!(StatusWord(0x9000).is_ok());
    /// ```
    pub const fn is_ok(self) -> bool {
        self.0 == STATUS_OK
    }

    /// Convert a status word into a `Result`, preserving successful status words.
    ///
    /// # Errors
    ///
    /// Returns user-decline or status errors for non-success status words.
    pub fn ensure_ok(self) -> Result<()> {
        match self.0 {
            STATUS_OK => Ok(()),
            STATUS_CONDITIONS_NOT_SATISFIED => Err(GovernanceLedgerError::UserDeclined),
            0x6D00 => Err(GovernanceLedgerError::Status {
                status: self.0,
                message: "instruction not supported by the open Governance Ledger app",
            }),
            0x6E00 => Err(GovernanceLedgerError::Status {
                status: self.0,
                message: "invalid Governance Ledger app class byte",
            }),
            0x6700 => Err(GovernanceLedgerError::Status {
                status: self.0,
                message: "invalid APDU length",
            }),
            0x6982 => Err(GovernanceLedgerError::Status {
                status: self.0,
                message: "Ledger security status not satisfied",
            }),
            0x6B01 => Err(GovernanceLedgerError::Status {
                status: self.0,
                message: "Governance Ledger app invalid state",
            }),
            _ => Err(GovernanceLedgerError::Status {
                status: self.0,
                message: "Governance Ledger app command failed",
            }),
        }
    }
}

/// Split a raw APDU reply into response data and status word.
///
/// # Arguments
///
/// * `reply` - Full APDU reply including the trailing two-byte status word.
///
/// # Errors
///
/// Returns an error if the reply is malformed or contains a non-success status word.
///
/// # Examples
///
/// ```
/// use ccd_wallet_ledger_governance::apdu::split_status_word;
/// let data = split_status_word(vec![1, 2, 0x90, 0x00]).unwrap();
/// assert_eq!(data, vec![1, 2]);
/// ```
pub fn split_status_word(reply: Vec<u8>) -> Result<Vec<u8>> {
    if reply.len() < 2 {
        return Err(GovernanceLedgerError::MalformedResponse {
            actual_len: reply.len(),
        });
    }
    let status_index = reply.len() - 2;
    let status = StatusWord(u16::from_be_bytes([
        reply[status_index],
        reply[status_index + 1],
    ]));
    status.ensure_ok()?;
    Ok(reply[..status_index].to_vec())
}
