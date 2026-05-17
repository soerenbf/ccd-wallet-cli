## ADDED Requirements

### Requirement: Account rename supports imported accounts
The CLI SHALL allow imported accounts to be renamed through the normal `account rename` flow. Renaming an imported account SHALL change only the account label and SHALL preserve imported source metadata and encrypted imported payloads.

#### Scenario: Rename imported account
- **WHEN** the user renames an imported account from `baker-0` to `local_baker`
- **THEN** the CLI updates the account label to `local_baker`
- **AND** preserves the account's imported source and encrypted imported secret payload

#### Scenario: Rename imported account rejects duplicate network label
- **WHEN** the user attempts to rename an imported account to a label already used by any account on the same network
- **THEN** the CLI rejects the rename with an actionable error
- **AND** does not modify the imported account

### Requirement: Account rename selection displays imported account provenance
Interactive account rename selection SHALL include imported accounts in the global fuzzy selector and SHALL include source metadata sufficient to distinguish imported accounts from seed-derived accounts.

#### Scenario: Imported account appears in fuzzy rename selector
- **WHEN** the user runs `ccd-wallet account rename` in interactive mode without an old label
- **AND** imported accounts exist
- **THEN** the fuzzy selector includes imported accounts
- **AND** imported account rows indicate that they are imported rather than seed-derived

#### Scenario: Searching by network finds imported accounts
- **WHEN** the user runs `ccd-wallet account rename` in interactive mode without an old label
- **AND** types a network name into the fuzzy selector
- **THEN** imported accounts on that network appear among the matches
