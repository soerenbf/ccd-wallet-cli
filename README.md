# ccd-wallet

A Rust command-line wallet project for the Concordium blockchain.

This repository currently provides the initial project bootstrap: a Cargo binary named `ccd-wallet`, foundational CLI structure, and a read-only node inspection command backed by the Concordium Rust SDK.

## Prerequisites

- Rust toolchain (Cargo + rustc)
- Access to a Concordium node gRPC endpoint

## Workspace layout

This repository is now a Cargo workspace:

- `crates/ccd-wallet-core`: shared library crate for storage, config, and wallet cryptography
- `crates/ccd-wallet-identity-provider`: shared library crate for identity issuance request construction, provider HTTP helpers, and callback handling
- `crates/ccd-wallet`: CLI binary crate

## Build

```bash
cargo build --workspace
```

## Lint

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Run

By default, the CLI will target a local Concordium node endpoint:

- `http://localhost:20000`

The project is configured to support both:

- local non-TLS gRPC endpoints such as `http://127.0.0.1:20001`
- public TLS gRPC endpoints such as `https://grpc.testnet.concordium.com:20000`

You can override the node endpoint with either:

- `--node <endpoint>` on the command line
- `CCD_WALLET_NODE_ENDPOINT=<endpoint>` in the environment

### Example: local node

```bash
cargo run -p ccd-wallet -- node info
```

### Example: explicit local endpoint

```bash
cargo run -p ccd-wallet -- node info --node http://127.0.0.1:20001
```

### Example: public testnet TLS endpoint

```bash
cargo run -p ccd-wallet -- node info --node https://grpc.testnet.concordium.com:20000
```

### Example: environment override

```bash
CCD_WALLET_NODE_ENDPOINT=https://grpc.testnet.concordium.com:20000 \
  cargo run -p ccd-wallet -- node info
```

### Example: register and manage networks

```bash
cargo run -p ccd-wallet -- network add --name testnet --node https://grpc.testnet.concordium.com:20000 --wallet-proxy https://wallet-proxy.testnet.concordium.com
cargo run -p ccd-wallet -- network list
cargo run -p ccd-wallet -- network show
cargo run -p ccd-wallet -- network show testnet
cargo run -p ccd-wallet -- network show --node https://grpc.testnet.concordium.com:20000
cargo run -p ccd-wallet -- network show testnet --node https://grpc.testnet.concordium.com:20000
cargo run -p ccd-wallet -- network rename testnet staging
cargo run -p ccd-wallet -- network reset staging
cargo run -p ccd-wallet -- network reset --genesis-hash <GENESIS_HASH>
cargo run -p ccd-wallet -- network delete staging
cargo run -p ccd-wallet -- network delete staging other-alias
cargo run -p ccd-wallet -- network use staging
cargo run -p ccd-wallet -- network use
```

Most user-facing setup flows can now prompt for missing non-secret values interactively. Use `--non-interactive` to disable prompt fallback and require values on the command line. Use `--no-defaults` on flows that would otherwise silently use the active seed or active network to force an explicit picker selection instead. When a picker has only one valid option, the CLI selects it automatically instead of showing a one-item selector. Existing-entity selection flows such as `seed use` and `network use` use selectors instead of asking you to retype a known label.

`network show [NAME]` inspects a configured network and prints `Network configuration` followed by `Consensus (<node endpoint>)`. Bare `network show` uses the active network by default unless `--no-defaults` is supplied. `network show --node <ENDPOINT>` switches into node-only mode: it queries the explicit endpoint, derives the observed genesis hash, prints `Network match(es) (<genesis hash>)` with any matching configured aliases, and then prints consensus details. `network show [NAME] --node <ENDPOINT>` keeps config mode but uses the explicit node as a diagnostic override, warning if the observed genesis hash does not match the configured network.

### TypeScript workspace and connect client

This repository also contains a pnpm-managed TypeScript workspace for browser-facing packages. The first package is `@ccd-wallet/connect-client`, an environment-flexible client for the `ccd-wallet connect` WebSocket JSON-RPC API.

```bash
pnpm install
pnpm --filter @ccd-wallet/connect-client build
pnpm --filter @ccd-wallet/connect-client test
```

