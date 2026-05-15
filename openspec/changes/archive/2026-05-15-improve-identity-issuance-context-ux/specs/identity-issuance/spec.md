## MODIFIED Requirements

### Requirement: Identity issuance can be initiated with a known provider ID
`identity new <LABEL> --provider <provider_id>` SHALL initiate the v1 Concordium identity issuance protocol with the specified identity provider without presenting a selection menu. `<LABEL>` is a user-supplied name for the identity; it must be non-empty, contain only ASCII alphanumeric characters, dashes, or underscores, and be unique within the resolved network.

`--seed <label>` is optional; if omitted, the active seed is used. `--network <label>` selects the network configuration that supplies `wallet_proxy`; `--node <url>` supplies the node used for on-chain data and determines the network by `genesis_hash`. If `--node` is omitted, `--network` or the active network is used. In interactive mode, missing non-secret arguments such as identity label, seed label, network label, node endpoint, or provider id SHALL be requested through `cliclack` prompts where that flow supports them. In `--non-interactive` mode, missing required values SHALL produce actionable errors instead of prompt fallback. When `--no-defaults` is supplied, the CLI SHALL NOT silently use active seed or active network state and SHALL instead request explicit selection from a picker with the active entity preselected when one exists.

Before proceeding with password entry, provider contact, or browser handoff, the CLI SHALL display the effective context using the resolved seed label and network label/node endpoint in a compact aligned block when those values were derived from defaults, explicit overrides, or inference. If the user just selected the value in an interactive picker during the same run, the CLI SHALL NOT immediately restate it.

#### Scenario: Non-interactive issuance with explicit provider and active seed
- **WHEN** the user runs `identity new my-identity --provider 1` with an active seed and active network configured
- **THEN** the CLI resolves the active seed and network
- **AND** displays `seed: <label>` and `network: <label> @ <node-endpoint>`
- **AND** constructs the identity request and proceeds to the browser handoff step

#### Scenario: Missing label is prompted interactively
- **WHEN** the user runs `identity new --provider 1`
- **AND** `--non-interactive` is not supplied
- **THEN** the CLI prompts for the missing identity label using `cliclack`
- **AND** continues issuance with the entered label

#### Scenario: Non-interactive issuance with explicit seed override
- **WHEN** the user runs `identity new my-identity --provider 1 --seed cold-wallet`
- **THEN** the CLI uses the `cold-wallet` seed instead of the active seed
- **AND** displays `seed: cold-wallet` before continuing

#### Scenario: Non-interactive issuance with explicit node override
- **WHEN** the user runs `identity new my-identity --provider 1 --node https://node.example.com:20000`
- **THEN** the CLI queries the node's `genesis_hash`
- **AND** selects the configured network with the same `genesis_hash`
- **AND** uses that network's `wallet_proxy` for wallet-facing provider metadata
- **AND** connects to the specified node URL for on-chain cryptographic and provider data
- **AND** displays the resolved network label together with `https://node.example.com:20000`

#### Scenario: Missing required value in non-interactive mode errors
- **WHEN** the user runs identity issuance with `--non-interactive`
- **AND** a required non-secret value is missing
- **THEN** the CLI does not prompt
- **AND** exits with an actionable error indicating what must be provided

#### Scenario: Label already exists for this network
- **WHEN** an identity with the same label already exists on the resolved network
- **THEN** the CLI exits with an error before any network call is made

#### Scenario: Same label on another network is allowed
- **WHEN** an identity with the same label exists on a different network
- **THEN** the CLI continues with issuance on the resolved network

#### Scenario: Invalid label format
- **WHEN** the supplied label contains whitespace, dots, or non-ASCII characters
- **THEN** the CLI exits with a clear validation error

#### Scenario: `--network` with matching node override
- **WHEN** the user supplies both `--network` and `--node`
- **AND** the node's `genesis_hash` matches the selected network's `genesis_hash`
- **THEN** the CLI uses the named network to resolve `wallet_proxy`
- **AND** uses the explicit node override for chain queries
- **AND** displays the effective context before continuing

#### Scenario: `--network` with mismatched node override
- **WHEN** the user supplies both `--network` and `--node`
- **AND** the node's `genesis_hash` does not match the selected network's `genesis_hash`
- **THEN** the CLI exits with an actionable error before contacting the identity provider

