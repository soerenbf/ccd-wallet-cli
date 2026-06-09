# ledger-concordium-client Specification

## Purpose
TBD - created by archiving change add-ledger-concordium-crate. Update Purpose after archive.

## Requirements
### Requirement: The system SHALL provide a dedicated low-level Concordium Ledger client crate
The system SHALL provide a Rust crate dedicated to interacting with the Concordium Ledger hardware wallet application, scoped as a low-level protocol client rather than a wallet orchestration layer.

#### Scenario: Workspace includes a dedicated Ledger client crate
- **WHEN** the change is implemented
- **THEN** the Rust workspace contains a dedicated crate for Concordium Ledger integration under `crates/`
- **AND** the crate's documented scope excludes database access, account selection, signed transaction assembly, and chain submission

### Requirement: The Ledger client SHALL expose command-oriented typed APIs
The Ledger client SHALL expose typed command methods that closely mirror Concordium Ledger app capabilities while accepting request data in Concordium-oriented forms.

#### Scenario: Command API mirrors Ledger capability boundaries
- **WHEN** a caller uses the crate to request a Ledger operation
- **THEN** the caller invokes a command-specific typed API for that capability rather than a generic raw APDU exchange API
- **AND** the command-specific API preserves capability-specific behavior such as distinct signing flows for different command families

#### Scenario: Command API covers referenced Ledger app surface
- **WHEN** the referenced JavaScript client exposes a Concordium Ledger app command for public keys, address verification, transaction signing, credential signing, update-credentials signing, app-name retrieval, or private-key export
- **THEN** the Ledger client crate exposes a corresponding low-level typed command API
- **AND** the API returns the raw device result without assembling signed transactions or submitting anything to a node

### Requirement: The Ledger client SHALL translate request values into complete APDU sequences
For each supported capability, the Ledger client SHALL translate the provided request values into the complete APDU sequence required by the Concordium Ledger app, including request framing, sequential multi-call choreography, and payload chunking.

#### Scenario: Oversized request payload is chunked automatically
- **WHEN** a supported Ledger operation requires more request bytes than fit in one APDU payload
- **THEN** the Ledger client splits the payload according to that command's protocol rules
- **AND** sends the resulting APDU calls sequentially without requiring the caller to manage chunking manually

#### Scenario: Multi-stage command choreography is encapsulated
- **WHEN** a supported Ledger capability requires multiple APDU stages with different instruction parameters
- **THEN** the Ledger client performs the required stages in the correct order
- **AND** the caller supplies only the typed request input for that capability

### Requirement: Signing operations SHALL return raw device outputs
Signing-oriented APIs in the Ledger client SHALL return raw device outputs, such as signature bytes, rather than constructing signed Concordium transactions.

#### Scenario: Signing call returns signature bytes only
- **WHEN** a caller invokes a signing-oriented Ledger command successfully
- **THEN** the Ledger client returns the signature or other raw command output produced by the device
- **AND** the client does not assemble or submit a signed transaction on the caller's behalf

### Requirement: The Ledger client SHALL define crate-local request types
The Ledger client SHALL define crate-local request types for its public command APIs so the protocol-facing API remains stable even when external Concordium SDK types evolve.

#### Scenario: Public command API uses crate-local request type
- **WHEN** a caller targets a supported Ledger capability through the public API
- **THEN** the command accepts a request type defined by the Ledger client crate
- **AND** the request type is documented in terms of the Ledger capability it drives

### Requirement: SDK conversions SHALL be optional and feature-gated
If the Ledger client provides conversions from `concordium-rust-sdk` types into its crate-local request types, those conversions SHALL be enabled through an explicit feature-gated SDK dependency.

#### Scenario: Consumer uses crate without Concordium SDK feature
- **WHEN** a consumer depends on the Ledger client crate without enabling the SDK integration feature
- **THEN** the crate remains usable through its crate-local request types
- **AND** the consumer is not required to depend on `concordium-rust-sdk`

#### Scenario: Consumer enables SDK integration feature
- **WHEN** a consumer enables the SDK integration feature
- **THEN** the Ledger client exposes conversion implementations from supported SDK types into the corresponding crate-local request types
- **AND** higher-level code can build Ledger requests from SDK-domain values without duplicating translation logic

### Requirement: APDU transport SHALL be abstracted from command logic
The Ledger client's command logic SHALL depend on an APDU transport abstraction rather than directly depending on one concrete hardware transport implementation.

#### Scenario: Command logic is testable with a mock transport
- **WHEN** the Ledger client command sequence is tested
- **THEN** the test can provide a mock or fake APDU transport implementation
- **AND** the command logic can be validated without requiring a physical Ledger device
