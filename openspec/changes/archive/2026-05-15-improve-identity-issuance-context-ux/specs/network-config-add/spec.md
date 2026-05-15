## MODIFIED Requirements

### Requirement: Register a named network via CLI
The CLI SHALL provide a `network add` subcommand that accepts a user-supplied name, a node endpoint, and a wallet proxy URL, connects to the node, derives the genesis block hash, and persists a named network entry to the durable config store. In interactive mode, missing non-secret inputs SHALL be requested through `cliclack` prompts; in `--non-interactive` mode, missing values SHALL be an error.

#### Scenario: Successfully register a new network
- **WHEN** the user runs `ccd-wallet network add --name <NAME> --node <ENDPOINT> --wallet-proxy <URL>` and the name does not already exist in the config
- **THEN** the CLI connects to the node at the given endpoint
- **AND** queries consensus information to derive `genesis_block`
- **AND** writes the network entry to `config.json` with the normalized endpoint, genesis hash, and `wallet_proxy`
- **AND** exits successfully with a confirmation message

#### Scenario: Missing inputs are prompted interactively
- **WHEN** the user runs `ccd-wallet network add`
- **AND** `--non-interactive` is not supplied
- **THEN** the CLI prompts for the missing name, node endpoint, and wallet proxy URL using `cliclack`
- **AND** continues registration using the entered values

#### Scenario: Missing input in non-interactive mode errors
- **WHEN** the user runs `ccd-wallet network add --non-interactive`
- **AND** one or more of name, node endpoint, or wallet proxy URL is missing
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating which values must be provided

#### Scenario: Reject duplicate network name
- **WHEN** the user runs `ccd-wallet network add` with a name that already exists in the config
- **THEN** the CLI exits with a non-zero status
- **AND** prints an error indicating the name is already registered
- **AND** does NOT modify the existing entry

#### Scenario: Fail gracefully when node is unreachable
- **WHEN** the user runs `ccd-wallet network add` and the node at the given endpoint cannot be reached
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating the node could not be contacted
- **AND** does NOT write any entry to the config file
