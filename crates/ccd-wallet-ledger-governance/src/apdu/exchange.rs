//! APDU request and reply types.

use super::constants::LEDGER_CLA;

/// A complete APDU command for the Concordium Governance Ledger app.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApduCommand {
    /// APDU class byte.
    pub cla: u8,
    /// APDU instruction byte.
    pub ins: u8,
    /// First instruction parameter.
    pub p1: u8,
    /// Second instruction parameter.
    pub p2: u8,
    /// APDU payload bytes.
    pub data: Vec<u8>,
}

impl ApduCommand {
    /// Construct a Governance Ledger app command using the default CLA byte.
    ///
    /// # Arguments
    ///
    /// * `ins` - Instruction byte.
    /// * `p1` - First APDU parameter.
    /// * `p2` - Second APDU parameter.
    /// * `data` - APDU payload bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger_governance::apdu::ApduCommand;
    /// let command = ApduCommand::new(0x01, 0x00, 0x00, vec![1, 2]);
    /// assert_eq!(command.cla, ccd_wallet_ledger_governance::apdu::LEDGER_CLA);
    /// ```
    pub fn new(ins: u8, p1: u8, p2: u8, data: Vec<u8>) -> Self {
        Self {
            cla: LEDGER_CLA,
            ins,
            p1,
            p2,
            data,
        }
    }
}

/// Successful APDU reply data with the status word removed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApduReply {
    /// Response payload bytes returned by the Ledger app.
    pub data: Vec<u8>,
}

impl ApduReply {
    /// Construct a reply from status-stripped response bytes.
    ///
    /// # Arguments
    ///
    /// * `data` - Response bytes without APDU status word.
    ///
    /// # Examples
    ///
    /// ```
    /// use ccd_wallet_ledger_governance::apdu::ApduReply;
    /// let reply = ApduReply::new(vec![1]);
    /// assert_eq!(reply.data, vec![1]);
    /// ```
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}
