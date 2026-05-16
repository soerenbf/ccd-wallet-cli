## MODIFIED Requirements

### Requirement: Existing-entity choice uses selector instead of free-text input
Supported prompt-first command flows SHALL use a `cliclack` selector instead of a free-text prompt when the user must choose from already known seeds or networks.

#### Scenario: Seed delete chooses an existing seed through a selector
- **WHEN** the user runs `ccd-wallet seed delete` without a label
- **THEN** the CLI renders a `cliclack` selector over configured seeds
- **AND** uses the selected seed label for the destructive flow

#### Scenario: Network reset selector renders partition rows with hashes and aliases
- **WHEN** the user runs `ccd-wallet network reset` without a target
- **AND** the wallet has stored network data for genesis hash `abc`
- **AND** configured aliases `testnet` and `staging-testnet` reference `abc`
- **THEN** the CLI renders a partition-oriented row that shows the genesis hash and matching aliases

#### Scenario: Network reset selector can include orphaned hashes
- **WHEN** the user runs `ccd-wallet network reset` without a target
- **AND** the wallet has stored network data for a genesis hash not present in config
- **THEN** the CLI renders a selectable orphaned-hash target labeled as orphaned in addition to configured partitions

### Requirement: Destructive flows use cliclack warnings before confirmation
Supported destructive deletion and reset flows SHALL warn the user through `cliclack` before accepting typed confirmation.

#### Scenario: Seed delete warns about owned data removal
- **WHEN** the user runs `ccd-wallet seed delete main_seed`
- **AND** the seed owns stored identities or accounts
- **THEN** the CLI emits a `cliclack` warning explaining that those identities and accounts will also be removed before confirmation is requested

#### Scenario: Network delete warns when data will become orphaned
- **WHEN** the user runs `ccd-wallet network delete testnet`
- **THEN** the CLI emits a `cliclack` warning that explains the action removes config aliases only
- **AND** warns when the deletion will leave identities/accounts for that network hash orphaned before confirmation is requested
