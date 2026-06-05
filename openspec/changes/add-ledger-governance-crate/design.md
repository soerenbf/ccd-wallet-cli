## Context

The repository already has an isolated low-level `ccd-wallet-ledger` crate for the regular Concordium Ledger app. That crate intentionally owns APDU construction, command sequencing, chunking, transport abstraction, and raw device response parsing while avoiding wallet storage, CLI UX, transaction assembly, and node submission.

Governance update submission currently lives in `crates/ccd-wallet/src/commands/governance.rs` and signs updates with locally available `UpdateKeyPair` material from the governance key vault. This does not fit operators who want governance keys to remain on a Ledger device running the separate Concordium Governance app.

The Concordium Ledger app repository contains a distinct `governance-app` application for retrieving keys and signing chain update transactions. Its command surface is update-family-specific rather than a generic "sign this hash" interface. Important sources of truth are the governance app source and end-to-end tests, with markdown instruction docs as supporting references because some docs appear to drift from source/test behavior.

## Goals / Non-Goals

**Goals:**
- Provide a low-level Rust protocol client for the Concordium Governance Ledger app.
- Expose one public client type with typed methods for public-key retrieval and the full Governance Ledger app signing command surface.
- Own APDU request construction, update-family-specific sequencing, P1/P2 selection, chunking, and response parsing.
- Return raw device outputs such as governance public keys and raw signatures.
- Define crate-local request/response types that can stay stable even if `concordium-rust-sdk` types evolve.
- Provide optional SDK conversions/helpers for common governance update payloads where they remain thin and unambiguous, using the same feature-gated SDK pattern as `ccd-wallet-ledger`.
- Keep command logic transport-agnostic and testable with a mock transport.

**Non-Goals:**
- Wiring Ledger-backed governance signing into the CLI.
- Changing governance key vault storage, encryption, import, listing, or removal behavior.
- Selecting governance signers or deriving authorization thresholds from chain state.
- Assembling signed `UpdateInstruction`/`BlockItem` values from returned signatures.
- Submitting governance updates to a node or waiting for finalization.
- Supporting blind signing of unknown serialized governance payloads unless the Governance Ledger app exposes a suitable generic protocol capability.
- Sharing abstractions with `ccd-wallet-ledger` before clear duplication justifies it.

## Decisions

### 1. Add a sibling crate instead of extending `ccd-wallet-ledger`

**Decision:** Add a separate Governance Ledger crate under `crates/` rather than adding governance commands to the regular Ledger app crate.

**Rationale:** The regular Concordium app and Governance app are separate device apps with different command surfaces and operator use cases. A sibling crate keeps each protocol client focused and avoids a single crate accumulating unrelated device-app concepts.

**Alternatives considered:**
- Extending `ccd-wallet-ledger`. Rejected because it couples two device apps and makes the existing crate less focused.
- Creating a shared generic Ledger protocol crate first. Rejected as premature; duplicated APDU/transport primitives are acceptable until a concrete need for sharing emerges.

### 2. Expose one public Governance Ledger client

**Decision:** The public API will center on one client type, such as `GovernanceLedgerApp<T>`, with command-specific methods.

**Rationale:** Consumers should have one obvious entry point, while internal modules can still remain command-family-oriented. This mirrors the current regular Ledger crate style and keeps use simple.

**Alternatives considered:**
- Separate public clients per update family. Rejected because it adds ceremony without improving the low-level protocol boundary.

### 3. Keep the API APDU-close and command-specific

**Decision:** Public signing methods will map to Governance Ledger command families, for example protocol update, exchange-rate update, higher-level key update, authorizations update, add identity provider, add anonymity revoker, and create PLT.

**Rationale:** The Governance Ledger app displays and signs typed update flows. It does not appear to expose a generic blind-sign or sign-update-hash command. Preserving command distinctions keeps request validation, APDU choreography, and tests aligned with device behavior.

