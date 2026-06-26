# identity-issuance Specification

## Purpose
TBD - created by archiving change add-identity-issuance. Update Purpose after archive.
## Requirements
### Requirement: Ledger identity issuance requires explicit export approval
When `identity new` targets a Ledger key source, the CLI SHALL treat Ledger secret export as an explicit security-mode change rather than a normal signing step. Interactive operation SHALL display a warning that the required identity issuance secrets will be exported into host process memory temporarily and SHALL require an explicit confirmation before any Ledger export command is sent. In `--non-interactive` mode, the command SHALL require the explicit opt-in flag `--allow-ledger-secret-export` and SHALL otherwise fail with an actionable error. Ledger-backed identity issuance SHALL require a Ledger app export path that provides every recovery-critical issuance secret deterministically from the Ledger.

#### Scenario: Interactive Ledger issuance requires confirmation
- **WHEN** the user runs `identity new my-identity --provider 1 --seed ledger-main`
- **AND** `ledger-main` resolves to an enrolled Ledger key source
- **THEN** the CLI displays a warning that identity issuance secrets will be exported temporarily into host process memory
- **AND** the CLI requires explicit confirmation before sending any Ledger export command

#### Scenario: Non-interactive Ledger issuance requires explicit opt-in
- **WHEN** the user runs Ledger-backed identity issuance with `--non-interactive`
- **AND** the command does not include `--allow-ledger-secret-export`
- **THEN** the CLI does not prompt
- **AND** exits with an actionable error explaining that Ledger secret export must be explicitly allowed with `--allow-ledger-secret-export`

#### Scenario: Ledger app export protocol lacks recovery-critical material
- **WHEN** the user runs Ledger-backed identity issuance
- **AND** the connected Concordium Ledger app cannot provide deterministic signature blinding randomness together with PRFKey and IDCredSec
- **THEN** the CLI exits before contacting the identity provider
- **AND** the error explains that Concordium Ledger app 5.5.0 or newer is required
- **AND** no pending identity row is written

#### Scenario: Declined Ledger export fails before pending identity storage
- **WHEN** the user declines the explicit Ledger export approval
- **THEN** the CLI exits without calling the identity provider
- **AND** no pending identity row is written

### Requirement: Identity issuance can be initiated with a known provider ID
`identity new <LABEL> --provider <provider_id>` SHALL initiate the v1 Concordium identity issuance protocol with the specified identity provider without presenting a selection menu. `<LABEL>` is a user-supplied name for the identity; it must be non-empty, contain only ASCII alphanumeric characters, dashes, or underscores, and be unique within the resolved network.

`--seed <label>` remains the user-facing flag for selecting the key source label and MAY refer either to a stored seed phrase or an enrolled Ledger signer owner. If omitted, the active key source is used. `--network <label>` selects the network configuration that supplies `wallet_proxy`; `--node <url>` supplies the node used for on-chain data and determines the network by `genesis_hash`. If `--node` is omitted, `--network` or the active network is used. In interactive mode, missing non-secret arguments such as identity label, key-source label, network label, node endpoint, or provider id SHALL be requested through `cliclack` prompts where that flow supports them. In `--non-interactive` mode, missing required values SHALL produce actionable errors instead of prompt fallback. When `--no-defaults` is supplied, the CLI SHALL NOT silently use active key-source or active network state and SHALL instead request explicit selection from a picker with the active entity preselected when one exists.

Before proceeding with local password entry, Ledger export approval, provider contact, or browser handoff, the CLI SHALL display the effective context using the resolved key-source label and network label/node endpoint in a compact aligned block when those values were derived from defaults, explicit overrides, or inference. If the user just selected the value in an interactive picker during the same run, the CLI SHALL NOT immediately restate it.

#### Scenario: Non-interactive issuance with explicit provider and active seed
- **WHEN** the user runs `identity new my-identity --provider 1` with an active seed and active network configured
- **THEN** the CLI resolves the active key source and network
- **AND** displays `key source: <label>` and `network: <label> @ <node-endpoint>`
- **AND** constructs the identity request and proceeds to the browser handoff step

#### Scenario: Missing label is prompted interactively
- **WHEN** the user runs `identity new --provider 1`
- **AND** `--non-interactive` is not supplied
- **THEN** the CLI prompts for the missing identity label using `cliclack`
- **AND** continues issuance with the entered label

#### Scenario: Non-interactive issuance with explicit key-source override
- **WHEN** the user runs `identity new my-identity --provider 1 --seed cold-wallet`
- **THEN** the CLI uses `cold-wallet` as the selected key source instead of the active key source
- **AND** displays `key source: cold-wallet` before continuing

