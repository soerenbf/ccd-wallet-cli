## ADDED Requirements

### Requirement: Identity issuance can be initiated with a known provider ID
`identity new <LABEL> --provider <provider_id>` SHALL initiate the v1 Concordium identity issuance protocol with the specified identity provider without presenting a selection menu. `<LABEL>` is a user-supplied name for the identity; it must be non-empty, contain only ASCII alphanumeric characters, dashes, or underscores, and be unique within the resolved network.

`--seed <label>` is optional; if omitted, the active seed is used. `--network <label>` selects the network configuration that supplies `wallet_proxy`; `--node <url>` supplies the node used for on-chain data and determines the network by `genesis_hash`. If `--node` is omitted, `--network` or the active network is used.

#### Scenario: Non-interactive issuance with explicit provider and active seed
- **WHEN** the user runs `identity new my-identity --provider 1` with an active seed and active network configured
- **THEN** the CLI resolves the active seed and network, prompts for no further input, constructs the identity request, and proceeds to the browser handoff step

#### Scenario: Non-interactive issuance with explicit seed override
- **WHEN** the user runs `identity new my-identity --provider 1 --seed cold-wallet`
- **THEN** the CLI uses the `cold-wallet` seed instead of the active seed

#### Scenario: Non-interactive issuance with explicit node override
- **WHEN** the user runs `identity new my-identity --provider 1 --node https://node.example.com:20000`
- **THEN** the CLI queries the node's `genesis_hash`
- **AND** selects the configured network with the same `genesis_hash`
- **AND** uses that network's `wallet_proxy` for wallet-facing provider metadata
- **AND** connects to the specified node URL for on-chain cryptographic and provider data

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

#### Scenario: `--network` with mismatched node override
- **WHEN** the user supplies both `--network` and `--node`
- **AND** the node's `genesis_hash` does not match the selected network's `genesis_hash`
- **THEN** the CLI exits with an actionable error before contacting the identity provider

#### Scenario: No active seed configured
- **WHEN** neither `--seed` is provided nor an active seed is set
- **THEN** the CLI exits with a clear error: "No active seed. Run `ccd-wallet seed use <LABEL>` or supply `--seed <LABEL>`."

#### Scenario: No active network configured
- **WHEN** neither `--network` is provided nor an active network is set
- **AND** no `--node` is provided that can be matched to a configured network
- **THEN** the CLI exits with an error explaining that a network is required to resolve `wallet_proxy`

#### Scenario: Node does not match any configured network
- **WHEN** `--node` is provided and its `genesis_hash` does not match any configured network entry
- **THEN** the CLI exits with an actionable error explaining that no configured network matches the supplied node

#### Scenario: Selected network has no wallet proxy configured
- **WHEN** the selected network entry does not contain `wallet_proxy`
- **THEN** the CLI exits with an actionable error before contacting the identity provider

### Requirement: Identity issuance can be initiated interactively
`identity new <LABEL> --interactive` SHALL fetch the list of available identity providers from the active (or specified) node and present an arrow-key selection prompt before proceeding.

#### Scenario: Interactive mode lists available providers
- **WHEN** the user runs `identity new my-identity --interactive` with a reachable node
- **THEN** the CLI displays an interactive selection list showing each provider name and provider identity and lets the user choose with the keyboard without typing a numeric list index

#### Scenario: Interactive mode with seed and network overrides
- **WHEN** the user runs `identity new --interactive --seed main --network testnet`
- **THEN** the CLI uses the specified seed and network for both IP list lookup and issuance request construction

#### Scenario: Node unreachable in interactive mode
- **WHEN** the node is unreachable when fetching the IP list
- **THEN** the CLI exits with an actionable error describing the connection failure

### Requirement: Identity issuance follows the Concordium v1 HTTP protocol
The CLI SHALL implement the v1 issuance protocol:
1. Resolve wallet-facing provider metadata from the selected network's `wallet_proxy`.
2. Build and send the issuance start `GET` request to the provider's `issuanceStart` URL as a preflight step.
3. If the preflight returns a redirect, open the redirect target URL in the system browser (or print it as a fallback).
4. If the preflight does not return a redirect, open the original issuance URL in the system browser.
5. Receive the callback containing `code_uri`.
6. Poll `code_uri` until status is `done` or `error`.
7. Store the resulting identity object.

#### Scenario: Wallet proxy does not provide provider metadata
- **WHEN** the selected `wallet_proxy` does not return metadata for the chosen identity provider
- **THEN** the CLI exits with an actionable error before browser handoff

#### Scenario: Identity provider responds with a redirect
- **WHEN** the IP's issuance start endpoint responds with a redirect
- **THEN** the CLI opens the redirect target URL in the browser

#### Scenario: Identity provider responds with a browser entry page
- **WHEN** the IP's issuance start endpoint responds without a redirect
- **THEN** the CLI opens the original issuance URL in the browser instead of failing early

#### Scenario: Successful issuance flow
- **WHEN** the full issuance flow completes with status `done`
- **THEN** the CLI stores the identity object and prints a success message for the assigned identity label

#### Scenario: Identity provider reports error
- **WHEN** polling returns status `error`
- **THEN** the CLI exits with the error detail from the provider response

#### Scenario: Polling times out
- **WHEN** polling has not resolved within 5 minutes
- **THEN** the CLI exits with an error indicating the identity is still pending and the `code_uri` is stored for later retry

### Requirement: Browser handoff uses manual callback paste
The CLI SHALL print the browser URL and prompt the user to paste the final redirect URL after completing browser-based identity verification. This is the MVP callback receiver.

#### Scenario: CLI prints provider URL and prompts for callback
- **WHEN** the browser handoff step is reached
- **THEN** the CLI prints the URL to open and instructs the user to paste the final redirect URL

#### Scenario: Pasted URL contains code_uri fragment
- **WHEN** the user pastes a URL of the form `<redirect_uri>#code_uri=<url>`
- **THEN** the CLI extracts `<url>` and proceeds to poll it

#### Scenario: Pasted URL contains error fragment
- **WHEN** the user pastes a URL containing `#error=<detail>`
- **THEN** the CLI exits with the error detail

#### Scenario: Pasted URL is unrecognisable
- **WHEN** the user pastes a URL that contains neither `#code_uri=` nor `#error=`
- **THEN** the CLI exits with an error asking the user to paste the correct URL
