## ADDED Requirements

### Requirement: Account listing includes imported accounts
The CLI SHALL include imported accounts in normal `account list` results when they match the resolved network and filter scope. Imported accounts SHALL be visibly distinguishable from seed-derived accounts in human-oriented output without exposing encrypted addresses by default.

#### Scenario: Imported account appears in network account list
- **WHEN** the user runs `ccd-wallet account list --network local --seed all`
- **AND** an imported account exists on `local`
- **THEN** the CLI includes the imported account in the account list output
- **AND** identifies it as imported or otherwise not seed-derived

#### Scenario: Imported account appears without seed scope when network matches
- **WHEN** the user lists accounts for a network containing imported accounts
- **THEN** imported accounts are included even though they do not belong to any seed

### Requirement: Imported account addresses remain hidden by default
The CLI SHALL apply the same address privacy policy to imported accounts as to derived accounts. Imported account addresses SHALL be omitted from list output unless the user explicitly requests address display.

#### Scenario: Default account list hides imported account address
- **WHEN** the user runs `ccd-wallet account list` without the address-reveal flag
- **AND** the result set includes imported accounts
- **THEN** imported account addresses are not included in the output

#### Scenario: Explicit address reveal unlocks imported vault
- **WHEN** the user runs `ccd-wallet account list --show-addresses`
- **AND** the result set includes imported accounts
- **THEN** the CLI prompts for the relevant imported accounts vault password before showing imported account addresses
- **AND** includes imported account addresses only after successful vault unlock
