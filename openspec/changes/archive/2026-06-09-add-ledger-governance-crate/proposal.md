## Why

The wallet's governance update flow currently assumes signer-capable governance key material is available locally, which prevents operators from signing governance chain updates with keys held on a Ledger device running the Concordium Governance app. We need a dedicated low-level Rust integration library so Ledger governance protocol details stay reusable and isolated from CLI orchestration, governance key storage, update assembly, and node submission.

## What Changes

- Add a new Rust workspace crate under `crates/` named `ccd-wallet-ledger-governance` for low-level interaction with the Concordium Governance Ledger app.
- Define a single typed public client for the Governance Ledger app that exposes the full documented/source-backed governance app command surface, including public-key export and governance update signing commands.
- Translate governance-oriented request data into the exact APDU sequences required by the device, including staged multi-call flows, update-family-specific P1/P2 choreography, authorizations version selection, and payload chunking where required.
- Return raw device outputs such as governance public keys and raw signatures rather than assembling signed update instructions, submitting updates, or waiting for finalization.
- Keep the crate intentionally narrow: no database access, no governance key selection, no CLI UX, no signed block-item assembly, no node submission, and no blind signing of unknown serialized payloads unless the Governance Ledger app exposes such a protocol capability.
- Make the crate practical to use with Concordium Rust SDK governance/update types by defining crate-local request types and providing optional feature-gated conversions, following the same SDK-optional pattern used by `ccd-wallet-ledger`.
- Establish a transport abstraction suitable for mock testing while shipping concrete HID transport support in the initial version.

## Capabilities

### New Capabilities
- `ledger-governance-client`: A low-level Concordium Governance Ledger app protocol client that exposes typed public-key and governance update signing APIs, performs governance-to-APDU translation and chunking, and returns raw device outputs suitable for higher-level governance update flows.

### Modified Capabilities
- None.

## Impact

- Affected code: new Rust crate under `crates/`, workspace manifest updates, and crate-level tests/docs for Governance Ledger APDU sequencing and chunking.
- Affected APIs: introduces a new reusable Rust API surface for Governance Ledger public-key retrieval and update-signing command execution.
- Dependencies: optional `concordium-rust-sdk` integration behind a feature, matching the SDK-gated conversion approach already used by `ccd-wallet-ledger`; initial concrete HID transport support is included alongside mock transport support.
- Systems unaffected by this change: wallet database schema, governance key vault encryption, current CLI command taxonomy, governance update submission/finalization flow, and existing local governance key signing behavior.
