## ADDED Requirements

### Requirement: Governance Ledger client crate is isolated and low-level
The workspace SHALL provide a dedicated Rust crate for low-level interaction with the Concordium Governance Ledger app. The crate SHALL NOT perform wallet database access, governance key vault access, signer selection, CLI prompting, signed update assembly, node submission, or finalization waiting.

#### Scenario: Crate is usable without wallet state
- **WHEN** a caller constructs the Governance Ledger client with an APDU transport
- **THEN** the client can execute supported device commands without requiring a wallet database connection or governance key vault password

#### Scenario: Crate does not submit governance updates
- **WHEN** a signing command returns successfully
- **THEN** the crate returns raw device output
- **AND** does not submit anything to a Concordium node

### Requirement: Governance Ledger client exposes a single typed public client
The crate SHALL expose one primary public client type for the Concordium Governance Ledger app. The client SHALL provide typed methods for public-key retrieval and governance update signing command families rather than requiring callers to perform raw APDU exchange.

#### Scenario: Caller uses one client entry point
- **WHEN** a caller wants to retrieve a governance public key or sign a supported governance update
- **THEN** the caller can use methods on the single Governance Ledger client type

#### Scenario: Raw APDU choreography is hidden from ordinary callers
- **WHEN** a caller invokes a typed signing method
- **THEN** the crate constructs the required APDU commands, P1/P2 values, staged requests, and chunks internally

### Requirement: Governance public keys can be exported from the device
The client SHALL support Governance Ledger public-key retrieval for supplied derivation paths. Public-key retrieval SHALL support the Governance Ledger app's confirmation and signed-public-key options where the device protocol supports them.

#### Scenario: Retrieve governance public key
- **WHEN** a caller requests a public key for a governance derivation path
- **THEN** the client sends the Governance Ledger public-key command
- **AND** returns the raw public key bytes from the device

#### Scenario: Retrieve signed governance public key
- **WHEN** a caller requests a signed public key for a governance derivation path
- **THEN** the client requests the signed-public-key variant from the device
- **AND** returns the public key and the device-provided signature bytes

### Requirement: Governance update signing command surface is covered
The client SHALL expose typed signing methods for the full Governance Ledger app command surface represented by the governance app source, tests, and instruction docs, including higher-level key updates, level 2 authorization updates, parameter updates, add anonymity revoker, add identity provider, protocol update, and create PLT flows.

#### Scenario: Sign fixed-shape governance parameter update
- **WHEN** a caller provides a typed request for a fixed-shape governance parameter update supported by the Governance Ledger app
- **THEN** the client sends the command sequence required by that update family
- **AND** returns the raw signature after device approval

#### Scenario: Sign staged governance update
- **WHEN** a caller provides a typed request for a governance update that requires staged field uploads or chunked payload data
- **THEN** the client sends all required APDU stages in protocol order
- **AND** returns the raw signature only after the final successful device response

#### Scenario: Sign governance key update
- **WHEN** a caller provides a typed request for a root, level 1, or level 2 governance key update supported by the Governance Ledger app
- **THEN** the client sends the update header, key material, authorization structures, thresholds, and version selectors required by the device protocol
- **AND** returns the raw signature after device approval

### Requirement: Device outputs remain raw protocol outputs
The client SHALL return raw public-key bytes, optional signed-public-key bytes, and raw signatures from device commands. The client SHALL NOT assemble signed governance update instructions, signed block items, or update signature maps.

#### Scenario: Signing returns raw signature
- **WHEN** a governance update signing command succeeds
- **THEN** the returned value contains the raw signature bytes from the device
- **AND** does not contain a constructed `UpdateInstruction`

#### Scenario: Higher-level code maps signatures
- **WHEN** higher-level wallet code needs a signed governance update instruction
- **THEN** it is responsible for associating raw signatures with governance key indices and assembling the signed update instruction

### Requirement: Unknown serialized governance payloads are not blind-signed by the client
The client SHALL NOT expose a generic blind-sign method for unknown serialized governance update payloads unless the Governance Ledger app exposes a suitable generic protocol capability. Ledger-backed governance signing SHALL be limited to typed device-supported flows.

#### Scenario: Unknown payload has no generic signing method
- **WHEN** a caller has serialized governance update bytes that cannot be decoded into a supported typed Governance Ledger request
- **THEN** the client does not provide a generic blind-sign command for those bytes

#### Scenario: Local-key blind signing remains outside the client
- **WHEN** the wallet supports blind signing with locally stored governance key material
- **THEN** that behavior remains outside the Governance Ledger client crate

### Requirement: Command logic is transport-abstracted and ships with HID and mock transport support
The crate SHALL define a minimal APDU transport abstraction and SHALL keep command construction and response parsing independent of concrete hardware transport details. The initial version SHALL include concrete HID transport support and a mock transport so tests can verify command sequences without a physical Ledger device.

#### Scenario: Mock transport records APDU sequence
- **WHEN** a test invokes a typed Governance Ledger command with a mock transport
- **THEN** the test can assert the exact APDU commands sent by the client

#### Scenario: HID transport is available in the initial version
- **WHEN** a caller wants to communicate with a Ledger device over HID
- **THEN** the crate provides concrete HID transport support in its initial version
- **AND** command construction remains layered on the transport abstraction rather than on HID-specific logic

### Requirement: SDK integration is optional and feature-gated like `ccd-wallet-ledger`
The crate SHALL define crate-local public request and response types as its stable API. Any conversions from `concordium-rust-sdk` governance or update types SHALL be optional and feature-gated, following the same SDK-optional pattern as `ccd-wallet-ledger`.

#### Scenario: Crate builds without SDK feature
- **WHEN** the crate is built with default features
- **THEN** it does not require `concordium-rust-sdk` conversions to compile

#### Scenario: SDK feature enables From conversions for update requests in the initial version
- **WHEN** the optional SDK feature is enabled
- **THEN** supported SDK governance/update values can be converted into corresponding crate-local update request types through feature-gated `From` implementations whose source values contain the actual SDK payload types
- **AND** those conversions are available in the initial version of the crate rather than deferred to a later change
- **AND** Ledger-only context such as derivation path, update header, and update-family selectors is supplied explicitly alongside the SDK payload rather than guessed or represented by custom SDK-prefixed wrapper request types
