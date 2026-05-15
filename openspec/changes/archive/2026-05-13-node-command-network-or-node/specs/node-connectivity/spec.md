## MODIFIED Requirements

### Requirement: Configurable Concordium node endpoint
The CLI SHALL allow the user to specify the target Concordium node for node commands using either a named network registered in the config store (`--network <NAME>`) or an explicit gRPC endpoint (`--node <ENDPOINT>`). Exactly one of the two options MUST be provided; supplying both or neither SHALL be an error.

#### Scenario: Explicit node endpoint is used when provided
- **WHEN** the user provides `--node <ENDPOINT>` to a node command
- **THEN** the CLI connects directly to that endpoint

#### Scenario: Named network resolves to its stored endpoint
- **WHEN** the user provides `--network <NAME>` to a node command
- **AND** the named network exists in the config store
- **THEN** the CLI resolves the endpoint from the network's stored `node_endpoint` and connects to it

#### Scenario: Unknown network name produces a clear error
- **WHEN** the user provides `--network <NAME>` and that name is not registered in the config store
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating the network is not registered

#### Scenario: Providing both flags is rejected
- **WHEN** the user provides both `--network` and `--node` to a node command
- **THEN** the CLI exits with a non-zero status
- **AND** prints an error indicating the flags are mutually exclusive

#### Scenario: Providing neither flag is rejected
- **WHEN** the user runs a node command without `--network` or `--node`
- **AND** `CCD_WALLET_NODE_ENDPOINT` is not set
- **THEN** the CLI exits with a non-zero status
- **AND** prints an error indicating that one of `--network` or `--node` is required
