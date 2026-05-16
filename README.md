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

`network reset [NAME]` prunes wallet-local identities and accounts for a network partition while keeping configured aliases intact. It accepts either a configured network name or `--genesis-hash <HASH>`. In interactive mode, omitted targets open a partition-oriented selector that shows rows like `6f8c…ab12 - testnet, staging-testnet` or `6f8c…ab12 (orphan)` together with identity/account counts.

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
cargo run -p ccd-wallet -- account list
cargo run -p ccd-wallet -- account list --network all --status pending
cargo run -p ccd-wallet -- account list --seed daily_seed --show-addresses
cargo run -p ccd-wallet -- account rename my_account main_account
cargo run -p ccd-wallet -- account rename --show-addresses --seed daily_seed
```

`account new [LABEL]` creates a normal Concordium account by deriving credential material from the selected seed and issued identity, submitting a credential deployment to the resolved node, and storing the local account record. If `--identity <LABEL>` is omitted, the CLI prompts you to choose a usable identity unless `--non-interactive` is supplied. Usable identities must belong to the selected seed and network and must not be expired. If a selected identity is still pending, the wallet checks the stored encrypted `code_uri` with the identity provider before account creation proceeds.

By default, `account new` waits until the credential deployment finalizes and then stores the new account address encrypted under the owning seed's password domain in a structured account private payload. Use `--no-wait` to return after successful submission; the account remains pending locally for future lazy finalization checks.

`account list` is human-oriented and scope-aware. By default it uses the active seed and active network, but you can broaden the scope with `--seed all` and/or `--network all`, then narrow with `--status <pending|finalized>`. Account addresses remain hidden unless you request `--show-addresses`, which prompts for the necessary seed password material to decrypt them.

`account rename` supports either an explicit old label or, when omitted in interactive mode, a fuzzy searchable selector across stored accounts. `account rename --show-addresses` requires a concrete seed scope, supplied either through `--seed <LABEL>` or through an interactive seed-selection prompt, before the selector can display decrypted addresses.

If loopback callbacks are not available in your environment, use `--manual-callback` to keep the browser handoff fully manual. In manual mode, the CLI prints the browser URL and asks you to paste the final redirect URL containing `#code_uri=` (or `#error=`) back into the terminal using the same interactive prompt framework.

Development note: this version consolidates the SQLite schema into a new initial migration. Existing development `wallet.db` files from earlier schema versions should be deleted and recreated.

## Logging

Tracing output is controlled with `RUST_LOG`.

Example:

```bash
RUST_LOG=info cargo run -- node info
```
