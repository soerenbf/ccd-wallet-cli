## ADDED Requirements

### Requirement: Ledger signer owners can be removed
The CLI SHALL provide a `ledger remove` command that removes an enrolled Ledger key source from local wallet state after explicit confirmation. The command SHALL accept an explicit Ledger key-source label or, in interactive mode, allow the user to select from configured Ledger key sources when the label is omitted. In `--non-interactive` mode, omitting the label SHALL be an error. Removal SHALL delete the Ledger signer owner through existing signer-owner deletion semantics, causing Ledger-owned identities, Ledger-owned derived accounts, owner vault metadata, Ledger owner details, and related private payload rows to be removed by cascade. Removal SHALL NOT require a connected Ledger device and SHALL NOT modify the physical Ledger device.

#### Scenario: Remove existing Ledger key source after confirmation
- **WHEN** the user runs `ccd-wallet ledger remove ledger-main`
- **AND** a Ledger key source labeled `ledger-main` exists
- **AND** the user confirms by typing `ledger-main`
- **THEN** the CLI deletes the Ledger signer owner
- **AND** deletes Ledger-owned identities and accounts by existing SQLite cascade semantics
- **AND** exits successfully with a confirmation message

#### Scenario: Removal warns about local owned state
- **WHEN** the user removes a Ledger key source that owns identities or accounts
- **THEN** the CLI warns that local removal will remove the identities and accounts owned by that key source
- **AND** the warning states that the physical Ledger device is not modified
- **AND** the CLI requires exact label confirmation before deleting local state

#### Scenario: Missing label opens Ledger selector
- **WHEN** the user runs `ccd-wallet ledger remove`
- **AND** `--non-interactive` is not supplied
- **THEN** the CLI opens a selector over configured Ledger key sources
- **AND** uses the selected label for confirmation and removal

#### Scenario: Remove rejects missing label in non-interactive mode
- **WHEN** the user runs `ccd-wallet ledger remove --non-interactive`
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating that the Ledger key-source label must be provided

#### Scenario: Remove rejects non-Ledger key source
- **WHEN** the user runs `ccd-wallet ledger remove seed-main`
- **AND** `seed-main` exists as a seed key source rather than a Ledger key source
- **THEN** the CLI exits with a non-zero status
- **AND** reports that Ledger key source `seed-main` is not configured
- **AND** does not delete the seed key source

#### Scenario: Remove rejected when confirmation does not match
- **WHEN** the user runs `ccd-wallet ledger remove ledger-main`
- **AND** a Ledger key source labeled `ledger-main` exists
- **AND** the user enters any confirmation other than `ledger-main`
- **THEN** the CLI exits with a non-zero status
- **AND** does not delete the Ledger signer owner

#### Scenario: Remove clears active key source when it targets removed Ledger
- **WHEN** `active_key_source` is `ledger-main`
- **AND** the user successfully removes Ledger key source `ledger-main`
- **THEN** the CLI removes the active key-source wallet-state entry

#### Scenario: Remove leaves unrelated active key source unchanged
- **WHEN** `active_key_source` is `other-source`
- **AND** the user successfully removes Ledger key source `ledger-main`
- **THEN** the CLI leaves `active_key_source` set to `other-source`