#### Scenario: Non-interactive issuance with explicit Ledger key-source override
- **WHEN** the user runs `identity new my-identity --provider 1 --seed ledger-main --non-interactive`
- **AND** `ledger-main` resolves to an enrolled Ledger key source
- **AND** the command includes `--allow-ledger-secret-export`
- **THEN** the CLI uses `ledger-main` as the selected key source
- **AND** displays `key source: ledger-main` before continuing

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

#### Scenario: No active key source configured
- **WHEN** neither `--seed` is provided nor an active key source is set
- **AND** `--non-interactive` is supplied
- **THEN** the CLI exits with a clear error explaining that an active key source must be set or `--seed <LABEL>` must be supplied

#### Scenario: No-defaults forces explicit key-source and network selection
- **WHEN** the user runs identity issuance with `--no-defaults`
- **AND** does not supply `--seed`, `--network`, or `--node`
- **THEN** the CLI prompts for explicit key-source and network selection instead of silently using active state
- **AND** the active key source and active network are preselected in their respective pickers when they exist

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
`identity new <LABEL> --interactive` SHALL fetch the list of issuable identity providers from the selected network's `wallet_proxy` and present an arrow-key selection prompt before proceeding. The command SHALL display the resolved key-source/network context before provider selection. The selected provider SHALL also be validated against the active (or specified) node before issuance continues.

#### Scenario: Interactive mode lists available providers
- **WHEN** the user runs `identity new my-identity --interactive` with a reachable node
- **THEN** the CLI displays any derived key-source/network context first as a compact aligned block
- **AND** displays an interactive selection list showing each provider name and provider identity and lets the user choose with the keyboard without typing a numeric list index

#### Scenario: Interactive mode with key-source and network overrides
- **WHEN** the user runs `identity new --interactive --seed main --network testnet`
- **THEN** the CLI uses the specified key source and network for both wallet-proxy provider lookup and issuance request construction
- **AND** displays `key source: main` and `network: testnet @ <node-endpoint>` in a compact aligned block before provider selection

#### Scenario: Interactive mode prompts missing values
- **WHEN** the user runs `identity new --interactive`
- **AND** `--non-interactive` is not supplied
- **THEN** the CLI may prompt for missing non-secret values needed to continue the flow using `cliclack`

#### Scenario: Single available provider is auto-selected
- **WHEN** the user reaches provider selection
- **AND** exactly one identity provider is available on the selected network
- **THEN** the CLI selects that provider automatically
- **AND** does not render a one-item selector

#### Scenario: Wallet proxy or node unavailable in interactive mode
- **WHEN** the wallet proxy is unreachable when fetching the issuance provider list
- **THEN** the CLI exits with an actionable error describing the connection failure
- **WHEN** the node is unreachable when validating the selected provider or loading issuance inputs
- **THEN** the CLI exits with an actionable error describing the connection failure

### Requirement: Identity issuance follows the Concordium v1 HTTP protocol
The CLI SHALL implement the v1 issuance protocol by orchestrating node queries, local storage, and the dedicated identity provider crate:
1. Resolve wallet-facing provider metadata from the selected network's `wallet_proxy`.
2. Unlock the selected key source's local storage domain once.
3. If the selected key source is a seed, derive the issuance material from the seed-backed wallet and use the seed DEK for encrypted identity private payload storage.
4. If the selected key source is a Ledger signer owner, verify the connected Ledger matches that signer owner, complete the explicit export approval flow, use the Ledger app 5.5.0+ purpose-based identity credential creation export to derive IDCredSec, PRFKey, and signature blinding randomness, and use the Ledger owner vault DEK for encrypted identity private payload storage.
5. Build and send the issuance start `GET` request to the provider's `issuanceStart` URL as a preflight step.
6. If the preflight returns a redirect, open the redirect target URL in the system browser (or print it as a fallback).
7. If the preflight does not return a redirect, open the original issuance URL in the system browser.
8. Receive the callback containing `code_uri`.
9. Store `code_uri` only inside the encrypted identity private payload.
10. Poll `code_uri` until status is `done` or `error`.
11. Store the resulting identity object only inside the encrypted identity private payload.

#### Scenario: Wallet proxy does not provide provider metadata
- **WHEN** the selected `wallet_proxy` does not return metadata for the chosen identity provider
- **THEN** the CLI exits with an actionable error before browser handoff

#### Scenario: Requested provider is unavailable for issuance
- **WHEN** the user supplies `--provider <ID>`
- **AND** the selected network's `wallet_proxy` does not list that provider
- **THEN** the CLI exits with an actionable error before browser handoff

