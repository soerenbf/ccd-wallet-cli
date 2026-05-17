# entity-listing Specification

## Purpose
TBD - created by archiving change add-list-and-rename-commands. Update Purpose after archive.
## Requirements
### Requirement: Managed entities can be listed through entity-family commands
The CLI SHALL provide human-oriented `list` subcommands for `network`, `seed`, `identity`, and `account`.

`network list` SHALL display configured network names and their node endpoints. `seed list` SHALL display configured seed labels and plaintext metadata available without unlock. `identity list` and `account list` SHALL display human-oriented summaries for the entities visible within the resolved scope.

This change does not require machine-readable output such as `--json`.

#### Scenario: List networks
- **WHEN** the user runs `ccd-wallet network list`
- **THEN** the CLI displays the configured network names in a human-oriented list
- **AND** includes each network's node endpoint

#### Scenario: List seeds
- **WHEN** the user runs `ccd-wallet seed list`
- **THEN** the CLI displays the configured seed labels in a human-oriented list
- **AND** does not prompt for a password

### Requirement: Identity and account listing use context-aware scope resolution and filters
`identity list` and `account list` SHALL resolve seed scope and network scope using the same active/default override model as other context-bearing commands. In addition to concrete labels, these list commands SHALL accept explicit wildcard values `all` for `--seed` and `--network`. If explicit scope arguments are supplied, the resolved scope SHALL be shown as context before the results are displayed.

After scope is resolved, the list commands SHALL support additional entity-specific filters where relevant.

For the first cut:
- `identity list` SHALL support `--provider` and `--status`
- `account list` SHALL support `--status`

This allows queries such as all identities on testnet created by identity provider `0`, or all pending accounts across all networks for one seed.

#### Scenario: Identity list defaults to active context
- **WHEN** the user runs `ccd-wallet identity list`
- **AND** an active seed and active network are configured
- **THEN** the CLI lists identities for that seed and network by default
- **AND** shows the resolved seed/network context before the results

#### Scenario: Explicit all-seed scope broadens listing
- **WHEN** the user runs `ccd-wallet identity list --seed all --network testnet`
- **THEN** the CLI lists identities across all seeds on `testnet`
- **AND** shows `seed: all` and `network: testnet` as resolved context

#### Scenario: Explicit all-network scope broadens listing
- **WHEN** the user runs `ccd-wallet account list --seed test --network all`
- **THEN** the CLI lists accounts across all configured networks for seed `test`
- **AND** shows `seed: test` and `network: all` as resolved context

#### Scenario: Provider filter narrows identity list
- **WHEN** the user runs `ccd-wallet identity list --network testnet --provider 0`
- **THEN** the CLI lists only identities on `testnet` whose identity provider id is `0`

#### Scenario: Identity status filter narrows identity list
- **WHEN** the user runs `ccd-wallet identity list --status pending`
- **THEN** the CLI lists only identities whose effective status matches `pending`

#### Scenario: Account status filter narrows account list
- **WHEN** the user runs `ccd-wallet account list --status pending`
- **THEN** the CLI lists only accounts whose status matches `pending`

### Requirement: Account list hides addresses unless explicitly requested
`account list` SHALL hide account addresses by default. The CLI SHALL support an explicit flag to reveal addresses in list output.

If addresses are requested, the CLI MAY require unlocking one or more seed domains to decrypt them.

#### Scenario: Default account list hides addresses
- **WHEN** the user runs `ccd-wallet account list`
- **THEN** the CLI displays human-oriented account metadata
- **AND** does not include account addresses by default

#### Scenario: Explicit flag reveals account addresses
- **WHEN** the user runs `ccd-wallet account list` with the address-reveal flag
- **THEN** the CLI includes account addresses in the output
- **AND** may prompt for the necessary seed password material to decrypt them

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

