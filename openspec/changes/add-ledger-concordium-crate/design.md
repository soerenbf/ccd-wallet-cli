## Context

The repository currently has no dedicated hardware-wallet integration layer. Existing signing flows assume signer-capable account material can be resolved locally and then passed into the Concordium Rust SDK's transaction helpers. That model is a poor fit for Ledger because the Concordium Ledger app exposes a command-oriented APDU protocol with command-specific sequencing rules, chunking rules, and multi-step request flows.

The proposed crate is intentionally foundational. It should be reusable by future CLI commands and other crates without pulling in database access, chain submission, or interactive UX. At the same time, it must remain practical to use with Concordium domain values, especially types from `concordium-rust-sdk`, because higher-level code in this repository already uses those types extensively.

Constraints and context:
- The crate belongs in the Rust workspace as a new crate under `crates/`.
- The public API should stay close to the Concordium Ledger app's APDU capabilities rather than hiding them behind a high-level "wallet" abstraction.
- The crate must return raw command outputs such as signatures and public keys, not signed transactions.
- The crate should be testable without physical hardware.
- Future higher-level APIs may live either in this crate or in a separate crate, so the foundation should not prematurely commit to orchestration concerns.

## Goals / Non-Goals

**Goals:**
- Provide a low-level Rust client for the Concordium Ledger app.
- Expose typed command methods that closely mirror the Concordium Ledger operations represented by the referenced JavaScript client while accepting Concordium-oriented request data.
- Own APDU request construction, sequential multi-call choreography, and payload chunking.
- Return raw command outputs such as signatures and public keys.
- Define crate-local request and response types that can remain stable even if upstream SDK types evolve.
- Make integration with `concordium-rust-sdk` ergonomic via an optional feature that enables conversion impls from SDK types.
- Isolate transport behind an abstraction suitable for mocks and alternate transports.

**Non-Goals:**
- Building signed Concordium transactions or block items.
- Submitting transactions to a node or waiting for finalization.
- Device discovery UX, account selection, password prompts, or database access.
- Modeling wallet state, account provenance, or storage concerns.
- Hiding command differences behind a single generic signing API in the first iteration.
- Omitting Ledger app transaction/signing command families that are represented by the referenced JavaScript client.

## Decisions

### 1. The crate will be APDU-close, not signer-abstract

**Decision:** The public API will expose command-specific methods such as public-key retrieval and transaction-signing entry points that map closely to Ledger app capabilities. Internally, each capability will own its APDU instruction values, parameterization, request sequencing, and response parsing.

**Rationale:** The Concordium Ledger app does not expose a uniform "sign any Concordium thing" flow. Different commands require different packetization and sequencing patterns. Preserving that structure in the crate keeps behavior understandable, makes protocol tests direct, and avoids flattening meaningful differences into a leaky generic abstraction.

**Alternatives considered:**
- A high-level signer trait returning ready-to-submit transactions. Rejected because it mixes protocol concerns with transaction assembly and chain submission concerns.
- A raw `exchange(ins, p1, p2, data)` public API. Rejected because it would force every caller to understand Ledger packet choreography and Concordium serialization details.

### 2. The crate will return raw device outputs, not signed transactions

**Decision:** Signing-oriented methods return raw signature bytes or similarly low-level command outputs. The crate does not wrap those outputs into Concordium signed-transaction objects.

**Rationale:** The device protocol fundamentally produces signatures and related command responses. Constructing signed transactions is a distinct responsibility that depends on higher-level account-signature structure, transaction assembly, and node submission concerns. Keeping the crate at the raw-output layer makes it easier to compose and test, and leaves room for a separate higher-level integration layer later.

**Alternatives considered:**
- Returning fully signed transactions. Rejected because it would force the crate to own transaction assembly and would blur the boundary between protocol client and wallet orchestration.

### 3. The public API will use crate-local request types with optional SDK conversions

**Decision:** The crate defines its own request types for Ledger operations. A feature-gated `concordium-rust-sdk` dependency will enable `From` or `TryFrom` implementations from relevant SDK types into those request types.

**Rationale:** Crate-local request types give the crate a stable public contract shaped around Ledger needs rather than whatever structure upstream SDK types happen to expose. Feature-gated conversions preserve ergonomics for this repository and other SDK-using consumers without forcing all users to depend on the SDK.

**Alternatives considered:**
- Accepting only SDK types directly. Rejected because it couples the crate's API and semver to the SDK too tightly.
- Accepting only raw bytes. Rejected because it pushes too much translation burden to callers and makes common usage awkward.

### 4. Transport will be abstracted, with hardware-specific adapters behind features

**Decision:** The crate exposes a transport abstraction for APDU exchange. Concrete HID support can live behind a feature in the crate, or be layered later, but command logic must not depend directly on HID details.

**Rationale:** Transport abstraction is the simplest way to keep APDU sequencing testable and to avoid hard-wiring the crate to one runtime or hardware access path. It also allows a mock transport to validate exact request sequences in unit tests.

**Alternatives considered:**
- Embedding HID directly into every command path. Rejected because it makes testing harder and narrows future reuse.

### 5. Serialization will reuse Concordium canonical forms where practical, then apply Ledger-specific segmentation

**Decision:** For transaction-like flows, the crate should prefer deriving canonical serialized payloads from Concordium domain values where feasible, then reorganize or segment those bytes according to Ledger app expectations. Where the device protocol requires field-by-field staging, the crate will construct those staged payloads explicitly.

**Rationale:** This keeps the crate aligned with Concordium domain semantics while still honoring device-protocol realities. It also avoids reimplementing more serialization logic than necessary.

**Alternatives considered:**
- Reimplementing all Concordium serialization from scratch inside the crate. Rejected because it increases maintenance surface and divergence risk.

### 6. Command modules will be feature-oriented

**Decision:** Internal organization will follow capability-oriented modules, with one module per Ledger command family or closely related flow, plus shared APDU/serialization helpers.

**Rationale:** The protocol itself is capability-oriented. This layout keeps chunking logic, instruction constants, and response parsing near the relevant command, which is easier to navigate than a single monolithic protocol file.

## Risks / Trade-offs

- **Protocol drift between Ledger app versions and reference libraries** → Mitigation: keep instruction constants and sequence builders explicit, document assumptions, and add golden/mock tests per capability.
- **Feature-gated SDK conversions may lag behind SDK changes** → Mitigation: keep conversions thin, treat them as ergonomic adapters, and keep crate-local request types as the stable core API.
- **APDU-close API may feel verbose for consumers** → Mitigation: accept that trade-off in the foundation crate and allow higher-level wrappers in a later layer.
- **Chunking and sequencing bugs are easy to introduce** → Mitigation: make command choreography test-first with exact mock-exchange assertions and boundary-case chunking tests.
- **Transport abstraction can over-generalize too early** → Mitigation: keep the abstraction minimal and focused on APDU exchange rather than device lifecycle policy.

## Migration Plan

- Add the new workspace crate and its initial feature flags without wiring it into existing CLI commands.
- Land the low-level protocol client, transport abstraction, full referenced command surface, and command tests as an isolated addition.
- Integrate the crate into higher-level command flows only in follow-up changes once the protocol surface is stable.
- Rollback is straightforward: the crate can be removed from the workspace without affecting existing wallet flows because this change does not modify current signing paths.

## Open Questions

- Should HID support ship in the crate immediately, or should the first change stop at the abstract transport plus mock transport?
- Which SDK types deserve first-class conversion impls in v1, and which should wait until higher-level integration work proves demand?
