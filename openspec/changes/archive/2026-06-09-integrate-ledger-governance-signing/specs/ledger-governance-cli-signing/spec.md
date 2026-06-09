## ADDED Requirements

### Requirement: Ledger governance signing mode is exclusive
The CLI SHALL provide a Ledger-backed governance update signing mode selected by `governance update --ledger`, and that mode SHALL NOT combine Ledger signatures with local governance key vault signatures in the same invocation.

#### Scenario: Ledger mode rejects local key selection
- **WHEN** the user runs `ccd-wallet governance update --ledger --key <VERIFY_KEY>`
- **THEN** the CLI rejects the command before signing
- **AND** explains that Ledger governance signing cannot be mixed with local governance key vault signing

#### Scenario: Local signing remains default
- **WHEN** the user runs `ccd-wallet governance update` without `--ledger`
- **THEN** the CLI uses the existing local governance key vault signer selection flow
- **AND** does not attempt to contact a Ledger device

### Requirement: Ledger governance signing requires typed update payloads
Ledger governance signing SHALL require a governance update payload that the wallet can decode into a supported typed update family.

#### Scenario: Ledger mode rejects blind signing
- **WHEN** the user runs `ccd-wallet governance update --ledger --serialized <HEX> --blind`
- **AND** the serialized payload cannot be decoded as a known governance update payload
- **THEN** the CLI rejects the command before opening a Ledger signing flow
- **AND** explains that Ledger governance signing does not support blind signing

#### Scenario: Ledger mode signs decoded serialized payload
- **WHEN** the user runs `ccd-wallet governance update --ledger --serialized <HEX>`
- **AND** the serialized payload decodes as a supported governance update payload
- **THEN** the CLI may use the decoded typed payload for Ledger signing

### Requirement: Ledger governance signer path is derived from the update family
The CLI SHALL derive the Governance Ledger signer path from the authorization family implied by the governance update being signed, default the governance key index to `0`, allow that key index to be overridden explicitly, and support exactly one Ledger signer in the all-in-one update command.

#### Scenario: Root-governed update uses root governance purpose
- **WHEN** the user runs `ccd-wallet governance update --ledger` for a governance update that must be signed by root governance keys
- **THEN** the CLI constructs the Governance Ledger signer path using governance purpose `0`
- **AND** uses governance key index `0` unless the user explicitly overrides it

#### Scenario: Level-1-governed update uses level-1 governance purpose
- **WHEN** the user runs `ccd-wallet governance update --ledger` for a governance update that must be signed by level 1 governance keys
- **THEN** the CLI constructs the Governance Ledger signer path using governance purpose `1`
- **AND** uses governance key index `0` unless the user explicitly overrides it

#### Scenario: Level-2-governed update uses level-2 governance purpose
- **WHEN** the user runs `ccd-wallet governance update --ledger` for a governance update that must be signed by level 2 governance keys
- **THEN** the CLI constructs the Governance Ledger signer path using governance purpose `2`
- **AND** uses governance key index `0` unless the user explicitly overrides it

#### Scenario: Ledger key index override selects a different signer
- **WHEN** the user runs `ccd-wallet governance update --ledger --ledger-key-index <N>`
- **THEN** the CLI uses governance key index `<N>` when constructing the Governance Ledger signer path
- **AND** does not require the user to provide the full derivation path

#### Scenario: All-in-one Ledger mode accepts only one signer
- **WHEN** the user runs `ccd-wallet governance update --ledger` with multiple Ledger signer selectors
- **THEN** the CLI rejects the command before signing
- **AND** explains that the all-in-one Ledger signing flow currently supports only one Ledger signer

### Requirement: Ledger signer keys are authorized before submission
The CLI SHALL verify Ledger signer public keys against the resolved on-chain governance authorization context and SHALL submit only when the collected Ledger signatures satisfy the relevant authorization threshold.

#### Scenario: Unauthorized Ledger key is rejected
- **WHEN** the user runs `ccd-wallet governance update --ledger`
- **AND** the public key exported from the derived Governance Ledger path is not authorized for the selected governance update family
- **THEN** the CLI rejects the signer before submitting the update
- **AND** reports that the Ledger governance key is not authorized for the selected update family

#### Scenario: Threshold above one is rejected in all-in-one Ledger mode
- **WHEN** the user runs `ccd-wallet governance update --ledger`
- **AND** the selected governance update family currently requires more than one authorized signature
- **THEN** the CLI does not submit the governance update
- **AND** reports that the all-in-one Ledger signing flow currently supports only one signer and the update requires a higher threshold

#### Scenario: Authorized single Ledger signature is submitted
- **WHEN** the user runs `ccd-wallet governance update --ledger`
- **AND** the selected governance update family currently requires one authorized signature
- **AND** the connected Ledger signing request succeeds
- **THEN** the CLI assembles the Ledger signature into the signed governance update instruction
- **AND** submits the update through the existing governance update submission flow

### Requirement: Ledger device signing progress is explicit
Ledger-backed governance signing SHALL present clear progress and failure messages around device public-key lookup and signing operations.

#### Scenario: User declines Ledger signing on device
- **WHEN** the Governance Ledger app reports that the user declined a signing request
- **THEN** the CLI aborts the governance update
- **AND** reports that Ledger signing was declined

#### Scenario: Ledger signing waits before submission
- **WHEN** the CLI is waiting for the connected Ledger device to sign a governance update
- **THEN** the CLI indicates that device-backed signing is in progress before submitting anything to the node
