# wallet-state Specification

## Purpose
TBD - created by archiving change sqlite-wallet-store. Update Purpose after archive.
## Requirements
### Requirement: Application state stored in SQLite wallet_state table
The CLI SHALL persist mutable application state as key/value rows in the `wallet_state` table inside `wallet.db`. The `active_network` key SHALL be the initial entry.

#### Scenario: Active network persisted to wallet_state
- **WHEN** the user runs `ccd-wallet network use <NAME>`
- **AND** `<NAME>` is a registered network in `config.json`
- **THEN** the CLI writes a row `("active_network", "<NAME>")` to `wallet_state`
- **AND** exits with a confirmation message

#### Scenario: Active network read from wallet_state
- **WHEN** the CLI resolves the active network for a command (e.g., `node info`) with no `--network` or `--node` flag
- **THEN** the CLI reads `active_network` from `wallet_state`
- **AND** uses it to resolve the node endpoint from `config.json`

#### Scenario: Missing active_network key produces actionable error
- **WHEN** a command requires an active network
- **AND** `wallet_state` has no `active_network` key
- **THEN** the CLI exits with a non-zero status
- **AND** prints an error advising the user to run `ccd-wallet network use <NAME>` or provide `--network`/`--node`

### Requirement: Active seed stored in wallet_state table
The CLI SHALL persist the active seed label as a key/value row in the `wallet_state` table using key `active_seed`.

#### Scenario: Active seed persisted to wallet_state
- **WHEN** the user runs `ccd-wallet seed use <LABEL>`
- **AND** `<LABEL>` is a configured seed label
- **THEN** the CLI writes a row `("active_seed", "<LABEL>")` to `wallet_state`
- **AND** exits with a confirmation message

#### Scenario: Active seed read from wallet_state
- **WHEN** the user runs `ccd-wallet seed show` without a label
- **THEN** the CLI reads `active_seed` from `wallet_state`
- **AND** uses it to resolve which seed should be unlocked and displayed

#### Scenario: Missing active_seed key produces actionable error
- **WHEN** a command requires an active seed
- **AND** `wallet_state` has no `active_seed` key
- **THEN** the CLI exits with a non-zero status
- **AND** prints an error advising the user to run `ccd-wallet seed use <LABEL>` or provide a label explicitly

### Requirement: Removing active seed clears active_seed state
When a seed is removed, the CLI SHALL clear `wallet_state.active_seed` if and only if it currently points to the removed seed label.

#### Scenario: Remove currently active seed
- **WHEN** the user removes seed `main_seed`
- **AND** `wallet_state` contains `active_seed = main_seed`
- **THEN** the CLI deletes the `active_seed` key from `wallet_state`

#### Scenario: Remove inactive seed leaves active seed unchanged
- **WHEN** the user removes seed `old_seed`
- **AND** `wallet_state` contains `active_seed = main_seed`
- **THEN** the CLI leaves `active_seed = main_seed` unchanged

