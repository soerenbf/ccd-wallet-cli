## MODIFIED Requirements

### Requirement: Configurable Concordium node endpoint
The CLI SHALL allow the user to specify the target Concordium node for node commands using either a named network registered in the config store (`--network <NAME>`) or an explicit gRPC endpoint (`--node <ENDPOINT>`). If neither option is supplied, the CLI SHALL fall back to the active network from the wallet-state store unless `--no-defaults` is supplied. Supplying both `--network` and `--node` SHALL be an error.

The CLI SHALL also allow `network show` to resolve its query node from either a configured network label or an explicit `--node <ENDPOINT>`. Bare `network show` SHALL use the active network in config mode when defaults are allowed, but `network show --node <ENDPOINT>` alone SHALL NOT silently derive additional configured-network context from the active network.

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
- **AND** `--no-defaults` is not supplied
- **THEN** the CLI resolves the endpoint from the active network's stored `node_endpoint` and connects to it

#### Scenario: No-defaults forces explicit network selection
- **WHEN** the user runs a node command without `--network` or `--node`
- **AND** `--no-defaults` is supplied
- **THEN** the CLI prompts the user to choose a configured network explicitly instead of silently using the active network
- **AND** the active network is preselected in the picker when one exists

#### Scenario: Missing active network produces a clear error
- **WHEN** the user runs a node command without `--network` or `--node`
- **AND** no active network is set in the wallet-state store
- **AND** `--no-defaults` is not supplied
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating that the user must provide `--network` or `--node`, or set an active network with `ccd-wallet network use <NAME>`

#### Scenario: Stale active network produces a clear error
- **WHEN** the user runs a node command without `--network` or `--node`
- **AND** the active network named in `wallet_state` does not exist in `config.json`
- **AND** `--no-defaults` is not supplied
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating that the active network is no longer registered

#### Scenario: Providing both flags is rejected
- **WHEN** the user provides both `--network` and `--node` to a node command
- **THEN** the CLI exits with a non-zero status
- **AND** prints an error indicating the flags are mutually exclusive

#### Scenario: Bare network show uses active network in config mode
- **WHEN** the user runs `ccd-wallet network show`
- **AND** an active network is configured
- **THEN** the CLI resolves the query endpoint from the active network's stored node endpoint
- **AND** treats the invocation as config-mode network inspection

#### Scenario: Explicit label plus node override keeps config mode
- **WHEN** the user runs `ccd-wallet network show testnet --node <ENDPOINT>`
- **AND** `testnet` is configured
- **THEN** the CLI queries the explicit endpoint directly
- **AND** still treats the invocation as config-mode network inspection for `testnet`

#### Scenario: Node-only network show does not use active network config implicitly
- **WHEN** the user runs `ccd-wallet network show --node <ENDPOINT>`
- **AND** an active network is configured
- **THEN** the CLI queries the explicit endpoint directly
- **AND** does not implicitly render the active network's configuration unless a label was explicitly supplied
