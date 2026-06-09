# ledger-recovery-sync Specification

## Purpose
Ledger-backed recovery sync defines how enrolled Ledger key sources recover identities and accounts without storing Ledger secrets persistently.

## Requirements
### Requirement: Ledger sync recovers identities and accounts for an enrolled Ledger key source
The CLI SHALL provide `ccd-wallet ledger sync [LABEL]` to recover identities and accounts for an enrolled Ledger-backed key source on a selected network. The command SHALL resolve the target Ledger key source, selected network, and provider filters using the same user-facing conventions as seed recovery where applicable, including support for `--network`, repeated `--provider`, `--non-interactive`, and `--no-defaults`. The command SHALL NOT silently derive its target from an active key source. When the label is omitted in interactive mode, the CLI MAY present a selector and SHALL preselect the active key source only when that active key source is Ledger-backed. Recovery SHALL verify that the connected Concordium Ledger device matches the enrolled key source before any recovery export occurs.

#### Scenario: Sync recovers identities and accounts for a matching Ledger key source
- **WHEN** the user runs `ccd-wallet ledger sync ledger-main --network testnet`
- **AND** `ledger-main` is an enrolled Ledger key source
- **AND** the configured network `testnet` exists
- **AND** the connected Concordium Ledger device matches the enrolled key source
- **THEN** the CLI starts recovery for that Ledger key source on `testnet`
- **AND** imports any discovered identities and accounts under the enrolled Ledger signer owner
- **AND** exits with a recovery summary

#### Scenario: Sync rejects mismatched connected Ledger device
- **WHEN** the user runs `ccd-wallet ledger sync ledger-main --network testnet`
- **AND** `ledger-main` is an enrolled Ledger key source
- **AND** the connected Concordium Ledger device does not match the enrolled key source
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable owner-mismatch error
- **AND** does not export recovery material or import identities or accounts

#### Scenario: Interactive sync can preselect the active Ledger key source
- **WHEN** the user runs `ccd-wallet ledger sync --network testnet`
- **AND** the command is running interactively
- **AND** the active key source is an enrolled Ledger key source named `ledger-main`
- **THEN** the CLI presents a Ledger key-source selector
- **AND** preselects `ledger-main` in that selector
- **AND** does not silently start recovery until the user confirms a selection

#### Scenario: Non-interactive sync requires explicit label when defaults are disabled
- **WHEN** the user runs `ccd-wallet ledger sync --non-interactive --no-defaults --network testnet`
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating that the Ledger key-source label must be provided

### Requirement: Ledger recovery uses explicit approved export semantics
Ledger-backed recovery SHALL require explicit approval before exporting recovery-critical material from the connected device. Interactive runs SHALL require one explicit approval interaction covering the recovery command session before recovery probing proceeds. Non-interactive runs SHALL require an explicit allow flag before any export-backed recovery work begins. Declined or missing approval SHALL fail the command before any recovery import is written.

#### Scenario: Interactive sync requires one up-front approval before export-backed recovery
- **WHEN** the user runs `ccd-wallet ledger sync ledger-main --network testnet`
- **AND** the command is running interactively
- **THEN** the CLI requests one explicit approval for Ledger-backed recovery export before probing providers
- **AND** does not begin export-backed recovery until that approval is granted
- **AND** does not require a separate host-side approval prompt for each subsequent identity probe

#### Scenario: Non-interactive sync without allow flag is rejected
- **WHEN** the user runs `ccd-wallet ledger sync ledger-main --network testnet --non-interactive`
- **AND** the command would need to export Ledger recovery material
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error instructing the user to rerun with the explicit allow flag
- **AND** does not export recovery material or import identities or accounts

#### Scenario: Declined approval prevents partial recovery writes
- **WHEN** the user starts `ccd-wallet ledger sync ledger-main --network testnet`
- **AND** the user declines the explicit Ledger recovery export approval
- **THEN** the CLI exits with a non-zero status
- **AND** does not import any recovered identities or accounts for that command run

### Requirement: Ledger recovery derives provider and account probes from transient exported material
The recovery engine SHALL use exported Ledger recovery material only as transient host-memory input for identity recovery requests and credential registration id derivation. The wallet SHALL NOT persist exported Ledger recovery secrets as seed state, cached recovery state, or substitute signer material. Recovery SHALL continue to use wallet-proxy provider discovery, selected-provider filtering, node-based account discovery, and truthful progress and summary reporting. Ledger-backed probing SHALL run sequentially within a command session: for each selected provider, account discovery SHALL run first for locally known recovered identities, then identity indexes SHALL be probed one at a time starting at the next unused identity index and stop at the first missing identity unless existing local state already proves later identities should be considered. Account discovery SHALL run immediately for each newly recovered identity, and account indexes SHALL be probed until the account inactivity bound is hit.

#### Scenario: Recovery imports discovered identities without persisting exported secrets
- **WHEN** Ledger recovery successfully exports the material needed to recover an identity
- **AND** a provider returns a recovered identity object
- **THEN** the wallet imports that identity under the enrolled Ledger signer owner
- **AND** does not store the exported Ledger recovery secret material as persistent wallet state

#### Scenario: Recovery finds accounts from exported PRF-derived credential ids
- **WHEN** Ledger recovery has recovered an identity
- **AND** the user approves account-credential discovery material export for that recovered identity
- **AND** the node returns account information for a derived credential registration id
- **THEN** the wallet imports the discovered account under the enrolled Ledger signer owner
- **AND** continues probing additional candidate credentials according to the bounded account recovery scan

#### Scenario: Recovery probes providers, identities, and accounts sequentially
- **WHEN** Ledger recovery runs for multiple selected providers
- **THEN** the CLI probes one provider at a time
- **AND** for each provider it prompts for account discovery material for locally known recovered identities before probing new identity indexes
- **AND** then prompts for identity recovery material one unused identity index at a time
- **AND** stops probing new identity indexes for that provider when the first missing identity is observed unless existing local state requires continuing
- **AND** prompts for account discovery material immediately after each newly recovered identity
- **AND** for each recovered identity it probes account indexes until the account inactivity bound is reached

#### Scenario: Recovery summary reports provider failures separately from recovered entities
- **WHEN** Ledger recovery finishes after discovering some entities and encountering some skipped or failed providers
- **THEN** the CLI prints the recovered identity and account totals
- **AND** separately reports skipped or failed providers
- **AND** exits successfully if at least part of the recovery completed safely
