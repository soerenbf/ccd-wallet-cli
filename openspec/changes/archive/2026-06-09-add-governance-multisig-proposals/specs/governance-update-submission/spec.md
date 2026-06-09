## ADDED Requirements

### Requirement: Governance proposal files can be created from JSON payload files
The CLI SHALL provide `ccd-wallet governance proposal create --json <FILE>` to create a detached governance proposal file from a governance update JSON payload. The created proposal file SHALL contain exactly a version number, the selected network genesis hash, the frozen update header, and the governance update payload JSON.

#### Scenario: Create proposal from JSON file
- **WHEN** the user runs `ccd-wallet governance proposal create --json payload.json --out proposal.json`
- **THEN** the CLI reads `payload.json` as a governance update payload
- **AND** resolves the selected network and current update sequence number from the node
- **AND** writes a proposal file containing `version`, `genesisHash`, `header`, and `payload`

#### Scenario: Create proposal requires explicit timing values
- **WHEN** the user runs `ccd-wallet governance proposal create --json payload.json`
- **AND** effective time or timeout has not been provided yet
- **THEN** the CLI requires the operator to enter both timing values explicitly
- **AND** does not apply detached-proposal defaults for either timing field

#### Scenario: Create proposal writes canonical pretty JSON
- **WHEN** the CLI writes a governance proposal file
- **THEN** it emits canonical pretty JSON derived from the parsed proposal data model
- **AND** does not preserve the original formatting of the input payload file

#### Scenario: Proposal payload size matches the frozen header
- **WHEN** the CLI writes a governance proposal file
- **THEN** it encodes the parsed governance update payload with the SDK update encoder
- **AND** stores a header whose `payloadSize` matches the encoded payload bytes

### Requirement: Detached governance proposals are revalidated before signing or submission
The CLI SHALL treat governance proposal files as frozen signing material but SHALL revalidate them online before detached signing or submission.

#### Scenario: Sign rejects proposal for the wrong network
- **WHEN** the user runs `ccd-wallet governance proposal sign proposal.json ...`
- **AND** the proposal file `genesisHash` does not match the selected network genesis hash
- **THEN** the CLI rejects the proposal before attempting to sign it

#### Scenario: Submit rejects proposal for the wrong network
- **WHEN** the user runs `ccd-wallet governance proposal submit proposal.json ...`
- **AND** the proposal file `genesisHash` does not match the selected network genesis hash
- **THEN** the CLI rejects the proposal before attempting to submit it

#### Scenario: Sign rejects stale proposal sequence number
- **WHEN** the user runs `ccd-wallet governance proposal sign proposal.json ...`
- **AND** the proposal header sequence number is no longer the current next sequence number for the payload's update queue
- **THEN** the CLI rejects the proposal as stale before producing a detached signature

#### Scenario: Submit rejects stale proposal sequence number
- **WHEN** the user runs `ccd-wallet governance proposal submit proposal.json ...`
- **AND** the proposal header sequence number is no longer the current next sequence number for the payload's update queue
- **THEN** the CLI rejects the proposal as stale before sending anything to the node

### Requirement: Detached governance signatures can be submitted with a proposal
The CLI SHALL provide `ccd-wallet governance proposal submit` to assemble a governance update from a proposal file plus one or more detached signature files and submit the resulting update to the node. The command SHALL accept detached signature input through repeated `--signature <FILE>` flags and through a directory input containing detached signature files.

#### Scenario: Submit detached signatures that satisfy threshold
- **WHEN** the user runs `ccd-wallet governance proposal submit proposal.json --signature sig-a.json --signature sig-b.json`
- **AND** the provided detached signatures are valid for the proposal signing hash
- **AND** the currently authorized signer set and threshold accept those signatures
- **THEN** the CLI assembles an update instruction and submits it to the resolved node

#### Scenario: Submit reads detached signatures from a directory
- **WHEN** the user runs `ccd-wallet governance proposal submit proposal.json --signature-dir signatures/`
- **AND** the directory contains detached signature files
- **THEN** the CLI loads detached signatures from that directory before threshold validation and submission

#### Scenario: Submitted detached proposal waits for finalization by default
- **WHEN** the user submits a detached governance proposal without `--no-wait`
- **THEN** the CLI waits for the governance update to finalize before returning success

#### Scenario: Submitted detached proposal honors no-wait
- **WHEN** the user submits a detached governance proposal with `--no-wait`
- **THEN** the CLI returns after successful submission without waiting for finalization
- **AND** still prints the submitted transaction hash or equivalent submission identifier

### Requirement: Detached signature files are minimal and signer-indexed
The CLI SHALL emit detached governance signature files that contain exactly a version number, the signer's governance verify key, and a `signature` object matching the logical `UpdateInstructionSignature` JSON shape. Each detached signature file SHALL contain exactly one signature entry keyed by the resolved `UpdateKeysIndex` for the signing verify key.

#### Scenario: Local detached signature stores resolved signer index
- **WHEN** the user signs a proposal with a local governance key
- **THEN** the CLI resolves the current signer index for that verify key from the node authorization context
- **AND** writes a detached signature file whose `signature.signatures` map contains exactly one entry for that index

#### Scenario: Detached signature file is written as canonical pretty JSON
- **WHEN** the CLI writes a detached governance signature file
- **THEN** it emits canonical pretty JSON derived from the parsed signature data model
- **AND** does not write a minified or formatting-preserving variant

#### Scenario: Submit rejects detached signature whose index does not match verify key
- **WHEN** the user runs `ccd-wallet governance proposal submit proposal.json --signature sig.json`
- **AND** the detached signature file's `verifyKey` no longer maps to the stored signature index in the current authorization context
- **THEN** the CLI rejects the detached signature before submission

#### Scenario: Submit rejects detached signatures below threshold
- **WHEN** the user runs `ccd-wallet governance proposal submit proposal.json --signature sig-a.json`
- **AND** the currently required threshold for the proposal update family is greater than the number of valid detached signatures provided
- **THEN** the CLI rejects the submission before sending anything to the node
