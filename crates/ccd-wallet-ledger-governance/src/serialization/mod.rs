//! Serialization and chunking helpers for Governance Ledger command sequences.

use crate::{
    apdu::MAX_APDU_PAYLOAD_SIZE,
    error::{GovernanceLedgerError, Result},
    types::DerivationPath,
};

/// Split bytes into APDU-sized chunks.
///
/// # Arguments
///
/// * `bytes` - Payload bytes to split.
///
/// # Examples
///
/// ```
/// use ccd_wallet_ledger_governance::serialization::chunk_payload;
/// assert_eq!(chunk_payload(&vec![1; 256]).len(), 2);
/// ```
pub fn chunk_payload(bytes: &[u8]) -> Vec<Vec<u8>> {
    bytes
        .chunks(MAX_APDU_PAYLOAD_SIZE)
        .map(<[u8]>::to_vec)
        .collect()
}

/// Split bytes into chunks and reject empty payloads.
///
/// # Arguments
///
/// * `field_name` - Field name used in error messages.
/// * `bytes` - Payload bytes to split.
///
/// # Errors
///
/// Returns an error if the payload is empty.
pub fn non_empty_chunks(field_name: &str, bytes: &[u8]) -> Result<Vec<Vec<u8>>> {
    if bytes.is_empty() {
        return Err(GovernanceLedgerError::invalid_request(format!(
            "{field_name} cannot be empty"
        )));
    }
    Ok(chunk_payload(bytes))
}

/// Prefix bytes with a four-byte big-endian length.
///
/// # Arguments
///
/// * `bytes` - Data bytes to prefix.
///
/// # Errors
///
/// Returns an error if `bytes` is longer than `u32::MAX`.
///
/// # Examples
///
/// ```
/// use ccd_wallet_ledger_governance::serialization::length_prefix_u32;
/// assert_eq!(length_prefix_u32(b"abc").unwrap(), vec![0, 0, 0, 3, b'a', b'b', b'c']);
/// ```
pub fn length_prefix_u32(bytes: &[u8]) -> Result<Vec<u8>> {
    let len = u32::try_from(bytes.len()).map_err(|_| {
        GovernanceLedgerError::invalid_request(format!(
            "payload is {} bytes; maximum length-prefixable size is {}",
            bytes.len(),
            u32::MAX
        ))
    })?;
    let mut output = Vec::with_capacity(4 + bytes.len());
    output.extend_from_slice(&len.to_be_bytes());
    output.extend_from_slice(bytes);
    Ok(output)
}

/// Serialize a path and arbitrary trailing bytes.
///
/// # Arguments
///
/// * `path` - Derivation path to prefix.
/// * `suffix` - Data to append after the path.
///
/// # Examples
///
/// ```
/// use ccd_wallet_ledger_governance::{DerivationPath, serialization::path_prefixed};
/// let data = path_prefixed(&DerivationPath::new([1]).unwrap(), &[2]);
/// assert_eq!(data, vec![1, 0, 0, 0, 1, 2]);
/// ```
pub fn path_prefixed(path: &DerivationPath, suffix: &[u8]) -> Vec<u8> {
    let mut bytes = path.to_ledger_bytes();
    bytes.extend_from_slice(suffix);
    bytes
}
