## ADDED Requirements

### Requirement: Governance updates can be submitted with Ledger-backed signatures
The governance update submission flow SHALL accept Ledger-backed signatures as an alternative to local governance key vault signatures when `--ledger` is selected, while preserving the same network resolution, sequence-number resolution, timing, submission, and finalization behavior used by local signing.

#### Scenario: Ledger-signed update uses existing submission flow
- **WHEN** the user runs `ccd-wallet governance update --ledger --json payload.json`
- **AND** Ledger signing succeeds with enough authorized governance signatures
- **THEN** the CLI submits the signed governance update to the resolved node endpoint
- **AND** waits for finalization unless `--no-wait` is supplied

#### Scenario: Ledger-signed update honors no-wait
- **WHEN** the user runs `ccd-wallet governance update --ledger --json payload.json --no-wait`
- **AND** Ledger signing and node submission succeed
- **THEN** the CLI returns after successful submission without waiting for finalization
- **AND** prints the submitted transaction hash or equivalent submission identifier

### Requirement: Governance update preparation is shared across signer backends
The governance update submission implementation SHALL prepare update payload, timing, sequence number, and chain authorization context before invoking either the local signer backend or the Ledger signer backend.

#### Scenario: Local backend receives the prepared update context
- **WHEN** the user submits a governance update without `--ledger`
- **THEN** the CLI signs the prepared update context with selected local governance key material
- **AND** preserves existing local governance signing behavior

#### Scenario: Ledger backend receives the prepared update context
- **WHEN** the user submits a governance update with `--ledger`
- **THEN** the CLI signs the prepared update context with the connected Governance Ledger app
- **AND** assembles the returned signatures into the same final governance update instruction structure used for submission
