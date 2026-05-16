# network-show Specification

## Purpose
TBD - created by archiving change add-network-show-command. Update Purpose after archive.
## Requirements
### Requirement: Show configured network details with consensus information
The CLI SHALL provide a `network show` command that can inspect a configured network alias and render both its stored configuration and live consensus information from a queried node. In config mode, the command SHALL show `Network configuration` before `Consensus (<node endpoint>)`.

#### Scenario: Show active network by default
- **WHEN** the user runs `ccd-wallet network show`
- **AND** an active network is configured
- **THEN** the CLI resolves that configured network
- **AND** renders `Network configuration` first
- **AND** renders `Consensus (<node endpoint>)` using that network's configured node endpoint

#### Scenario: Show explicit network label
- **WHEN** the user runs `ccd-wallet network show testnet`
- **AND** `testnet` is configured
- **THEN** the CLI renders a `Network configuration` section for `testnet`
- **AND** renders a `Consensus (<node endpoint>)` section using `testnet`'s configured node endpoint

#### Scenario: Config mode includes stored network details
- **WHEN** the user runs `ccd-wallet network show testnet`
- **THEN** the `Network configuration` section includes the configured network name
- **AND** includes the configured node endpoint
- **AND** includes the configured wallet proxy
- **AND** includes the configured genesis hash

### Requirement: Show node-derived network matches for a raw endpoint
The CLI SHALL support `network show --node <ENDPOINT>` as a node-only inspection mode. In node-only mode, the command SHALL query consensus information from the explicit endpoint, derive the observed genesis hash, and show matching configured network aliases, if any, before showing `Consensus (<node endpoint>)`.

#### Scenario: Node-only mode shows matching configured aliases
- **WHEN** the user runs `ccd-wallet network show --node <ENDPOINT>`
- **AND** the queried node reports genesis hash `abc`
- **AND** configured aliases `testnet` and `other_testnet` both use genesis hash `abc`
- **THEN** the CLI renders `Network matches (abc)` before consensus details
- **AND** lists `testnet` and `other_testnet` together with their configured node endpoints

#### Scenario: Node-only mode shows no configured match
- **WHEN** the user runs `ccd-wallet network show --node <ENDPOINT>`
- **AND** the queried node reports genesis hash `abc`
- **AND** no configured network uses genesis hash `abc`
- **THEN** the CLI renders `Network match (abc)` with an explicit no-match summary before consensus details
- **AND** still renders the queried node's consensus information

### Requirement: Config mode supports explicit query-node override
The CLI SHALL support `network show <LABEL> --node <ENDPOINT>` as a diagnostic override mode. In this mode, the command SHALL keep the selected network configuration as the primary entity rendered while querying consensus from the explicit endpoint.

#### Scenario: Label plus node override keeps config-first rendering
- **WHEN** the user runs `ccd-wallet network show testnet --node <ENDPOINT>`
- **AND** `testnet` is configured
- **THEN** the CLI renders `Network configuration` for `testnet`
- **AND** does NOT switch into node-only `Network match(es)` rendering
- **AND** renders `Consensus (<ENDPOINT>)` using the explicit endpoint

#### Scenario: Override mode warns on configured-vs-observed mismatch
- **WHEN** the user runs `ccd-wallet network show testnet --node <ENDPOINT>`
- **AND** `testnet` is configured with genesis hash `abc`
- **AND** the queried node reports genesis hash `def`
- **THEN** the CLI renders the configured network section for `testnet`
- **AND** renders the queried consensus information
- **AND** includes a warning indicating that the configured network does not match the observed node genesis hash

### Requirement: Consensus output is human-oriented and endpoint-scoped
The command SHALL render consensus information under a `Consensus (<node endpoint>)` heading using a curated human-oriented summary rather than a raw debug dump.

#### Scenario: Consensus section identifies the queried endpoint
- **WHEN** the CLI shows consensus information for any `network show` invocation
- **THEN** the consensus section heading includes the node endpoint that was queried

#### Scenario: Consensus section includes observed network identity
- **WHEN** the CLI shows consensus information for any `network show` invocation
- **THEN** the output includes the observed genesis hash returned by the queried node
- **AND** includes additional human-meaningful consensus fields selected by the command

