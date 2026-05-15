## 1. Schema and storage foundations

- [x] 1.1 Add SQLite schema and migration support for account metadata rows and encrypted account private payload rows
- [x] 1.2 Implement `ccd-wallet-core` account store types for plaintext metadata, structured encrypted `AccountPrivatePayload`, and lifecycle status transitions
- [x] 1.3 Enforce account uniqueness on `(network_genesis_hash, seed_id, ip_identity, identity_index, credential_counter)` and `(network_genesis_hash, label)` in the store layer
- [x] 1.4 Add store helpers for next credential counter allocation within `(network_genesis_hash, seed_id, ip_identity, identity_index)`
- [x] 1.5 Extend identity storage schema and store logic with plaintext identity expiry metadata only
- [x] 1.6 Add identity store support for lazy confirmation of pending identities using stored encrypted issuance state
- [x] 1.7 Add or update store-layer tests covering encrypted account payloads, uniqueness, counter allocation, plaintext expiry metadata, and pending-identity lazy confirmation

## 2. Wallet derivation and Concordium account deployment primitives

- [x] 2.1 Extend `ccd-wallet-core/src/wallet.rs` with account-level derivation helpers compatible with `~/Developer/Concordium/concordium-rust-sdk/concordium-base/rust-src/key_derivation`
- [x] 2.2 Add a small account-creation helper layer that normalizes stored identity payloads, extracts the issued identity object, and builds credential deployment inputs from current chain context
- [x] 2.3 Add tests for account derivation helpers using `key_derivation` crate vectors and any identity payload normalization logic introduced for account creation

## 3. CLI account creation flow

- [x] 3.1 Add `account` CLI command definitions and wire them into `main.rs` and `commands/mod.rs`
- [x] 3.2 Implement `account new` context resolution and interactive/non-interactive identity selection using only usable identities for the resolved seed and network
- [x] 3.3 Implement identity issuance skip-wait support plus lazy confirmation of pending identities during account creation, and expiry prevalidation during selection and again immediately before submission
- [x] 3.4 Implement seed unlock, credential deployment submission, default wait-for-finalization behavior, optional skip-wait flag, pending/finalized lifecycle handling, and success/error messaging for account creation
- [x] 3.5 Add command-level tests covering usable identity filtering, pending-identity lazy confirmation, expiry rejection, skip-wait behavior, non-interactive errors, and successful lifecycle transitions

## 4. Documentation and polish

- [x] 4.1 Update README examples and command documentation for account creation, identity issuance/account creation skip-wait behavior, identity eligibility, and encrypted account persistence behavior
- [x] 4.2 Run formatting, linting, and relevant test suites for the workspace and fix any issues found