The package lives at `packages/ccd-wallet-connect-client` and currently supports pairing and explicit account requests by network.

The workspace also includes `packages/ccd-wallet-connect-example`, a Vite + React integration reference showing how a web application can pair with the wallet and request an account for a target network.

### Example: pair a browser dApp with the wallet

```bash
cargo run -p ccd-wallet -- connect
cargo run -p ccd-wallet -- connect --bind 127.0.0.1:22771
```

`connect` starts a temporary browser-facing WebSocket session on localhost. It is not a background daemon: the wallet is connectable only while the command is running, and pressing Ctrl-C closes the session.

The browser API uses a single WebSocket channel with JSON-RPC 2.0 messages. The initial API is intentionally narrow and supports session pairing plus explicit account requests by network only. Transaction proposal, signing, submission, and governance-key pairing are out of scope for this first connected-wallet flow.

A browser dApp begins pairing by sending a `pair` request that includes a six-digit challenge code. The CLI shows the browser origin and asks you to type the same challenge to approve that the visible browser tab matches the terminal request. Successful pairing returns only a session token. The application can then request an approved account address for a target network by supplying that session token together with the network genesis hash.

Account labels, seed labels, and the full wallet inventory are not exposed through the browser API. Only one browser session can be paired at a time; additional pairing requests are rejected while a session is active.

### Example: inspect transaction status and details

```bash
cargo run -p ccd-wallet -- transaction show 0fda6e284f9cd4429c6f76fd1bf6179aad4fa1bb218fe5ec8ad33916bf84a833 --network testnet
cargo run -p ccd-wallet -- transaction show 0fda6e284f9cd4429c6f76fd1bf6179aad4fa1bb218fe5ec8ad33916bf84a833 --node https://grpc.testnet.concordium.com:20000
cargo run -p ccd-wallet -- transaction show 0fda6e284f9cd4429c6f76fd1bf6179aad4fa1bb218fe5ec8ad33916bf84a833
```

`transaction show <HASH>` is a read-only node-backed inspection command. It does not require the wallet to store transactions locally. The command resolves the query node from `--network <NAME>`, `--node <ENDPOINT>`, or the active network by default, and `--no-defaults` forces explicit network selection.

The output is detailed by default. It renders stable fields such as `Transaction`, `Queried via`, `Status`, block hash, block time, outcome, type, and energy in a human-oriented format. For committed and finalized transactions it then renders the concrete block-item summary variant explicitly:
- account transactions show static fields such as sender and cost, then either `Events: <N>` or `Reject reason:`
- credential deployments show static fields such as credential type, address, and registration id
- chain updates show stable fields such as effective time and update type, then a `Payload:` section
- token creation shows static `CreatePlt` fields, then `Events: <N>`

JSON is used only for the nested non-static payloads within those variants. Received transactions do not print empty nested detail sections.

If the selected node does not know the hash, `transaction show` prints `Status: absent` instead of surfacing a raw not-found error. Because transaction hashes are network-dependent, `absent` can also mean you queried the wrong network or node.

### Example: manage seed phrases

```bash
cargo run -p ccd-wallet -- seed add main_seed
cargo run -p ccd-wallet -- seed add generated_seed --random
cargo run -p ccd-wallet -- seed list
cargo run -p ccd-wallet -- seed rename main_seed daily_seed
cargo run -p ccd-wallet -- seed use daily_seed
cargo run -p ccd-wallet -- seed use
cargo run -p ccd-wallet -- seed show
cargo run -p ccd-wallet -- seed show daily_seed
cargo run -p ccd-wallet -- seed delete daily_seed
```

`seed add` requests the seed phrase and password through hidden interactive prompts. If the seed label is omitted, the CLI prompts for it unless `--non-interactive` is supplied. Do not pass seed phrases or seed passwords as command-line arguments. Use `seed add <LABEL> --random` to generate a new 24-word seed phrase; the generated phrase is temporarily revealed after it is encrypted and stored, and can later be shown again with `seed show <LABEL>`.

