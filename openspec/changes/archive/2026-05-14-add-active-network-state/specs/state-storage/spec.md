## ADDED Requirements

### Requirement: Separate persistent state file
The CLI SHALL persist mutable operational state in `~/.config/ccd-wallet/state.json`, separate from `~/.config/ccd-wallet/config.json`.

#### Scenario: First state write creates the file
- **WHEN** the CLI persists mutable state for the first time
- **THEN** it creates `~/.config/ccd-wallet/state.json` if it does not already exist
- **AND** the file contains a versioned schema

### Requirement: Versioned state schema
The state file SHALL include a top-level `version` field set to `1`.

#### Scenario: Inspect the state file
- **WHEN** a user opens `state.json` after setting an active network
- **THEN** the file contains `"version": 1`
- **AND** the file contains an `"active_network"` field with the selected network name
