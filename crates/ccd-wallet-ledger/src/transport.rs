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

#[cfg(feature = "hid")]
mod hid {
    use super::*;
    use hidapi::{HidApi, HidDevice};

    const LEDGER_VENDOR_ID: u16 = 0x2c97;
    const LEDGER_USAGE_PAGE: u16 = 0xffa0;
    const CHANNEL: u16 = 0x0101;
    const TAG_APDU: u8 = 0x05;
    const PACKET_SIZE: usize = 64;

    /// HID transport for exchanging APDU commands with a Ledger device.
    ///
    /// The transport implements the Ledger HID APDU framing used by Ledger apps and opens the
    /// first matching Ledger HID interface by default.
    pub struct HidTransport {
        device: HidDevice,
    }

    impl HidTransport {
        /// Open the first Ledger HID device that exposes the Ledger APDU usage page.
        ///
        /// # Errors
        ///
        /// Returns a transport error if HID enumeration fails or no Ledger device is found.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use ccd_wallet_ledger::HidTransport;
        /// let _transport = HidTransport::open_first()?;
        /// # ccd_wallet_ledger::Result::Ok(())
        /// ```
        pub fn open_first() -> Result<Self> {
            let api = HidApi::new().map_err(|err| LedgerError::transport(err.to_string()))?;
            let device_info = api
                .device_list()
                .find(|device| {
                    device.vendor_id() == LEDGER_VENDOR_ID
                        && device.usage_page() == LEDGER_USAGE_PAGE
                })
                .ok_or_else(|| LedgerError::transport("no Ledger HID device found"))?;
            let device = device_info
                .open_device(&api)
                .map_err(|err| LedgerError::transport(err.to_string()))?;
            Ok(Self { device })
        }

        /// Construct a HID transport from an already-open HID device.
        ///
        /// # Arguments
        /// * `device` - Open HID device for Ledger APDU communication.
        pub fn new(device: HidDevice) -> Self {
            Self { device }
        }
    }

    impl LedgerTransport for HidTransport {
        fn exchange_raw(&mut self, command: &ApduCommand) -> Result<Vec<u8>> {
            let apdu = encode_apdu(command)?;
            write_framed(&self.device, &apdu)?;
            read_framed(&self.device)
        }
    }

    fn encode_apdu(command: &ApduCommand) -> Result<Vec<u8>> {
        let lc = u8::try_from(command.data.len()).map_err(|_| {
            LedgerError::invalid_request(format!(
                "APDU data is {} bytes; maximum is 255",
                command.data.len()
            ))
        })?;
        let mut apdu = Vec::with_capacity(5 + command.data.len());
        apdu.extend_from_slice(&[command.cla, command.ins, command.p1, command.p2, lc]);
        apdu.extend_from_slice(&command.data);
        Ok(apdu)
    }

    fn write_framed(device: &HidDevice, apdu: &[u8]) -> Result<()> {
        let apdu_len = u16::try_from(apdu.len()).map_err(|_| {
            LedgerError::invalid_request(format!("encoded APDU is too large: {} bytes", apdu.len()))
        })?;
        let mut offset = 0;
        let mut sequence = 0u16;
        while offset < apdu.len() || sequence == 0 {
            let mut packet = [0u8; PACKET_SIZE];
            packet[0..2].copy_from_slice(&CHANNEL.to_be_bytes());
            packet[2] = TAG_APDU;
            packet[3..5].copy_from_slice(&sequence.to_be_bytes());
            let header_len = if sequence == 0 {
                packet[5..7].copy_from_slice(&apdu_len.to_be_bytes());
                7
            } else {
                5
            };
            let capacity = PACKET_SIZE - header_len;
            let count = (apdu.len() - offset).min(capacity);
            packet[header_len..header_len + count].copy_from_slice(&apdu[offset..offset + count]);
            device
                .write(&packet)
                .map_err(|err| LedgerError::transport(err.to_string()))?;
            offset += count;
            sequence = sequence
                .checked_add(1)
                .ok_or_else(|| LedgerError::transport("HID sequence overflow"))?;
        }
        Ok(())
    }

    fn read_framed(device: &HidDevice) -> Result<Vec<u8>> {
        let mut sequence = 0u16;
        let mut output = Vec::new();
        let mut expected_len = None;
        loop {
            let mut packet = [0u8; PACKET_SIZE];
            let size = device
                .read(&mut packet)
                .map_err(|err| LedgerError::transport(err.to_string()))?;
            if (sequence == 0 && size < 7) || size < 5 {
                return Err(LedgerError::transport("short Ledger HID packet"));
            }
            if u16::from_be_bytes([packet[0], packet[1]]) != CHANNEL || packet[2] != TAG_APDU {
                return Err(LedgerError::transport(
                    "unexpected Ledger HID channel or tag",
                ));
            }
            if u16::from_be_bytes([packet[3], packet[4]]) != sequence {
                return Err(LedgerError::transport(
                    "unexpected Ledger HID sequence number",
                ));
            }
            let header_len = if sequence == 0 {
                let len = usize::from(u16::from_be_bytes([packet[5], packet[6]]));
                expected_len = Some(len);
                output.reserve(len);
                7
            } else {
                5
            };
            let expected_len = expected_len
                .ok_or_else(|| LedgerError::transport("missing Ledger HID response length"))?;
            let remaining = expected_len.saturating_sub(output.len());
            let count = remaining.min(size.saturating_sub(header_len));
            output.extend_from_slice(&packet[header_len..header_len + count]);
            if output.len() >= expected_len {
                output.truncate(expected_len);
                return Ok(output);
            }
            sequence = sequence
                .checked_add(1)
                .ok_or_else(|| LedgerError::transport("HID sequence overflow"))?;
        }
    }

    pub use HidTransport as Transport;
}

#[cfg(feature = "hid")]
pub use hid::Transport as HidTransport;
