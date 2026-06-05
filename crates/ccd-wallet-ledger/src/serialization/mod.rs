//! Serialization helpers for Concordium Ledger command payloads.

pub mod chunking;

pub use chunking::{chunk_payload, chunk_payload_with_path, length_prefix_u16};
