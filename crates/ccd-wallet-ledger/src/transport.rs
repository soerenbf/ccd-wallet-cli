//! Transport abstraction for APDU exchange with a Ledger device.

use crate::{
    apdu::{ApduCommand, ApduReply, split_status_word},
    error::{LedgerError, Result},
};

/// Minimal transport interface required by Concordium Ledger command logic.
///
/// Implementations send a complete APDU command and return the raw device reply including
/// the trailing APDU status word. Command helpers strip and interpret the status word.
///
/// # Errors
///
/// Implementations return [`LedgerError::Transport`] or a transport-specific conversion
/// when the command cannot be sent or no valid reply can be obtained.
///
/// # Examples
///
/// ```
/// use ccd_wallet_ledger::{ApduCommand, LedgerTransport, LedgerError};
///
/// struct Echo;
/// impl LedgerTransport for Echo {
///     fn exchange_raw(&mut self, command: &ApduCommand) -> Result<Vec<u8>, LedgerError> {
///         let mut reply = command.data.clone();
///         reply.extend_from_slice(&[0x90, 0x00]);
///         Ok(reply)
///     }
/// }
/// ```
pub trait LedgerTransport {
    /// Send one raw APDU command and return the raw reply including status word.
    ///
    /// # Arguments
    ///
    /// * `command` - APDU command to send.
    ///
    /// # Errors
    ///
    /// Returns an error if the transport cannot exchange the command with the device.
    fn exchange_raw(&mut self, command: &ApduCommand) -> Result<Vec<u8>>;

    /// Send one APDU command and return response data after status-word validation.
    ///
    /// # Arguments
    ///
    /// * `command` - APDU command to send.
    ///
    /// # Errors
    ///
    /// Returns an error if the transport exchange fails, the reply is malformed, or the
    /// Ledger app returns a non-success status word.
    fn exchange(&mut self, command: &ApduCommand) -> Result<ApduReply> {
        let data = split_status_word(self.exchange_raw(command)?)?;
        Ok(ApduReply::new(data))
    }
}

/// Mock APDU transport for unit tests.
#[derive(Clone, Debug, Default)]
pub struct MockTransport {
    replies: std::collections::VecDeque<Vec<u8>>,
    commands: Vec<ApduCommand>,
}

impl MockTransport {
    /// Construct a mock transport with queued raw replies.
    ///
    /// # Arguments
    ///
    /// * `replies` - Raw APDU replies, each including a two-byte status word.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger::MockTransport;
    /// let transport = MockTransport::new(vec![vec![0x90, 0x00]]);
    /// assert!(transport.commands().is_empty());
    /// ```
    pub fn new(replies: impl IntoIterator<Item = Vec<u8>>) -> Self {
        Self {
            replies: replies.into_iter().collect(),
            commands: Vec::new(),
        }
    }

    /// Return all APDU commands received by the mock transport.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger::MockTransport;
    /// let transport = MockTransport::default();
    /// assert_eq!(transport.commands().len(), 0);
    /// ```
    pub fn commands(&self) -> &[ApduCommand] {
        &self.commands
    }

    /// Consume the transport and return all captured commands.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger::MockTransport;
    /// let transport = MockTransport::default();
    /// let commands = transport.into_commands();
    /// assert!(commands.is_empty());
    /// ```
    pub fn into_commands(self) -> Vec<ApduCommand> {
        self.commands
    }
}

impl LedgerTransport for MockTransport {
    fn exchange_raw(&mut self, command: &ApduCommand) -> Result<Vec<u8>> {
        self.commands.push(command.clone());
        self.replies
            .pop_front()
            .ok_or_else(|| LedgerError::transport("mock transport has no queued reply"))
    }
}
