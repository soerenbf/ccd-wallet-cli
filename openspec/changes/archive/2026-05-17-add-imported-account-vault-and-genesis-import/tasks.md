## 1. Storage and Migration

- [x] 1.1 Add schema migration for source-aware account metadata, derived-account source metadata, imported-account source metadata, imported account vaults, and imported encrypted payloads.
- [x] 1.2 Migrate existing account rows into the derived-account source representation while preserving network-wide account label uniqueness.
- [x] 1.3 Implement imported account vault store helpers for create-or-find by network genesis hash, password setup, unlock, and encrypted DEK handling.
- [x] 1.4 Implement imported account payload encryption/decryption with AAD binding to account row, network genesis hash, and imported vault context.
- [x] 1.5 Update derived account store helpers to work through the source-aware account model without changing existing derived-account behavior.
- [x] 1.6 Add store tests for migration, cross-source label collisions, imported payload encryption/decryption, seed deletion behavior, and network prune cascades.

## 2. Imported Account Secret Model

- [x] 2.1 Define an internal imported account secret payload that captures account address, account signing keys, credential/encryption material required from genesis JSON, and import source metadata.
- [x] 2.2 Implement genesis account JSON parsing and validation with actionable errors for malformed or incomplete files.
- [x] 2.3 Implement idempotent imported account insertion logic that validates network-wide label uniqueness and stores encrypted imported secret payloads.
- [x] 2.4 Add tests using representative genesis account JSON fixtures for successful parse/import and malformed input failures.

## 3. CLI Import Flow

- [x] 3.1 Extend the account CLI with a single-file genesis import command accepting file path, account label, network selection, `--no-defaults`, and `--non-interactive` as appropriate.
- [x] 3.2 Resolve the import network to a concrete configured network/genesis hash before creating vaults or writing account data.
- [x] 3.3 Require an import label; when omitted in interactive mode, prompt with the JSON filename stem as the suggested default/placeholder.
- [x] 3.4 Validate prompted or explicit labels with normal account-label rules and reject labels already used by any account on the resolved network before writing data.
- [x] 3.5 Create the imported accounts vault implicitly on first import for the resolved network and reuse it for subsequent imports.
- [x] 3.6 Add CLI tests for explicit-label import, prompted-label validation, duplicate-label failure, non-interactive missing-label failure, and directory-path rejection.

## 4. Listing, Rename, and Address Reveal

- [x] 4.1 Update account listing query/rendering so imported accounts appear in normal account lists for matching networks and are visibly marked as imported.
- [x] 4.2 Preserve hidden-address-by-default behavior for imported accounts and unlock imported vaults only when explicit address reveal is requested.
- [x] 4.3 Update account rename fuzzy selection to include imported accounts and searchable imported provenance/network metadata.
- [x] 4.4 Ensure account rename preserves imported source metadata and rejects duplicate labels across all account sources on the network.
- [x] 4.5 Add tests for imported account list rendering, address reveal vault selection, and imported account rename behavior.

## 5. Signing Source Plumbing and Documentation

- [x] 5.1 Add a source-aware account signing material resolver that routes derived accounts to seed derivation and imported accounts to imported vault payloads.
- [x] 5.2 Add non-UI tests proving derived and imported account signing-source resolution choose the correct unlock/material path and reject incomplete imported payloads.
- [x] 5.3 Update README documentation with genesis import examples, label behavior, imported vault behavior, and address privacy notes.
- [x] 5.4 Run `cargo fmt`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace`.
