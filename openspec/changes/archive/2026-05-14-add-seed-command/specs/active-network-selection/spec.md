## MODIFIED Requirements

### Requirement: Set active network by name
The CLI SHALL provide a `network use <NAME>` command that validates the named network exists in the config store and persists it as the active network in the wallet-state store.

#### Scenario: Select an existing network as active
- **WHEN** the user runs `ccd-wallet network use local`
- **AND** `local` exists in `config.json`
- **THEN** the CLI writes `active_network = local` to the SQLite `wallet_state` table
- **AND** exits successfully with a confirmation message

#### Scenario: Reject unknown active network selection
- **WHEN** the user runs `ccd-wallet network use unknown`
- **AND** `unknown` does not exist in `config.json`
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating the network is not registered
- **AND** does NOT write an active network selection to the wallet-state store
