## MODIFIED Requirements

### Requirement: Reset network-scoped wallet data explicitly
The CLI SHALL provide a `network reset` command that prunes wallet-local identities, accounts, imported-account vault data, and governance-vault data for a resolved network partition while keeping network config aliases intact. The command SHALL accept either a configured network label or `--genesis-hash <HASH>`. Supplying both a label and `--genesis-hash` SHALL be an error. The command SHALL NOT infer its target from the active network.

#### Scenario: Reset by configured label
- **WHEN** the user runs `ccd-wallet network reset testnet`
- **AND** `testnet` is configured with genesis hash `abc`
- **THEN** the CLI deletes all identities whose `network_genesis_hash = abc`
- **AND** deletes all accounts whose `network_genesis_hash = abc`
- **AND** deletes imported-account vault data whose `network_genesis_hash = abc`
- **AND** deletes governance-vault data whose `network_genesis_hash = abc`
- **AND** leaves the `testnet` config entry intact

#### Scenario: Reset by explicit genesis hash
- **WHEN** the user runs `ccd-wallet network reset --genesis-hash abc`
- **THEN** the CLI deletes all identities whose `network_genesis_hash = abc`
- **AND** deletes all accounts whose `network_genesis_hash = abc`
- **AND** deletes imported-account vault data whose `network_genesis_hash = abc`
- **AND** deletes governance-vault data whose `network_genesis_hash = abc`
- **AND** does not require a configured network alias for `abc`

#### Scenario: Reset does not use active network implicitly
- **WHEN** the user runs `ccd-wallet network reset`
- **AND** `--non-interactive` is supplied
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating that a network label or `--genesis-hash` must be provided

#### Scenario: Interactive reset renders partition rows with hashes and aliases
- **WHEN** the user runs `ccd-wallet network reset`
- **AND** interactive mode is enabled
- **AND** genesis hash `abc` has configured aliases `testnet` and `staging-testnet`
- **THEN** the selector includes a partition-oriented row for `abc`
- **AND** that row shows the genesis hash together with the matching aliases
- **AND** the row can include affected identity/account counts for that partition

#### Scenario: Interactive reset can target orphaned network data
- **WHEN** the user runs `ccd-wallet network reset`
- **AND** interactive mode is enabled
- **AND** stored identities, accounts, imported account vaults, or governance vaults reference genesis hash `abc`
- **AND** no configured network alias references `abc`
- **THEN** the selector includes an orphaned target for genesis hash `abc`
- **AND** that row is labeled as orphaned
- **AND** the user can choose it to prune that stored network data
