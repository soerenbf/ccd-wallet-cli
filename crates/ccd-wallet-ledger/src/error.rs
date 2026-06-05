//! Error types for Concordium Ledger app communication.

use std::{error::Error as StdError, fmt};

/// Result type used by the Concordium Ledger client.
pub type Result<T> = std::result::Result<T, LedgerError>;

/// Errors produced while constructing, sending, or parsing Ledger app commands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LedgerError {
    /// A request value could not be represented in the Ledger app protocol.
    InvalidRequest(String),
    /// The APDU transport failed before a Ledger app status word was available.
    Transport(String),
    /// The APDU reply was too short to contain the required status word.
    MalformedResponse { actual_len: usize },
    /// The Ledger app returned a non-success status word.
    Status { status: u16, message: &'static str },
    /// The user declined the operation on the Ledger device.
    UserDeclined,
    /// A signing command returned a response that is not a 64-byte Ed25519 signature.
    InvalidSignatureLength { actual_len: usize },
}

impl LedgerError {
    /// Construct an invalid-request error.
    ///
    /// # Arguments
    ///
    /// * `message` - Human-readable reason the request is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger::LedgerError;
    /// let err = LedgerError::invalid_request("empty transaction");
    /// assert!(err.to_string().contains("empty transaction"));
    /// ```
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest(message.into())
    }

    /// Construct a transport error from a displayable failure.
    ///
    /// # Arguments
    ///
    /// * `message` - Transport failure description.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger::LedgerError;
    /// let err = LedgerError::transport("device unavailable");
    /// assert!(err.to_string().contains("device unavailable"));
    /// ```
    pub fn transport(message: impl Into<String>) -> Self {
        Self::Transport(message.into())
    }
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => write!(f, "invalid Ledger request: {message}"),
            Self::Transport(message) => write!(f, "Ledger transport error: {message}"),
            Self::MalformedResponse { actual_len } => write!(
                f,
                "Ledger response is too short to contain a status word: {actual_len} bytes"
            ),
            Self::Status { status, message } => {
                write!(f, "Ledger app returned status 0x{status:04x}: {message}")
            }
            Self::UserDeclined => f.write_str("Ledger operation declined by user"),
            Self::InvalidSignatureLength { actual_len } => write!(
                f,
                "Ledger signing response must be 64 bytes, got {actual_len} bytes"
            ),
        }
    }
}

impl StdError for LedgerError {}
