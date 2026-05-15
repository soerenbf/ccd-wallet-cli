## ADDED Requirements

### Requirement: Configurable Concordium node endpoint
The CLI SHALL allow the user to select the Concordium node endpoint through command-line input, with an environment-variable-based fallback for local development.

#### Scenario: Command-line endpoint overrides environment configuration
- **WHEN** the user provides a node endpoint flag and an environment variable is also set
- **THEN** the CLI uses the value supplied on the command line

#### Scenario: Environment configuration is used when no flag is supplied
- **WHEN** the user runs a node command without a node endpoint flag
- **AND** the configured environment variable is set
- **THEN** the CLI uses the endpoint from the environment variable

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
