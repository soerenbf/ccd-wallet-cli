## 1. Workspace and crate foundation

- [x] 1.1 Add a new `crates/ccd-wallet-ledger` crate to the Rust workspace with crate-level documentation describing its low-level protocol-client scope.
- [x] 1.2 Add the initial dependency and feature layout, including a minimal default configuration, an optional transport feature, and an optional `concordium-rust-sdk` integration feature.
- [x] 1.3 Define the base public module structure for errors, transport abstraction, request/response types, APDU helpers, and command modules.

## 2. Core protocol abstractions

- [x] 2.1 Implement the transport abstraction for APDU exchange and provide a mock/fake transport for tests.
- [x] 2.2 Define common APDU constants, status/error handling, and response parsing helpers shared across command modules.
- [x] 2.3 Implement shared chunking and sequential exchange helpers for command flows that span multiple APDU calls.
- [x] 2.4 Add typed path/request primitives needed by the first supported Ledger command families.

## 3. Command-oriented Ledger client surface

- [x] 3.1 Implement the low-level client entry type and command-specific public key retrieval API returning raw device outputs.
- [x] 3.2 Implement the first signing-oriented command modules using crate-local request types and raw signature outputs.
- [x] 3.3 Implement command-specific APDU sequencing for flows that require staged request uploads or chunked payloads.
- [x] 3.4 Ensure the public API remains command-oriented and does not assemble signed Concordium transactions.

## 4. SDK integration ergonomics

- [x] 4.1 Define crate-local public request types for the supported command families and document their fields.
- [x] 4.2 Add feature-gated conversions from supported `concordium-rust-sdk` types into the crate-local request types.
- [x] 4.3 Verify the crate remains usable without enabling the SDK feature.

## 5. Test coverage and documentation

- [x] 5.1 Add unit tests for APDU request construction, chunk boundaries, and multi-stage command choreography.
- [x] 5.2 Add tests using the mock transport to verify exact exchange sequences and parsed outputs without hardware.
- [x] 5.3 Document the supported command surface, feature flags, low-level return types, and explicit non-goals such as transaction assembly and submission.

## 6. Full referenced Ledger command surface

- [x] 6.1 Add low-level request/response types for address verification, app-name lookup, memo/schedule transfers, configure-baker, register-data, transfer-to-public, public-info-for-IP, credential deployment, update-credentials, and private-key export flows.
- [x] 6.2 Implement APDU command builders and client methods for all missing referenced command families, preserving command-specific P1/P2 sequencing and chunking rules.
- [x] 6.3 Add mock-transport tests covering representative APDU sequences for the newly supported command families.
- [x] 6.4 Update crate documentation to list the expanded supported command surface.

## 7. Crate guide documentation

- [x] 7.1 Add a crate README and focused `docs/` pages for transport, public keys/address verification, transaction signing, contracts/modules, credentials/identity, private-key export, and testing.
- [x] 7.2 Split legacy address verification into a dedicated request/function instead of a flag on the current address-verification request.
