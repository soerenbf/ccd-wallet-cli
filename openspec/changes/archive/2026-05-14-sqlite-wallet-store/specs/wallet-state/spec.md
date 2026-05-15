## ADDED Requirements

### Requirement: Application state stored in SQLite wallet_state table
The CLI SHALL persist mutable application state as key/value rows in the `wallet_state` table inside `wallet.db`. The `active_network` key SHALL be the initial entry.

#### Scenario: Active network persisted to wallet_state
- **WHEN** the user runs `ccd-wallet config network use <NAME>`
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
- **AND** prints an error advising the user to run `ccd-wallet config network use <NAME>` or provide `--network`/`--node`

## REMOVED Requirements

### Requirement: Separate persistent state file
**Reason**: Replaced by `wallet_state` table in `wallet.db`. A separate JSON state file is no longer needed now that SQLite is the persistence layer.
**Migration**: No migration is implemented; the app has no existing users and `state.json` is removed outright.

### Requirement: Versioned state schema
**Reason**: Schema versioning is now handled by the `schema_version` table in `wallet.db` (see `sqlite-store` capability). A `version` field in a JSON file is no longer applicable.
**Migration**: Subsumed by the SQLite schema migration system.