#### Scenario: Wallet proxy and node provider registries disagree
- **WHEN** the selected `wallet_proxy` lists the chosen provider
- **AND** the selected node does not register that provider on-chain
- **THEN** the CLI exits with an actionable error before browser handoff

#### Scenario: Identity provider responds with a redirect
- **WHEN** the IP's issuance start endpoint responds with a redirect
- **THEN** the CLI opens the redirect target URL in the browser

#### Scenario: Identity provider responds with a browser entry page
- **WHEN** the IP's issuance start endpoint responds without a redirect
- **THEN** the CLI opens the original issuance URL in the browser instead of failing early

#### Scenario: Successful seed-backed issuance flow
- **WHEN** the full issuance flow completes with status `done` for a seed-backed key source
- **THEN** the CLI stores the identity object encrypted under the owning seed password domain
- **AND** prints a success message for the assigned identity label

#### Scenario: Successful Ledger-backed issuance flow
- **WHEN** the full issuance flow completes with status `done` for a Ledger-backed key source after explicit export approval
- **AND** the connected Concordium Ledger app supports the 5.5.0+ purpose-based identity credential creation export protocol
- **THEN** the CLI stores the identity object encrypted under the owning Ledger signer-owner password domain
- **AND** prints a success message for the assigned identity label

#### Scenario: Identity provider reports error
- **WHEN** polling returns status `error`
- **THEN** the CLI deletes the pending identity row and its encrypted private payload
- **AND** exits with the error detail from the provider response

#### Scenario: Polling times out
- **WHEN** polling has not resolved within 5 minutes
- **THEN** the CLI exits with an error indicating the identity is still pending
- **AND** the stored `code_uri` remains encrypted under the owning seed password domain

#### Scenario: No plaintext identity private data is stored during issuance
- **WHEN** identity issuance stores `code_uri` or identity object data
- **THEN** neither value is written to SQLite as plaintext

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

### Requirement: Browser handoff uses loopback callback by default
The CLI SHALL use a local loopback callback receiver by default for identity issuance. The receiver SHALL bind only to `127.0.0.1` on an ephemeral port, generate a single-use nonce-bearing callback path, and provide that URL as the issuance `redirect_uri`.

#### Scenario: Loopback callback session provides redirect URI
- **WHEN** identity issuance reaches browser handoff setup
- **THEN** the CLI starts a local loopback callback receiver
- **AND** uses a redirect URI of the form `http://127.0.0.1:<port>/callback/<nonce>` for the issuance request

#### Scenario: Browser callback fragment is bridged to CLI
- **WHEN** the browser lands on the loopback callback URL with `#code_uri=<url>` in the fragment
- **THEN** the local callback page reads the fragment in the browser
- **AND** posts it back to the loopback receiver
- **AND** the CLI extracts `<url>` and proceeds to poll it

#### Scenario: Browser callback error is bridged to CLI
- **WHEN** the browser lands on the loopback callback URL with `#error=<detail>` in the fragment
- **THEN** the local callback page posts the fragment back to the loopback receiver
- **AND** the CLI exits with the provider error detail

#### Scenario: Callback page is minimal
- **WHEN** the browser loads the loopback callback page
- **THEN** the page displays only minimal status text needed to complete the handoff and tell the user they may close the tab

#### Scenario: Loopback callback is single-use
- **WHEN** the loopback receiver accepts a callback result
- **THEN** it resolves the waiting issuance flow
- **AND** refuses or ignores subsequent completion attempts for the same nonce
- **AND** shuts down the local callback listener

#### Scenario: Unexpected callback path is rejected
- **WHEN** a request uses the wrong callback path or nonce
- **THEN** the loopback receiver rejects the request without completing the issuance flow

#### Scenario: Explicit manual mode bypasses loopback
- **WHEN** the user selects manual callback mode with the explicit flag
- **THEN** the CLI does not start a loopback callback receiver
- **AND** uses manual callback paste for browser handoff

#### Scenario: Loopback timeout does not automatically fall back
- **WHEN** loopback callback mode times out before receiving a callback
- **THEN** the CLI exits with an actionable error explaining that the user can retry with the manual callback flag

#### Scenario: Pending identity row is inserted after code URI is received
- **WHEN** loopback mode starts and browser handoff begins
- **THEN** no pending identity row is inserted yet
- **WHEN** the callback receiver obtains a `code_uri`
- **THEN** the CLI inserts the pending identity row as in the existing issuance flow