`seed list` displays configured seed labels without requiring a password. `seed rename [OLD_LABEL] [NEW_LABEL]` changes a seed's label while preserving the underlying seed id and encrypted payload; if the old label is omitted in interactive mode, the CLI lets you select the source seed first.

`seed use [LABEL]` sets the active seed. If the label is omitted, the CLI opens a seed selector instead of asking you to type the label, unless `--non-interactive` is supplied. `seed show [LABEL]` reveals the decrypted seed phrase after a password prompt. If no label is supplied, `seed show` uses the active seed by default, or forces an explicit picker when `--no-defaults` is supplied.

`seed delete [LABEL]` deletes a seed after asking you to type the label as confirmation. If the label is omitted, the CLI opens a selector unless `--non-interactive` is supplied. Deleting a seed also removes all identities and accounts owned by that seed. If the deleted seed is active, the active seed selection is cleared.

`network reset [NAME]` prunes wallet-local identities, accounts, imported-account vault data, and governance-vault data for a network partition while keeping configured aliases intact. It accepts either a configured network name or `--genesis-hash <HASH>`. In interactive mode, omitted targets open a partition-oriented selector that shows rows like `6f8c…ab12 - testnet, staging-testnet` or `6f8c…ab12 (orphan)` together with identity/account counts.

`network delete [NAME]...` removes one or more configured network aliases only; it does not prune identities or accounts. If labels are omitted in interactive mode, the CLI opens an alias multiselect. When deleting aliases would orphan remaining local network data, the CLI warns and points you to `network reset` for cleanup.

For safety, `seed show` displays the seed phrase in a temporary terminal view and hides it when you press any key or after 30 seconds, whichever happens first. This reduces terminal scrollback exposure, but it cannot protect against screenshots, terminal/session logging, tmux/screen behavior, or clipboard history if you copy the phrase.

Seed labels may contain only ASCII letters, digits, dash (`-`), and underscore (`_`).

### Example: issue, inspect, and rename identities

```bash
cargo run -p ccd-wallet -- identity new my_identity --provider 1 --network testnet
cargo run -p ccd-wallet -- identity new my_identity --interactive --network testnet
cargo run -p ccd-wallet -- identity new my_identity --provider 1 --seed daily_seed --network testnet
cargo run -p ccd-wallet -- identity new my_identity --provider 1 --network testnet --node https://grpc.testnet.concordium.com:20000
cargo run -p ccd-wallet -- identity new my_identity --provider 1 --network testnet --no-wait
cargo run -p ccd-wallet -- identity list
cargo run -p ccd-wallet -- identity list --network testnet --provider 1 --status done
cargo run -p ccd-wallet -- identity rename my_identity primary_identity
cargo run -p ccd-wallet -- identity rename
```

`identity new [LABEL]` uses the active seed by default, unless `--seed <LABEL>` is supplied. If the label is omitted, the CLI prompts for it unless `--non-interactive` is supplied. `--provider <ID>` selects an identity provider directly; if no provider is supplied, the CLI can prompt you to choose one interactively. `--interactive` queries the selected node for available identity providers and opens an arrow-key selector showing both provider names and provider ids. `--network <NAME>` selects the network configuration, including its `wallet_proxy`; `--node <ENDPOINT>` optionally overrides only the node endpoint used for chain queries. Use `--no-defaults` to force explicit selection instead of silently using the active seed or active network.

Identity labels follow the same format as seed labels and must be unique within a network.

The identity issuance flow is browser-assisted: the CLI resolves wallet-facing provider metadata from the selected network's `wallet_proxy`, constructs the request, starts a temporary callback receiver on `127.0.0.1`, and opens the identity provider URL in your browser. Before continuing, it shows the effective context as `seed: <label>` and `network: <label> @ <node-endpoint>`. After verification, the browser returns to the local callback page and the CLI continues automatically. By default the CLI waits for the provider to finish issuing the identity. Use `--no-wait` to return after the callback `code_uri` is stored; the identity remains pending locally and can be checked lazily when it is later used for account creation.

Identity private payloads, including the issuance `code_uri` and issued identity object, are encrypted in SQLite under the owning seed's password domain. Identity labels and public metadata such as network, provider id, status, timestamps, and identity expiry remain plaintext. Expiry metadata is used to avoid account creation attempts with expired identities.

