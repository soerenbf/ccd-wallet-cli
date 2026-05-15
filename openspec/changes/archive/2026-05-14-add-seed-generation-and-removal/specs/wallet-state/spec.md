## ADDED Requirements

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