**Alternatives considered:**
- A generic `sign_update_payload` API. Rejected because it would hide meaningful protocol differences and imply blind-signing support the device app does not appear to provide.
- A raw public APDU exchange API. Rejected because it would push Ledger-specific staging and Concordium serialization burden to callers.

### 4. Return raw signatures and public keys, not signed updates

**Decision:** Signing methods return raw signatures. Public-key methods return raw public-key responses. The crate does not wrap signatures into update signature maps or construct signed update instructions.

**Rationale:** Mapping signatures to governance key indices depends on chain parameters, selected signer sets, authorization families, and threshold logic. Those are higher-level wallet concerns and should not leak into the protocol client.

**Alternatives considered:**
- Returning signed `UpdateInstruction` values. Rejected because it would mix APDU protocol handling with chain-context and submission-oriented orchestration.

### 5. Public-key export is first-class

**Decision:** Public-key retrieval for governance derivation paths is part of the initial crate scope.

**Rationale:** Operators need device-derived governance public keys to update governance key sets on chain and to match device-backed keys against chain authorization state. Public-key export is therefore core to the governance key lifecycle, not an optional convenience.

### 6. Use crate-local request types with feature-gated SDK conversions, matching `ccd-wallet-ledger`

**Decision:** The crate defines its own public request types and enables selected `concordium-rust-sdk` conversions behind an optional feature, following the same SDK-optional pattern already established by `ccd-wallet-ledger`.

**Rationale:** Crate-local types keep the stable API shaped around Ledger protocol needs. Feature-gated conversions make the crate ergonomic for this repository without coupling all users to the SDK, and consistency with `ccd-wallet-ledger` keeps the two Ledger crates aligned in dependency and API expectations.

**Alternatives considered:**
- Accepting SDK types directly everywhere. Rejected because SDK shape and Ledger protocol shape are not identical.
- Accepting only serialized bytes. Rejected because many governance app flows require field-level staging and device review semantics.

### 7. Treat source and tests as primary protocol references

**Decision:** Instruction constants, P1/P2 values, and staging rules should be validated against the governance app source and end-to-end tests, using markdown docs as supporting references.

**Rationale:** The governance markdown docs are useful but show signs of drift from source/test behavior. Golden and mock tests should preserve the exact protocol behavior implemented by the device app.

## Risks / Trade-offs

- **Governance app protocol drift** → Keep instruction constants explicit, document assumptions, and add mock/golden tests per command family.
- **Markdown docs disagree with source/tests** → Prefer source and end-to-end tests, and record discrepancies in crate docs/tests where relevant.
- **Full command-surface parity is larger than a narrow integration slice** → Keep the crate isolated and test-driven so the broader surface does not affect CLI behavior until a follow-up integration change.
- **SDK conversions become complex** → Keep conversions optional and thin; prefer explicit crate-local request constructors for ambiguous flows.
- **No Ledger blind-signing for unknown payloads** → Preserve current local-key blind-sign behavior outside this crate and document that Ledger-backed governance signing is limited to typed device-supported flows.
- **Future duplication with `ccd-wallet-ledger`** → Accept duplication initially; extract shared helpers only after both crates reveal stable common patterns.

## Migration Plan

- Add the new crate and workspace manifest entry without wiring it into existing CLI commands.
- Implement the transport abstraction, APDU helpers, typed request/response model, command methods, tests, and crate documentation as an isolated addition.
- Leave existing governance key vault and local-key governance update flows unchanged.
- Integrate Ledger-backed governance signing in a later change once the low-level protocol crate is stable.
- Rollback is straightforward: remove the new crate and workspace entry because no existing runtime behavior depends on it.

## Open Questions

- What should the final crate name be? Candidate: `ccd-wallet-ledger-governance`.
- Which SDK conversions should ship initially versus waiting for higher-level integration work to prove demand?
- Should concrete HID transport support ship immediately, or should the first version stop at abstract/mock transport with HID reserved behind a feature?
