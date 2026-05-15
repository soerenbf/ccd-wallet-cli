## MODIFIED Requirements

### Requirement: Configurable Concordium node endpoint
The CLI SHALL allow the user to specify the target Concordium node for node commands using either a named network registered in the config store (`--network <NAME>`) or an explicit gRPC endpoint (`--node <ENDPOINT>`). If neither option is supplied, the CLI SHALL fall back to the active network from the wallet-state store. Supplying both `--network` and `--node` SHALL be an error.

#### Scenario: Explicit node endpoint is used when provided
- **WHEN** the user provides `--node <ENDPOINT>` to a node command
- **THEN** the CLI connects directly to that endpoint

#### Scenario: Named network resolves to its stored endpoint
- **WHEN** the user provides `--network <NAME>` to a node command
- **AND** the named network exists in the config store
- **THEN** the CLI resolves the endpoint from the network's stored `node_endpoint` and connects to it

#### Scenario: Active network is used when no selector is provided
- **WHEN** the user runs a node command without `--network` or `--node`
- **AND** the SQLite `wallet_state` table contains an `active_network` name
- **AND** that network exists in the config store
- **THEN** the CLI resolves the endpoint from the active network's stored `node_endpoint` and connects to it

#### Scenario: Missing active network produces a clear error
- **WHEN** the user runs a node command without `--network` or `--node`
- **AND** no active network is set in the wallet-state store
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating that the user must provide `--network` or `--node`, or set an active network with `ccd-wallet network use <NAME>`

#### Scenario: Stale active network produces a clear error
- **WHEN** the user runs a node command without `--network` or `--node`
- **AND** the active network named in `wallet_state` does not exist in `config.json`
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating that the active network is no longer registered

#### Scenario: Providing both flags is rejected
- **WHEN** the user provides both `--network` and `--node` to a node command
- **THEN** the CLI exits with a non-zero status
- **AND** prints an error indicating the flags are mutually exclusive

### Requirement: Read-only node connectivity command
The CLI SHALL provide at least one read-only command that connects to a Concordium node through the Concordium Rust SDK and returns node information to the user.

#### Scenario: Query a reachable node successfully
- **WHEN** the user runs the read-only node command against a reachable Concordium node endpoint
- **THEN** the CLI establishes a connection through the Concordium Rust SDK
- **AND** the command exits successfully
- **AND** the CLI prints returned node information in a human-readable form

#### Scenario: Surface a useful connection failure
- **WHEN** the user runs the read-only node command against an unreachable or invalid endpoint
- **THEN** the command exits with a non-zero status
- **AND** the CLI prints an actionable error message indicating that node connectivity failed
