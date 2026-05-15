## ADDED Requirements

### Requirement: Durable config file initialization
The CLI SHALL initialize a versioned durable config file at `~/.config/ccd-wallet/config.json` when it does not already exist, creating parent directories as needed.

#### Scenario: First-ever invocation on a clean system
- **WHEN** the user runs `ccd-wallet config network add` and no config file exists
- **THEN** the CLI creates the config directory and `config.json` with an empty networks map and `"version": 1`
- **AND** the command proceeds without error

#### Scenario: Config dir cannot be determined
- **WHEN** the platform config directory cannot be resolved
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error identifying that the config directory could not be determined

### Requirement: Config file schema stability
The config file SHALL include a top-level `version` field set to `1` to support future schema migrations.

#### Scenario: Inspect a saved config file
- **WHEN** a user opens `config.json` after at least one network has been added
- **THEN** the file contains `"version": 1` at the top level
- **AND** the file contains a `"networks"` object keyed by network name
