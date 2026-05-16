## MODIFIED Requirements

### Requirement: Removing active seed clears active_seed state
When a seed is deleted, the CLI SHALL clear `wallet_state.active_seed` if and only if it currently points to the deleted seed label.

#### Scenario: Delete currently active seed
- **WHEN** the user deletes seed `main_seed`
- **AND** `wallet_state` contains `active_seed = main_seed`
- **THEN** the CLI deletes the `active_seed` key from `wallet_state`

#### Scenario: Delete inactive seed leaves active seed unchanged
- **WHEN** the user deletes seed `old_seed`
- **AND** `wallet_state` contains `active_seed = main_seed`
- **THEN** the CLI leaves `active_seed = main_seed` unchanged

### Requirement: Deleting active network alias clears active_network state
When a network delete flow removes the currently active network alias, the CLI SHALL clear `wallet_state.active_network`. Network reset SHALL NOT clear `active_network` by itself.

#### Scenario: Delete currently active network alias
- **WHEN** the user deletes network alias `testnet`
- **AND** `wallet_state` contains `active_network = testnet`
- **THEN** the CLI deletes the `active_network` key from `wallet_state`

#### Scenario: Delete inactive network alias leaves active network unchanged
- **WHEN** the user deletes network alias `old-testnet`
- **AND** `wallet_state` contains `active_network = testnet`
- **THEN** the CLI leaves `active_network = testnet` unchanged

#### Scenario: Reset network leaves active network unchanged
- **WHEN** the user resets network partition `abc`
- **AND** `wallet_state` contains `active_network = testnet`
- **AND** `testnet` remains configured for `abc`
- **THEN** the CLI leaves `active_network = testnet` unchanged
