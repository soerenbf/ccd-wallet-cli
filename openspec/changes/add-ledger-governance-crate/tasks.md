## 1. Workspace and crate foundation

- [x] 1.1 Add a new Governance Ledger Rust crate named `ccd-wallet-ledger-governance` under `crates/` and register it in the Cargo workspace.
- [x] 1.2 Add crate-level documentation describing the low-level Governance Ledger protocol-client scope and explicit non-goals.
- [x] 1.3 Define the initial dependency and feature layout, including default no-SDK usage, an optional SDK integration feature matching `ccd-wallet-ledger`, and immediate concrete HID transport support alongside mock transport support.
- [x] 1.4 Create the public module structure for APDU helpers, errors, transport abstraction, request/response types, serialization helpers, and command modules.

## 2. Core protocol abstractions

- [x] 2.1 Implement a minimal APDU command/reply model and Governance Ledger status/error handling.
- [x] 2.2 Implement the APDU transport trait, a concrete HID transport, and a mock transport that records exact command sequences for tests.
- [x] 2.3 Implement shared helpers for derivation path encoding, update-header encoding, fixed-size integer encoding, length prefixes, and 255-byte payload chunking.
- [x] 2.4 Define raw public-key, signed-public-key, and raw-signature response types with response parsing and user-decline handling.

## 3. Public client and public-key retrieval

- [x] 3.1 Implement the single public Governance Ledger client type over a generic APDU transport.
- [x] 3.2 Implement public-key request/options types for confirmation and signed-public-key variants.
- [x] 3.3 Implement public-key command construction, response parsing, client method, and mock-transport tests.
- [x] 3.4 Document public-key export usage for discovering device-backed governance verify keys that can be placed on chain.

## 4. Fixed-shape governance update signing commands

- [x] 4.1 Define typed request structs for exchange-rate, transaction-fee-distribution, gas-rewards, foundation-account, mint-distribution, baker-stake-threshold, cooldown-parameters, pool-parameters, time-parameters, timeout-parameters, min-block-time, block-energy-limit, finalization-committee-parameters, and validator-score-parameters updates.
- [x] 4.2 Implement APDU builders and client methods for each fixed-shape update family, preserving instruction constants and P1/P2 values from governance app source/tests.
- [x] 4.3 Add mock-transport tests that assert exact APDU command sequences and raw signature parsing for representative fixed-shape update families.

## 5. Staged and chunked governance update signing commands

- [x] 5.1 Define typed request structs for protocol update, add-anonymity-revoker, add-identity-provider, and create-PLT flows.
- [x] 5.2 Implement staged APDU sequencing for protocol updates, including message, specification URL, specification hash, and auxiliary data chunks.
- [x] 5.3 Implement staged APDU sequencing for add-anonymity-revoker and add-identity-provider description fields and key material.
- [x] 5.4 Implement create-PLT staging, initialization-parameter length handling, chunking, and final response parsing.
- [x] 5.5 Add mock-transport tests for staged and chunked command choreography, including multi-chunk payload boundaries.

## 6. Governance key update and authorization signing commands

- [x] 6.1 Define typed request structs for root-key, level-1-key, and level-2-authorization update flows.
- [x] 6.2 Implement higher-level key update APDU sequencing for root and level-1 key update variants.
- [x] 6.3 Implement level-2 authorization update APDU sequencing with authorizations V0/V1/V2 selector handling.
- [x] 6.4 Add tests that cover root, level-1, and level-2 authorization command variants and verify source/test-backed instruction constants.

## 7. Optional SDK integration

- [x] 7.1 Add feature-gated `From` conversions from `concordium-rust-sdk` governance/update types into corresponding crate-local update request/header/value types where mappings are unambiguous, matching the SDK-gated conversion pattern used by `ccd-wallet-ledger`.
- [x] 7.2 Keep ambiguous SDK-to-Ledger mappings as explicit constructors rather than implicit conversions.
- [x] 7.3 Verify the crate compiles and tests pass both with and without the optional SDK feature, similar to `ccd-wallet-ledger`.

## 8. Documentation and protocol coverage validation

- [x] 8.1 Add a crate README that lists supported Governance Ledger command families, feature flags, request model, return model, and non-goals.
- [x] 8.2 Add focused docs for transport/testing, public-key export, fixed-shape updates, staged/chunked updates, governance key updates, and SDK integration.
- [x] 8.3 Cross-check implemented instruction constants and staged command flows against governance app source, governance end-to-end tests, and markdown instruction docs.
- [x] 8.4 Document known source/doc discrepancies and the chosen protocol source-of-truth order.

## 9. Workspace verification

- [x] 9.1 Run formatting for the Rust workspace.
- [x] 9.2 Run crate tests for the new Governance Ledger crate.
- [x] 9.3 Run relevant workspace checks to ensure the new crate does not affect existing wallet CLI behavior.
