## Why

The identity provider issuance code has grown beyond a small helper module inside `ccd-wallet-core`. Extracting it into its own workspace crate makes the boundary explicit, keeps `ccd-wallet-core` focused on wallet/storage concerns, and prepares the code for reuse by future wallet frontends or callback transports.

## What Changes

- Add a new workspace crate for identity provider issuance functionality.
- Move the current `identity_provider` module out of `ccd-wallet-core` into the new crate.
- Update `ccd-wallet-core` and `ccd-wallet` dependencies/imports to use the new crate.
- Keep identity issuance behavior unchanged.
- Keep the new crate internal to the workspace for now; no publishing/API stability guarantee is introduced.
- Preserve existing tests and move identity-provider-specific tests with the code.

## Capabilities

### New Capabilities
- `identity-provider-crate`: The identity provider issuance implementation is provided by a dedicated workspace crate with a clear public API consumed by the CLI/core crates.

### Modified Capabilities
- `identity-provider-client`: The implementation location changes from `ccd-wallet-core::identity_provider` to the dedicated crate, while HTTP protocol behavior remains unchanged.
- `identity-issuance`: Imports and orchestration are updated to consume the dedicated crate, while CLI behavior remains unchanged.

## Impact

- Workspace `Cargo.toml` gains a new crate member.
- New crate, likely `crates/ccd-wallet-identity-provider`, owns request construction, HTTP client helpers, and callback session logic.
- `ccd-wallet-core` no longer exports `identity_provider`.
- `ccd-wallet` depends on the new crate directly for identity issuance orchestration.
- Tests for callback, HTTP client, and request construction move with the crate.