`identity list` is human-oriented and scope-aware. By default it uses the active seed and active network, but you can broaden the scope with `--seed all` and/or `--network all`, then narrow with filters such as `--provider <ID>` and `--status <pending|done|expired>`. `identity rename` supports either an explicit old label or, when omitted in interactive mode, a fuzzy searchable selector across stored identities that includes seed/network metadata.

### Example: create, inspect, and rename accounts

```bash
cargo run -p ccd-wallet -- account new my_account --identity primary_identity --network testnet
cargo run -p ccd-wallet -- account new my_account --identity primary_identity --network testnet --no-wait
cargo run -p ccd-wallet -- account new my_account --identity primary_identity --seed daily_seed --network testnet
cargo run -p ccd-wallet -- account import genesis ~/Developer/Concordium/concordium-node/concordium-node/test-runs/p11-simple/genesis/accounts/baker-0.json --network local --label baker-0
cargo run -p ccd-wallet -- account list
cargo run -p ccd-wallet -- account list --network all --status pending
cargo run -p ccd-wallet -- account list --seed daily_seed --show-addresses
cargo run -p ccd-wallet -- account rename my_account main_account
cargo run -p ccd-wallet -- account rename --show-addresses --seed daily_seed
```

`account import genesis <FILE> --network <NAME> --label <LABEL>` imports a single genesis account JSON bundle as an account on the resolved network. Imported accounts are not tied to a seed or identity; their secret material is encrypted in an imported accounts vault scoped to the network genesis hash. The vault is created automatically on first import for that network and reused for later imports. If `--label` is omitted in interactive mode, the CLI prompts for a label using the JSON filename stem as the suggested value. Labels must follow normal account-label rules and remain unique across all accounts on the network, whether seed-derived or imported. Imported account addresses remain hidden by default and are only decrypted when address display is explicitly requested.

`account new [LABEL]` creates a normal Concordium account by deriving credential material from the selected seed and issued identity, submitting a credential deployment to the resolved node, and storing the local account record. If `--identity <LABEL>` is omitted, the CLI prompts you to choose a usable identity unless `--non-interactive` is supplied. Usable identities must belong to the selected seed and network and must not be expired. If a selected identity is still pending, the wallet checks the stored encrypted `code_uri` with the identity provider before account creation proceeds.

By default, `account new` waits until the credential deployment finalizes and then stores the new account address encrypted under the owning seed's password domain in a structured account private payload. Use `--no-wait` to return after successful submission; the account remains pending locally for future lazy finalization checks.

`account list` is human-oriented and scope-aware. By default it uses the resolved network but shows accounts across all seeds on that network unless you explicitly narrow with `--seed <LABEL>` (or `--seed all`) and/or `--status <pending|finalized>`. Account rows use a bracket-first format such as `[daily_seed] my_account` or `[imported] baker-0`, optionally followed by the decrypted address in parentheses when `--show-addresses` is enabled. Compact suffix metadata still shows the network and, for seed-derived accounts, provider/identity/credential information. Because address display is seed-scoped, `account list --show-addresses` requires an explicit `--seed <LABEL>`.

`account rename` supports either an explicit old label or, when omitted in interactive mode, a fuzzy searchable selector across stored accounts. `account rename --show-addresses` requires a concrete seed scope, supplied either through `--seed <LABEL>` or through an interactive seed-selection prompt, before the selector can display decrypted addresses.

If loopback callbacks are not available in your environment, use `--manual-callback` to keep the browser handoff fully manual. In manual mode, the CLI prints the browser URL and asks you to paste the final redirect URL containing `#code_uri=` (or `#error=`) back into the terminal using the same interactive prompt framework.

### Example: import, inspect, and remove governance keys

```bash
cargo run -p ccd-wallet -- governance keys import ~/Developer/Concordium/concordium-node/concordium-node/test-runs/p11-simple/genesis/update-keys/root-key-0.json --network local
cargo run -p ccd-wallet -- governance keys import --dir ~/Developer/Concordium/concordium-node/concordium-node/test-runs/p11-simple/genesis/update-keys --network local
cargo run -p ccd-wallet -- governance keys list
cargo run -p ccd-wallet -- governance keys remove
cargo run -p ccd-wallet -- governance keys remove <VERIFY_KEY>
cargo run -p ccd-wallet -- governance keys remove --all --network local
```