#### Scenario: No active seed configured
- **WHEN** neither `--seed` is provided nor an active seed is set
- **AND** `--non-interactive` is supplied
- **THEN** the CLI exits with a clear error: "No active seed. Run `ccd-wallet seed use <LABEL>` or supply `--seed <LABEL>`."

#### Scenario: No-defaults forces explicit seed and network selection
- **WHEN** the user runs identity issuance with `--no-defaults`
- **AND** does not supply `--seed`, `--network`, or `--node`
- **THEN** the CLI prompts for explicit seed and network selection instead of silently using active state
- **AND** the active seed and active network are preselected in their respective pickers when they exist

#### Scenario: No active network configured
- **WHEN** neither `--network` is provided nor an active network is set
- **AND** no `--node` is provided that can be matched to a configured network
- **AND** `--non-interactive` is supplied
- **THEN** the CLI exits with an error explaining that a network is required to resolve `wallet_proxy`

#### Scenario: Node does not match any configured network
- **WHEN** `--node` is provided and its `genesis_hash` does not match any configured network entry
- **THEN** the CLI exits with an actionable error explaining that no configured network matches the supplied node

#### Scenario: Selected network has no wallet proxy configured
- **WHEN** the selected network entry does not contain `wallet_proxy`
- **THEN** the CLI exits with an actionable error before contacting the identity provider

### Requirement: Identity issuance can be initiated interactively
`identity new <LABEL> --interactive` SHALL fetch the list of available identity providers from the active (or specified) node and present an arrow-key selection prompt before proceeding. The command SHALL display the resolved seed/network context before provider selection.

#### Scenario: Interactive mode lists available providers
- **WHEN** the user runs `identity new my-identity --interactive` with a reachable node
- **THEN** the CLI displays any derived seed/network context first as a compact aligned block
- **AND** displays an interactive selection list showing each provider name and provider identity and lets the user choose with the keyboard without typing a numeric list index

#### Scenario: Interactive mode with seed and network overrides
- **WHEN** the user runs `identity new --interactive --seed main --network testnet`
- **THEN** the CLI uses the specified seed and network for both IP list lookup and issuance request construction
- **AND** displays `seed: main` and `network: testnet @ <node-endpoint>` in a compact aligned block before provider selection

#### Scenario: Interactive mode prompts missing values
- **WHEN** the user runs `identity new --interactive`
- **AND** `--non-interactive` is not supplied
- **THEN** the CLI may prompt for missing non-secret values needed to continue the flow using `cliclack`

#### Scenario: Single available provider is auto-selected
- **WHEN** the user reaches provider selection
- **AND** exactly one identity provider is available on the selected network
- **THEN** the CLI selects that provider automatically
- **AND** does not render a one-item selector

#### Scenario: Node unreachable in interactive mode
- **WHEN** the node is unreachable when fetching the IP list
- **THEN** the CLI exits with an actionable error describing the connection failure

### Requirement: Browser handoff uses manual callback paste
The CLI SHALL keep manual callback paste available as an explicit callback transport selected by flag. In manual mode, the CLI prints the browser URL and prompts the user to paste the final redirect URL after completing browser-based identity verification. The CLI SHALL NOT automatically switch from loopback mode to manual paste after a loopback timeout. All identity issuance input prompts SHALL use `cliclack`.

#### Scenario: CLI prints provider URL and prompts for callback in manual mode
- **WHEN** the browser handoff step uses manual callback mode
- **THEN** the CLI prints the URL to open
- **AND** uses a `cliclack` input prompt to request the final redirect URL

#### Scenario: Pasted URL contains code_uri fragment
- **WHEN** the user pastes a URL of the form `<redirect_uri>#code_uri=<url>`
- **THEN** the CLI extracts `<url>` and proceeds to poll it

#### Scenario: Pasted URL contains error fragment
- **WHEN** the user pastes a URL containing `#error=<detail>`
- **THEN** the CLI exits with the error detail

#### Scenario: Pasted URL is unrecognisable
- **WHEN** the user pastes a URL that contains neither `#code_uri=` nor `#error=`
- **THEN** the CLI exits with an error asking the user to paste the correct URL
