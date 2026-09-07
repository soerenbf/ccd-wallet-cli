# Credentials and identity-related commands

The Concordium Ledger app exposes several identity and credential-related command flows. This crate models those flows as staged low-level request types.

## Public info for identity provider

Use `PublicInfoForIpSigningRequest` with `sign_public_info_for_ip`.

The request stages are:

- `initial`: serialized path, ID credential public value, registration ID, and public-key count
- `keys`: serialized key index, scheme, and verification-key entries
- `threshold`: serialized signature threshold

## Credential deployment

Use `CredentialDeploymentSigningRequest` with `sign_credential_deployment`.

A credential deployment request contains:

- `path`: already serialized Ledger derivation-path bytes
- `credential`: a `CredentialSigningPayload`
- `context`: either `CredentialDeploymentContext::New` with expiry bytes or `CredentialDeploymentContext::Existing` with account-address bytes

`CredentialSigningPayload` groups the staged credential fields sent to the device:

- verification-key count
- one key index / scheme / public key entry per APDU
- account signature threshold / credential ID / identity provider / anonymity-revocation threshold / AR count
- one anonymity-revoker identity and encrypted-share pair per APDU
- credential date fields
- revealed attributes, staged as a tag plus raw UTF-8 byte length followed by raw UTF-8 bytes
- proof length
- proof chunks

The crate owns the APDU choreography but does not construct the cryptographic credential payload itself.

## Update credentials

Use `UpdateCredentialsSigningRequest` with `sign_update_credentials`.

The request includes:

- `header_kind_and_index_length`: path-prefixed account-transaction header, transaction kind, and new-credential count
- `new_credentials`: entries with credential index plus staged credential payload
- `credential_id_count`: serialized removed-credential count
- `credential_ids`: serialized removed credential IDs
- `threshold`: serialized resulting account threshold

The update-credentials command uses a P2 subprotocol. The crate preserves the referenced JavaScript sequencing by setting P2 for initial state, credential index, credential fields, credential ID count, credential IDs, and threshold.

## Identity issuance material

The app 5.4.1 legacy new-path private-key export protocol can export PRFKey and IDCredSec for a selected identity provider and identity index, but it does not export deterministic signature blinding randomness. App 5.5.0+ purpose-based identity credential creation export adds the missing signature blinding randomness and returns all three values as length-prefixed 32-byte fields. Higher-level identity issuance code must reject legacy raw responses when a recoverable Ledger-backed identity requires all three values.

A physical Ledger device running Concordium app `5.6.2` has been used to validate that the purpose-based identity issuance flow works end-to-end for Ledger-backed `identity new`.

## Ledger-backed account creation

Account creation requires Concordium Ledger app `5.6.4` or newer. After explicit approval, higher-level wallet code obtains two transient purpose-based exports: Account Creation returns length-prefixed `32/32/64` fields (PRFKey, IDCredSec, and an extended commitment-randomness root), while Identity Recovery returns IDCredSec and signature blinding randomness. The matching IDCredSec values are required before host-side credential and proof construction proceeds.

The extended 64-byte root supports hardened derivation of attribute randomness. It is not an account signing key: the account signing key remains on the Ledger device, which returns the final credential-deployment signature. Exported and derived secret material must be zeroized and never persisted.

## Design intent

These APIs are deliberately low-level. Higher-level code should prepare canonical Concordium credential bytes, then pass staged byte fields into this crate for Ledger exchange and raw signature retrieval.
