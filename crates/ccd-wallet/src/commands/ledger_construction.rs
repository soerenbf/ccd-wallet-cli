#![allow(dead_code)]

//! Ledger identity/account construction bridge.
//!
//! This module is the explicit boundary between wallet orchestration and the
//! low-level Concordium Ledger APDU client. It prevents Ledger-backed flows from
//! silently falling back to seed-derived construction or private-key export when
//! a high-level identity/account flow is not yet representable end-to-end.

use anyhow::{Result, bail};
use ccd_wallet_core::store::signer_owners::LedgerOwnerDetailsRecord;

/// Inputs that identify a Ledger-backed identity issuance construction request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LedgerIdentityIssuanceInput<'a> {
    /// Enrolled Ledger owner details matched against the connected device.
    pub owner_details: &'a LedgerOwnerDetailsRecord,
    /// Network genesis hash for the identity issuance flow.
    pub network_genesis_hash: &'a str,
    /// Identity provider index.
    pub ip_identity: u32,
    /// Identity index under the Ledger key source.
    pub identity_index: u32,
}

/// Inputs that identify a Ledger-backed credential deployment construction request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LedgerCredentialDeploymentInput<'a> {
    /// Enrolled Ledger owner details matched against the connected device.
    pub owner_details: &'a LedgerOwnerDetailsRecord,
    /// Network genesis hash for the account creation flow.
    pub network_genesis_hash: &'a str,
    /// Identity provider index.
    pub ip_identity: u32,
    /// Identity index under the Ledger key source.
    pub identity_index: u32,
    /// Credential counter for the derived account.
    pub credential_counter: u8,
}

/// Prepare a Ledger-backed identity issuance request.
///
/// # Arguments
/// * `input` - Ledger owner and identity issuance coordinates.
///
/// # Errors
/// Returns an unsupported-flow error until the wallet has an end-to-end
/// implementation that can construct the identity request through supported
/// Ledger app operations without implicit private-key export.
///
/// # Examples
///
/// ```ignore
/// let request = ledger_construction::construct_identity_issuance(input)?;
/// ```
pub(crate) fn construct_identity_issuance(input: LedgerIdentityIssuanceInput<'_>) -> Result<()> {
    bail!(
        "Ledger identity issuance is not yet supported for key source '{}' on network '{}' (provider {}, identity index {}); no local identity state was written",
        input.owner_details.fingerprint,
        input.network_genesis_hash,
        input.ip_identity,
        input.identity_index,
    )
}

/// Prepare a Ledger-backed account credential deployment.
///
/// # Arguments
/// * `input` - Ledger owner and account derivation coordinates.
///
/// # Errors
/// Returns an unsupported-flow error until the wallet has an end-to-end
/// implementation that can construct and sign credential deployments through
/// supported Ledger app operations without implicit private-key export.
///
/// # Examples
///
/// ```ignore
/// let deployment = ledger_construction::construct_credential_deployment(input)?;
/// ```
pub(crate) fn construct_credential_deployment(
    input: LedgerCredentialDeploymentInput<'_>,
) -> Result<()> {
    bail!(
        "Ledger account creation is not yet supported for key source '{}' on network '{}' (provider {}, identity index {}, credential {}); no transaction was submitted",
        input.owner_details.fingerprint,
        input.network_genesis_hash,
        input.ip_identity,
        input.identity_index,
        input.credential_counter,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn details() -> LedgerOwnerDetailsRecord {
        LedgerOwnerDetailsRecord {
            signer_owner_id: "owner".to_owned(),
            canonical_public_key: vec![7; 32],
            fingerprint: "07070707".to_owned(),
            enrollment_path: "m/44'/919'/0'/0'/0'".to_owned(),
            app_name: Some("Concordium".to_owned()),
            created_at: 1,
            updated_at: 1,
            last_seen_at: None,
        }
    }

    #[test]
    fn identity_construction_fails_before_storage_when_unsupported() {
        let details = details();
        let err = construct_identity_issuance(LedgerIdentityIssuanceInput {
            owner_details: &details,
            network_genesis_hash: "net",
            ip_identity: 7,
            identity_index: 0,
        })
        .unwrap_err();
        assert!(err.to_string().contains("not yet supported"));
        assert!(
            err.to_string()
                .contains("no local identity state was written")
        );
    }

    #[test]
    fn account_construction_fails_before_submission_when_unsupported() {
        let details = details();
        let err = construct_credential_deployment(LedgerCredentialDeploymentInput {
            owner_details: &details,
            network_genesis_hash: "net",
            ip_identity: 7,
            identity_index: 0,
            credential_counter: 0,
        })
        .unwrap_err();
        assert!(err.to_string().contains("not yet supported"));
        assert!(err.to_string().contains("no transaction was submitted"));
    }
}
