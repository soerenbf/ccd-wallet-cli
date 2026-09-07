# chain-parameter-inspection Specification

## Purpose
Provide consistent read-only inspection of all normalized Concordium chain parameters at a selected block for operators and automation.

## Requirements

### Requirement: Chain parameters can be queried from the node command space
The CLI SHALL provide `ccd-wallet node chain-parameters` and SHALL query all normalized chain parameters through the Concordium node gRPC API.

#### Scenario: Query last finalized chain parameters by default
- **WHEN** the user runs `ccd-wallet node chain-parameters` without `--block`
- **THEN** the CLI queries chain parameters at the last finalized block
- **AND** reports the block hash resolved by the node

#### Scenario: Query chain parameters at an explicit block
- **WHEN** the user runs `ccd-wallet node chain-parameters --block <BLOCK>`
- **THEN** the CLI parses `<BLOCK>` with the same block-selector syntax used by existing read-only commands
- **AND** queries chain parameters at the selected block

#### Scenario: Invalid block selector fails actionably
- **WHEN** the user supplies an invalid `--block` value
- **THEN** the CLI exits with a non-zero status
- **AND** identifies the invalid block selector

### Requirement: Chain-parameter command follows existing read-only flags and input behavior
The command SHALL expose `--network <NAME>`, `--node <ENDPOINT>`, `--block <BLOCK>`, `--json`, and `--no-defaults` with the same meanings, conflict rules, environment fallback, default resolution, and interactive network selection behavior as comparable read-only node-backed commands. The command SHALL NOT introduce required positional input or command-specific prompts.

#### Scenario: Active network is used by default
- **WHEN** the user runs `ccd-wallet node chain-parameters` without `--network`, `--node`, or `--no-defaults`
- **AND** an active configured network exists
- **THEN** the command queries that network's stored node endpoint without prompting

#### Scenario: No-defaults uses the existing network picker
- **WHEN** the user runs `ccd-wallet node chain-parameters --no-defaults` without `--network` or `--node`
- **THEN** the CLI uses the existing configured-network selection UI
- **AND** preselects the active network when one exists

#### Scenario: Explicit node uses endpoint environment behavior
- **WHEN** the user supplies `--node <ENDPOINT>` or the existing node-endpoint environment variable
- **THEN** the command connects directly to that endpoint according to existing node-command behavior

#### Scenario: Network and node conflict
- **WHEN** the user supplies both `--network` and `--node`
- **THEN** clap rejects the invocation as mutually exclusive before querying

### Requirement: Chain parameters have human-readable and JSON output
The command SHALL render a human-readable view by default and SHALL emit only a pretty-printed, camel-case JSON document when `--json` is supplied. Both forms SHALL be produced from the same wallet-owned normalized output model and SHALL include every chain parameter represented by the SDK response, including governance update keys and authorization structures.

#### Scenario: Human output shows all available parameters
- **WHEN** the user runs `ccd-wallet node chain-parameters` without `--json`
- **THEN** the CLI prints a scannable human-readable parameter view
- **AND** includes the resolved endpoint and block hash context
- **AND** displays every parameter available at the selected protocol version

#### Scenario: JSON output is machine-readable
- **WHEN** the user runs `ccd-wallet node chain-parameters --json`
- **THEN** stdout contains one valid pretty-printed JSON document and no interactive decoration
- **AND** its field names use camel case
- **AND** it includes endpoint and block-hash context

#### Scenario: Unsupported-version fields remain explicit
- **WHEN** a normalized chain parameter is unavailable at the selected protocol version
- **AND** the user requests JSON output
- **THEN** the corresponding stable output field is `null` rather than being omitted

### Requirement: P11 token and lock parameters are visible
The chain-parameter output SHALL expose the maximum lock duration and token-parameters authorization structure returned for protocol version 11.

#### Scenario: P11 response includes maximum lock duration
- **WHEN** the selected block returns a maximum lock duration
- **THEN** human output displays it as a Concordium duration
- **AND** JSON output includes it as `maxLockDuration`

#### Scenario: P11 response includes token-parameters authorization
- **WHEN** the selected block returns a token-parameters authorization structure
- **THEN** the output includes its authorized key indices and threshold
