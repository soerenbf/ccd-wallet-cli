## Why

The wallet currently assumes signer-capable account material is available locally, which makes it difficult to add Ledger-backed signing without entangling hardware-wallet protocol details with CLI flows, account storage, or transaction submission. We need a dedicated foundation for Concordium Ledger integration now so later CLI and wallet features can build on a well-scoped, reusable protocol client instead of ad hoc device handling.

## What Changes

- Add a new Rust workspace crate in `crates/` for low-level interaction with the Concordium Ledger hardware wallet application.
- Define a typed public API that closely mirrors the Ledger app's APDU capabilities from the referenced JavaScript client while returning raw command outputs such as signatures, public keys, verification statuses, and exported byte payloads rather than signed transactions.
- Translate Concordium request data into the exact APDU sequences required by the device, including sequential multi-call flows and payload chunking where required for transfer, memo, schedule, delegation, baker, register-data, shielded transfer, module deployment, contract init/update, identity/credential, update-credentials, and PLT flows.
- Keep the crate intentionally narrow: no database access, no account selection, no chain submission, no signed transaction assembly, and no CLI UX.
- Make the crate practical to use with Concordium Rust SDK types by defining crate-local request types and providing optional `From` conversions behind a feature-gated SDK dependency.
- Establish a transport abstraction suitable for mock testing and future transport adapters, with HID support able to live behind a feature.

## Capabilities

### New Capabilities
- `ledger-concordium-client`: A low-level Concordium Ledger app protocol client that exposes typed command APIs for the Ledger app operations represented by the referenced JavaScript client, performs Concordium-to-APDU translation and chunking, and returns raw device results suitable for higher-level signing flows.

### Modified Capabilities
- None.

## Impact

- Affected code: new crate under `crates/` (expected `crates/ccd-wallet-ledger`), workspace manifest updates, and crate-level tests/mocks for APDU sequencing and chunking.
- Affected APIs: introduces a new internal/public Rust API surface for Ledger-backed public key retrieval and signing-oriented command execution.
- Dependencies: likely Ledger transport support and optional `concordium-rust-sdk` integration behind a feature for conversion impls.
- Systems unaffected by this change: wallet database structure, encryption model, existing CLI command taxonomy, and node submission flows.
