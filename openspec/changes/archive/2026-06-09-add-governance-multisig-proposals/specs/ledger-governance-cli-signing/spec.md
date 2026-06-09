## ADDED Requirements

### Requirement: Detached governance signing keeps signer backend selection explicit
The CLI SHALL support detached governance proposal signing with the same local signer selection behavior used by the all-in-one governance update flow, and SHALL provide a Ledger-backed detached signing mode through `ccd-wallet governance proposal sign --ledger`.

#### Scenario: Ledger signs detached proposal
- **WHEN** the user runs `ccd-wallet governance proposal sign proposal.json --ledger --out sig.json`
- **AND** the proposal payload decodes as a supported governance update payload
- **AND** the proposal remains valid after online revalidation
- **THEN** the CLI signs the proposal with the connected Concordium Governance Ledger app
- **AND** writes a detached signature file

#### Scenario: Local detached signing reuses the existing selection flow
- **WHEN** the user runs `ccd-wallet governance proposal sign proposal.json` without `--ledger` and without explicit key flags
- **THEN** the CLI uses the same interactive governance-key selection flow used by the all-in-one governance update command
- **AND** signs with selected local governance key material

#### Scenario: Ledger detached signing rejects unknown or blind payloads
- **WHEN** the user runs `ccd-wallet governance proposal sign proposal.json --ledger`
- **AND** the proposal payload cannot be decoded as a supported typed governance update payload
- **THEN** the CLI rejects the signing request before opening a Ledger blind-signing flow

### Requirement: Detached Ledger signing resolves signer index from the live authorization context
For detached Ledger signing, the CLI SHALL derive the Governance Ledger path from the proposal's update family, export the Ledger public key, confirm that the key is currently authorized for the proposal, and write the resulting detached signature under the resolved `UpdateKeysIndex`.

#### Scenario: Detached Ledger signing derives path from update family
- **WHEN** the user runs `ccd-wallet governance proposal sign proposal.json --ledger`
- **AND** the proposal payload requires root governance authorization
- **THEN** the CLI constructs the Governance Ledger signer path using governance purpose `0`

#### Scenario: Detached Ledger signing honors ledger key index override
- **WHEN** the user runs `ccd-wallet governance proposal sign proposal.json --ledger --ledger-key-index <N>`
- **THEN** the CLI uses governance key index `<N>` when constructing the Governance Ledger signer path

#### Scenario: Detached Ledger signing rejects unauthorized Ledger key
- **WHEN** the user runs `ccd-wallet governance proposal sign proposal.json --ledger`
- **AND** the Ledger public key exported from the derived signer path is not currently authorized for the proposal update family
- **THEN** the CLI rejects the signing request before writing a detached signature file

#### Scenario: Detached Ledger signature file stores resolved signer index
- **WHEN** detached Ledger signing succeeds
- **THEN** the CLI writes a detached signature file whose `verifyKey` matches the exported Ledger governance public key
- **AND** whose `signature.signatures` map contains exactly one entry for the resolved signer index

### Requirement: Detached Ledger signing supports multi-party thresholds through repeated detached signatures
The detached Ledger signing flow SHALL NOT enforce the all-in-one single-signer submission limitation, and SHALL instead allow operators to produce one detached signature file per authorized Ledger signer for later submission.

#### Scenario: Threshold-greater-than-one proposal can still be Ledger-signed detached
- **WHEN** the user runs `ccd-wallet governance proposal sign proposal.json --ledger`
- **AND** the proposal update family currently requires more than one signature
- **THEN** the CLI may still produce a detached signature file for the connected authorized Ledger signer
- **AND** does not require the threshold to be satisfied during that signing invocation
