# ledger-identity-account-construction Specification

## Purpose
TBD - created by archiving change add-ledger-signer-owner-model. Update Purpose after archive.
## Requirements
### Requirement: Ledger construction layer bridges wallet flows to APDU commands
The system SHALL provide a Ledger identity/account construction layer between CLI orchestration and the low-level Ledger APDU client. The layer SHALL prepare the Ledger request payloads needed for identity issuance, account credential deployment, and account transaction signing, and SHALL return SDK-compatible values or raw signatures needed by higher-level wallet flows.

#### Scenario: CLI uses construction layer for Ledger identity issuance
- **WHEN** identity issuance targets a Ledger key source
- **THEN** the CLI invokes the Ledger construction layer rather than seed-derived `ConcordiumHdWallet` derivation methods
- **AND** the construction layer prepares the required Ledger app requests for the identity issuance flow

#### Scenario: CLI uses construction layer for Ledger account creation
- **WHEN** account creation targets a Ledger-owned identity
- **THEN** the CLI invokes the Ledger construction layer to prepare credential deployment payloads and Ledger signing requests
- **AND** the resulting credential deployment is suitable for normal node submission by the existing account creation flow

### Requirement: Ledger construction layer declares supported flows
The Ledger construction layer SHALL explicitly model which identity, credential, and account transaction flows are supported by the Concordium Ledger app. If a requested wallet flow cannot be represented by the available Ledger app commands, the layer SHALL return an actionable unsupported-flow error before any transaction is submitted.

#### Scenario: Unsupported Ledger identity flow fails before storage mutation
- **WHEN** a Ledger-backed identity issuance request requires a Ledger app operation that is not implemented or not supported
- **THEN** the wallet reports an unsupported Ledger identity flow
- **AND** no pending identity row is written

#### Scenario: Unsupported Ledger transaction flow fails before submission
- **WHEN** a Ledger-backed account transaction cannot be mapped to a supported Ledger signing command
- **THEN** the wallet reports an unsupported Ledger transaction flow
- **AND** no unsigned, partially signed, or fallback-signed transaction is submitted

### Requirement: Ledger construction layer does not implicitly export private key material
Ledger-backed identity and account flows SHALL NOT use Ledger private-key export commands as an implicit fallback. If a flow requires exporting key material from the Ledger, that behavior SHALL be represented as an explicit user-approved capability with security wording that distinguishes it from on-device signing.

#### Scenario: Missing on-device support does not silently export secrets
- **WHEN** a Ledger-backed flow lacks a supported on-device construction or signing command
- **THEN** the wallet does not call a Ledger private-key export command automatically
- **AND** the flow fails with an actionable unsupported-flow error

#### Scenario: Export-based flow requires explicit approval model
- **WHEN** a future change introduces an export-based Ledger flow
- **THEN** the flow requires explicit user-facing approval language
- **AND** the exported secret material is not stored in the wallet database as Ledger-owned persistent signing material

### Requirement: Ledger construction outputs are testable without physical hardware
The Ledger construction layer SHALL be testable with mock Ledger transports and deterministic serialized inputs. Tests SHALL verify request staging, owner matching, duplicate prevention, user rejection handling, and no-submission behavior for unsupported or rejected Ledger flows.

#### Scenario: Mock transport verifies staged Ledger requests
- **WHEN** a test constructs a Ledger-backed credential deployment through the construction layer
- **THEN** the mock Ledger transport captures the expected APDU sequence
- **AND** the test does not require a physical Ledger device

#### Scenario: User rejection fails safely
- **WHEN** the mock Ledger transport returns a user-rejection status for a signing operation
- **THEN** the construction layer returns a rejection error
- **AND** the CLI flow does not submit a transaction

