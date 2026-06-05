//! Payload chunking helpers for Ledger command sequences.

use crate::{
    apdu::constants::MAX_APDU_PAYLOAD_SIZE,
    error::{LedgerError, Result},
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
/// use ccd_wallet_ledger::serialization::chunk_payload;
/// let chunks = chunk_payload(&vec![1; 256]);
/// assert_eq!(chunks.len(), 2);
/// ```
pub fn chunk_payload(bytes: &[u8]) -> Vec<Vec<u8>> {
    bytes
        .chunks(MAX_APDU_PAYLOAD_SIZE)
        .map(<[u8]>::to_vec)
        .collect()
}

/// Split transaction bytes, prefixing the first chunk with the Ledger derivation path.
///
/// # Arguments
///
/// * `path` - Derivation path to prefix to the first APDU payload.
/// * `bytes` - Transaction bytes to chunk.
///
/// # Errors
///
/// Returns an error if `bytes` is empty.
///
/// # Examples
///
/// ```
/// use ccd_wallet_ledger::{DerivationPath, serialization::chunk_payload_with_path};
/// let path = DerivationPath::new([1]).unwrap();
/// let chunks = chunk_payload_with_path(&path, &[2]).unwrap();
/// assert_eq!(chunks[0], vec![1, 0, 0, 0, 1, 2]);
/// ```
pub fn chunk_payload_with_path(path: &DerivationPath, bytes: &[u8]) -> Result<Vec<Vec<u8>>> {
    if bytes.is_empty() {
        return Err(LedgerError::invalid_request("payload to chunk is empty"));
    }
    let path_bytes = path.to_ledger_bytes();
    if path_bytes.len() >= MAX_APDU_PAYLOAD_SIZE {
        return Err(LedgerError::invalid_request(
            "derivation path leaves no room for payload bytes in first APDU chunk",
        ));
    }
    let first_capacity = MAX_APDU_PAYLOAD_SIZE - path_bytes.len();
    let mut chunks = Vec::new();
    let first_len = bytes.len().min(first_capacity);
    let mut first = Vec::with_capacity(path_bytes.len() + first_len);
    first.extend_from_slice(&path_bytes);
    first.extend_from_slice(&bytes[..first_len]);
    chunks.push(first);

    for chunk in bytes[first_len..].chunks(MAX_APDU_PAYLOAD_SIZE) {
        chunks.push(chunk.to_vec());
    }
    Ok(chunks)
}

/// Prefix bytes with a two-byte big-endian length.
///
/// # Arguments
///
/// * `bytes` - Data bytes to prefix.
///
/// # Errors
///
/// Returns an error if `bytes` is longer than `u16::MAX`.
///
/// # Examples
///
/// ```
/// use ccd_wallet_ledger::serialization::length_prefix_u16;
/// assert_eq!(length_prefix_u16(b"abc").unwrap(), vec![0, 3, b'a', b'b', b'c']);
/// ```
pub fn length_prefix_u16(bytes: &[u8]) -> Result<Vec<u8>> {
    let len = u16::try_from(bytes.len()).map_err(|_| {
        LedgerError::invalid_request(format!(
            "payload is {} bytes; maximum length-prefixable size is {}",
            bytes.len(),
            u16::MAX
        ))
    })?;
    let mut output = Vec::with_capacity(2 + bytes.len());
    output.extend_from_slice(&len.to_be_bytes());
    output.extend_from_slice(bytes);
    Ok(output)
}
