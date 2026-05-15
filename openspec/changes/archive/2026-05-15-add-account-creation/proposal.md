## Why

The wallet can issue and store identities, but it cannot yet turn an issued identity into an on-chain Concordium account. This blocks the core post-issuance user journey and leaves the project without a way to derive, deploy, and persist accounts from the identities it already manages.

## What Changes

- Add an `account` command flow for creating a new Concordium account from a stored identity.
- Add account persistence in SQLite using plaintext indexing metadata plus an encrypted account private payload under the owning seed password domain.
- Enforce account creation eligibility rules, including rejecting expired identities during identity selection and before submission.
- Track credential counters in the correct derivation scope: per `(network_genesis_hash, seed_id, ip_identity, identity_index)` tuple.
- Extend identity storage metadata so account creation can determine identity usability, including expiry, without decrypting every stored identity.

## Capabilities

### New Capabilities
- `account-creation`: Create a new Concordium account from a stored issued identity, submit the credential deployment to a node, and persist account metadata plus encrypted private payload data.
- `account-storage`: Store wallet-managed accounts in SQLite with plaintext relational metadata and an extensible encrypted private payload under the owning seed password domain.

### Modified Capabilities
- `identity-storage`: Store additional plaintext identity usability metadata needed for account creation, including identity expiry, while keeping private identity payload data encrypted.

## Impact

- Affected code: `crates/ccd-wallet/src/cli.rs`, new `crates/ccd-wallet/src/commands/account/*`, `crates/ccd-wallet-core/src/store/*`, `crates/ccd-wallet-core/src/wallet.rs`, and README/user-facing command documentation.
- Affected systems: SQLite schema and migrations, seed-domain encryption model, Concordium credential deployment flow, and interactive CLI selection UX.
- Dependencies: existing Concordium Rust SDK credential deployment APIs and current identity issuance/storage flows.
