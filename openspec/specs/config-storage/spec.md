# config-storage Specification

## Purpose
TBD - created by archiving change add-network-config. Update Purpose after archive.
## Requirements
### Requirement: Durable config file initialization
The CLI SHALL initialize a versioned durable config file at `~/.config/ccd-wallet/config.json` when it does not already exist, creating parent directories as needed.

#### Scenario: First-ever invocation on a clean system
- **WHEN** the user runs `ccd-wallet network add` and no config file exists
- **THEN** the CLI creates the config directory and `config.json` with an empty networks map and `"version": 1`
- **AND** the command proceeds without error

#### Scenario: Config dir cannot be determined
- **WHEN** the platform config directory cannot be resolved
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error identifying that the config directory could not be determined

### Requirement: Config file schema stability
The config file SHALL include a top-level `version` field set to `1` to support future schema migrations.

Network entries SHALL now include `wallet_proxy` in addition to `node_endpoint` and `genesis_hash`. Network names remain the keys inside the top-level `networks` object, renaming a network SHALL move the stored entry to a new key without changing the entry's `node_endpoint`, `genesis_hash`, or `wallet_proxy` values, and the config layer SHALL support deleting one or more aliases by network name and listing aliases that reference a given genesis hash.

#### Scenario: Inspect a saved config file
- **WHEN** a user opens `config.json` after at least one network has been added
- **THEN** the file contains `"version": 1` at the top level
- **AND** the file contains a `"networks"` object keyed by network name
- **AND** each network entry used for identity issuance contains `node_endpoint`, `genesis_hash`, and `wallet_proxy`

#### Scenario: Renaming network preserves stored entry data
- **WHEN** the user renames a configured network from `testnet` to `staging`
- **THEN** the `networks` object key changes from `testnet` to `staging`
- **AND** the stored `node_endpoint`, `genesis_hash`, and `wallet_proxy` values remain unchanged

#### Scenario: Delete one alias by name preserves other aliases
- **WHEN** configured aliases `testnet-a` and `testnet-b` both reference genesis hash `abc`
- **AND** the config layer deletes alias `testnet-a`
- **THEN** the `testnet-a` key is removed from the `networks` object
- **AND** the `testnet-b` key remains present

#### Scenario: List aliases by genesis hash
- **WHEN** configured aliases `testnet-a` and `testnet-b` both reference genesis hash `abc`
- **THEN** the config layer can return both alias names for genesis hash `abc`