`governance keys import <FILE>` imports a single governance keypair JSON file into a governance vault scoped by the selected network's genesis hash. `governance keys import --dir <DIR>` imports all recognized keypair files in the directory and ignores aggregate snapshot files such as `governance-keys.json`. The governance vault is created automatically on first import for a network and has its own password domain separate from seeds and imported accounts.

`governance keys list` is an unlock-and-query command: it uses the active network by default, first checks whether a governance vault exists for the resolved network, then prompts for the governance vault password only when stored governance keys are present. After unlock, it decrypts stored governance keys, queries live chain parameters from the selected node, and renders tag-first rows sorted for operational use: `level 2`, then `level 1`, then `root`, then `not authorized`. By default the command abbreviates verify keys to a compact form such as `1234...5678`; use `--show-full` to render full verify keys. Root and level 1 rows summarize governance-key update authority, while level 2 rows summarize the update families the key can currently sign (for example `protocol` or `create plt`). The wallet does not trust aggregate governance snapshots as persistent authority metadata.

`governance keys remove <VERIFY_KEY>` removes one imported governance key by its public key. `governance keys remove` without a public key opens an interactive fuzzy multiselect after vault unlock, reuses the same authorization-aware tag-first summaries as `governance keys list`, and abbreviates displayed verify keys to a compact form such as `1234...5678` so multiple removals are easier to review. `governance keys remove --all` removes all governance keys for the selected network. If the governance vault becomes empty after removal, it is deleted automatically.

### Example: sign and submit governance updates

```bash
cargo run -p ccd-wallet -- governance update --json update.json --network local --effective-time 0
cargo run -p ccd-wallet -- governance update --serialized <HEX> --key <VERIFY_KEY> --timeout 5m
cargo run -p ccd-wallet -- governance update --serialized <HEX> --blind --sign-as protocol --key <VERIFY_KEY> --sequence-number 7 --no-wait
```

`governance update` signs and submits governance update payloads using keys from the governance key vault. Use `--json <FILE>` for a decoded JSON payload file, or `--serialized <HEX>` for Concordium-serialized update payload bytes. In interactive mode, omitting the value after `--json` prompts for pasted JSON, and omitting the value after `--serialized` prompts for pasted hex. As a Create PLT convenience only, JSON payloads may provide `initializationParameters` as JSON instead of hex; the wallet converts that field to Concordium CBOR before signing.

For known payloads, the wallet decodes the update type, derives the authorization family and sequence-number queue, queries live chain parameters, and prompts for signers if `--key <VERIFY_KEY>` is omitted. Signer prompts reuse the governance key rows from `governance keys list`, sort keys authorized for the current update first, and preselect authorized keys up to the required threshold when the threshold is known.

For unknown serialized payloads, use `--blind` only after independently reviewing the payload with trusted tooling. Blind signing emits warnings. If the payload cannot be decoded and `--sign-as <AUTH_FAMILY>` is omitted, you must provide explicit `--key <VERIFY_KEY>` signer flags and `--sequence-number <N>`. `--sign-as` is a helper for cases where you know the authorization family; examples include `protocol`, `create-plt`, `root`, and `level1`.

Effective time and timeout inputs accept relative durations (`5m`, `30m`, `1h`, `15d`), RFC3339 datetimes, or unix seconds. If effective time is omitted in interactive mode, the prompt defaults to `0` (immediate). If timeout is omitted, the prompt defaults to five minutes from now for immediate updates, or five minutes before a scheduled effective time, displayed as RFC3339. For scheduled updates, timeout must not be after effective time. By default, the command waits for finalization; use `--no-wait` to return after successful submission.

Development note: this version consolidates the SQLite schema into a new initial migration. Existing development `wallet.db` files from earlier schema versions should be deleted and recreated.

## Logging

Tracing output is controlled with `RUST_LOG`.

Example:

```bash
RUST_LOG=info cargo run -- node info
```
