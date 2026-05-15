## 1. Workspace crate setup

- [x] 1.1 Add `crates/ccd-wallet-identity-provider` as a workspace member.
- [x] 1.2 Create the new crate manifest with the dependencies required by the moved identity provider code.
- [x] 1.3 Create the new crate source layout and public module exports.

## 2. Move identity provider implementation

- [x] 2.1 Move request construction from `ccd-wallet-core/src/identity_provider/mod.rs` into the new crate.
- [x] 2.2 Move identity provider HTTP client helpers into the new crate.
- [x] 2.3 Move callback session code and callback HTML resource into the new crate.
- [x] 2.4 Move identity-provider-specific tests with the implementation.

## 3. Update crate boundaries

- [x] 3.1 Remove `pub mod identity_provider` from `ccd-wallet-core`.
- [x] 3.2 Add `ccd-wallet-identity-provider` as a dependency of the CLI crate.
- [x] 3.3 Update CLI imports and call sites to use the new crate.
- [x] 3.4 Ensure `ccd-wallet-core` still exposes only storage, config, and wallet functionality needed by the new crate and CLI.

## 4. Validation and documentation

- [x] 4.1 Run formatting.
- [x] 4.2 Run workspace clippy with warnings denied.
- [x] 4.3 Run workspace tests.
- [x] 4.4 Update README workspace layout documentation to mention the new crate.
