## ADDED Requirements

### Requirement: Chain-parameter inspection reuses node connectivity resolution
The `node chain-parameters` command SHALL use the same endpoint resolution, active-network default, explicit network selection, direct node endpoint, node-endpoint environment variable, no-defaults picker, and actionable connection-error behavior as existing node commands.

#### Scenario: Chain-parameter query uses resolved endpoint
- **WHEN** endpoint resolution succeeds for `node chain-parameters`
- **THEN** the CLI performs the gRPC chain-parameter query against that resolved endpoint

#### Scenario: Chain-parameter connection failure is actionable
- **WHEN** the resolved endpoint cannot be reached
- **THEN** the command exits with a non-zero status
- **AND** identifies the endpoint that could not be connected to

#### Scenario: Chain-parameter query failure is actionable
- **WHEN** the node connection succeeds but the chain-parameter gRPC query fails
- **THEN** the command exits with a non-zero status
- **AND** identifies chain-parameter retrieval as the failed operation

### Requirement: Chain-update inspection reuses node connectivity resolution
The `node chain-updates` command SHALL use the same endpoint resolution, active-network default, explicit network selection, direct node endpoint, node-endpoint environment variable, no-defaults picker, and actionable connection-error behavior as `node chain-parameters`.

#### Scenario: Chain-update query uses resolved endpoint
- **WHEN** endpoint resolution succeeds for `node chain-updates`
- **THEN** all chain-update gRPC queries use that resolved endpoint

#### Scenario: Chain-update connection failure is actionable
- **WHEN** the resolved endpoint cannot be reached
- **THEN** the command exits with a non-zero status
- **AND** identifies the endpoint that could not be connected to
