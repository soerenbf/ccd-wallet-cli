# chain-update-inspection Specification

## Purpose
Provide operators and automation with a consistent read-only view of scheduled governance changes and the sequence numbers required for subsequent updates.

## Requirements

### Requirement: Chain update state can be queried from the node command space
The CLI SHALL provide `ccd-wallet node chain-updates` and SHALL query both pending chain updates and all next governance update sequence numbers through the Concordium node gRPC API.

#### Scenario: Query last finalized chain update state by default
- **WHEN** the user runs `ccd-wallet node chain-updates` without `--block`
- **THEN** the CLI queries chain update state at the last finalized block
- **AND** reports the block hash resolved by the node

#### Scenario: Query chain update state at an explicit block
- **WHEN** the user runs `ccd-wallet node chain-updates --block <BLOCK>`
- **THEN** the CLI parses `<BLOCK>` with the same block-selector syntax used by existing read-only commands
- **AND** reports pending updates and next sequence numbers for the selected block

#### Scenario: Invalid block selector fails actionably
- **WHEN** the user supplies an invalid `--block` value
- **THEN** the CLI exits with a non-zero status
- **AND** identifies the invalid block selector

### Requirement: Chain update state is read from one consistent block
The command SHALL ensure that pending updates and next update sequence numbers describe the same resolved block, including when the requested selector is moving such as `best` or `last-final`.

#### Scenario: Moving selector resolves to one snapshot
- **WHEN** the user queries `best` or `last-final`
- **THEN** the CLI resolves one query to a concrete block hash
- **AND** queries the other dataset at that concrete block hash
- **AND** emits that block hash as the snapshot context

#### Scenario: Snapshot query failure is actionable
- **WHEN** either dataset cannot be queried at the resolved block
- **THEN** the command exits with a non-zero status
- **AND** identifies whether pending updates or next sequence numbers failed

### Requirement: Chain-updates follows existing read-only flags and input behavior
The command SHALL expose `--network <NAME>`, `--node <ENDPOINT>`, `--block <BLOCK>`, `--json`, and `--no-defaults` with the same meanings, conflicts, environment fallback, default resolution, and configured-network selection UI as `node chain-parameters`. It SHALL NOT introduce required positional input or command-specific prompts.

#### Scenario: Active network is used by default
- **WHEN** the user runs `ccd-wallet node chain-updates` without endpoint selectors or `--no-defaults`
- **AND** an active configured network exists
- **THEN** the command queries that network without prompting

#### Scenario: No-defaults uses the existing network picker
- **WHEN** the user runs `ccd-wallet node chain-updates --no-defaults` without an endpoint selector
- **THEN** the CLI uses the existing configured-network selection UI
- **AND** preselects the active network when one exists

#### Scenario: Network and node conflict
- **WHEN** the user supplies both `--network` and `--node`
- **THEN** clap rejects the invocation before querying

### Requirement: Chain update state has human-readable and JSON output
The command SHALL render a human-readable view by default and SHALL emit only one pretty-printed camel-case JSON document when `--json` is supplied. Both forms SHALL include endpoint and block-hash context, all next sequence-number queues, and every pending update returned by the node.

#### Scenario: Human output is operationally scannable
- **WHEN** the user runs `ccd-wallet node chain-updates` without `--json`
- **THEN** the output separates next sequence numbers from pending updates
- **AND** each pending update includes its effective time and typed effect when known

#### Scenario: JSON output is complete and machine-readable
- **WHEN** the user runs `ccd-wallet node chain-updates --json`
- **THEN** stdout contains one valid JSON document without interactive decoration
- **AND** `nextSequenceNumbers` includes every normalized SDK sequence queue
- **AND** `pendingUpdates` contains every update returned by the stream

#### Scenario: No pending updates is explicit
- **WHEN** the node returns no pending updates
- **THEN** human output states that there are no pending updates
- **AND** JSON output contains an empty `pendingUpdates` array

### Requirement: P11 chain update state is visible
The chain-update output SHALL expose the maximum-lock-duration sequence number and typed pending Max Lock Duration effects.

#### Scenario: Maximum lock duration sequence is shown
- **WHEN** the node returns next update sequence numbers for P11
- **THEN** output includes the `maxLockDuration` queue

#### Scenario: Pending maximum lock duration update is shown
- **WHEN** the node returns a pending Max Lock Duration update
- **THEN** output identifies it as `maxLockDuration`
- **AND** displays its proposed duration and effective time

### Requirement: Unknown pending effects remain visible
A pending update whose effect is unknown to the SDK SHALL remain represented in command output rather than causing the entire inspection command to fail.

#### Scenario: Unknown effect is rendered explicitly
- **WHEN** the pending-update stream contains an unknown forward-compatible effect
- **THEN** the CLI includes that pending update with its effective time
- **AND** marks the effect as unknown
- **AND** continues rendering the remaining pending updates
