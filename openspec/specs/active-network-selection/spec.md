# active-network-selection Specification

## Purpose
TBD - created by archiving change add-active-network-state. Update Purpose after archive.
## Requirements
### Requirement: Set active network by name
The CLI SHALL provide a `network use [NAME]` command that validates the named network exists in the config store and persists it as the active network in the wallet-state store. In interactive mode, omitting the name SHALL open a selector over configured networks; in `--non-interactive` mode, omitting the name SHALL be an error.

#### Scenario: Select an existing network as active
- **WHEN** the user runs `ccd-wallet network use local`
- **AND** `local` exists in `config.json`
- **THEN** the CLI writes `active_network = local` to the SQLite `wallet_state` table
- **AND** exits successfully with a confirmation message

#### Scenario: Missing name opens a selector for network use
- **WHEN** the user runs `ccd-wallet network use`
- **AND** `--non-interactive` is not supplied
- **THEN** the CLI opens a `cliclack` selector over configured network names
- **AND** preselects the active network when one exists
- **AND** uses the selected name for active-network selection

#### Scenario: Reject missing name in non-interactive mode
- **WHEN** the user runs `ccd-wallet network use --non-interactive`
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating that the network name must be provided

#### Scenario: Reject unknown active network selection
- **WHEN** the user runs `ccd-wallet network use unknown`
- **AND** `unknown` does not exist in `config.json`
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating the network is not registered
- **AND** does NOT write an active network selection to the wallet-state store

