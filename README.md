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

### Example: register and select a network

```bash
cargo run -p ccd-wallet -- network add --name testnet --node https://grpc.testnet.concordium.com:20000 --wallet-proxy https://wallet-proxy.testnet.concordium.com
cargo run -p ccd-wallet -- network use testnet
```

### Example: manage seed phrases

```bash
cargo run -p ccd-wallet -- seed add main_seed
cargo run -p ccd-wallet -- seed add generated_seed --random
cargo run -p ccd-wallet -- seed use main_seed
cargo run -p ccd-wallet -- seed show
cargo run -p ccd-wallet -- seed show main_seed
cargo run -p ccd-wallet -- seed remove main_seed
```

`seed add` requests the seed phrase and password through hidden interactive prompts. Do not pass seed phrases or seed passwords as command-line arguments. Use `seed add <LABEL> --random` to generate a new 24-word seed phrase; the generated phrase is temporarily revealed after it is encrypted and stored, and can later be shown again with `seed show <LABEL>`.

`seed use <LABEL>` sets the active seed. `seed show [LABEL]` reveals the decrypted seed phrase after a password prompt. If no label is supplied, `seed show` uses the active seed.

`seed remove <LABEL>` removes a seed after asking you to type the label as confirmation. If the removed seed is active, the active seed selection is cleared.

For safety, `seed show` displays the seed phrase in a temporary terminal view and hides it when you press any key or after 30 seconds, whichever happens first. This reduces terminal scrollback exposure, but it cannot protect against screenshots, terminal/session logging, tmux/screen behavior, or clipboard history if you copy the phrase.

Seed labels may contain only ASCII letters, digits, dash (`-`), and underscore (`_`).

### Example: issue a new identity

```bash
cargo run -p ccd-wallet -- identity new my_identity --provider 1 --network testnet
cargo run -p ccd-wallet -- identity new my_identity --interactive --network testnet
cargo run -p ccd-wallet -- identity new my_identity --provider 1 --seed main_seed --network testnet
cargo run -p ccd-wallet -- identity new my_identity --provider 1 --network testnet --node https://grpc.testnet.concordium.com:20000
```

`identity new <LABEL>` uses the active seed by default, unless `--seed <LABEL>` is supplied. `--provider <ID>` selects an identity provider directly; `--interactive` queries the selected node for available identity providers and opens an arrow-key selector showing both provider names and provider ids. `--network <NAME>` selects the network configuration, including its `wallet_proxy`; `--node <ENDPOINT>` optionally overrides only the node endpoint used for chain queries.

Identity labels follow the same format as seed labels and must be unique within a network.

The identity issuance flow is browser-assisted: the CLI resolves wallet-facing provider metadata from the selected network's `wallet_proxy`, constructs the request, starts a temporary callback receiver on `127.0.0.1`, and opens the identity provider URL in your browser. After verification, the browser returns to the local callback page and the CLI continues automatically.

If loopback callbacks are not available in your environment, use `--manual-callback` to keep the browser handoff fully manual. In manual mode, the CLI prints the browser URL and asks you to paste the final redirect URL containing `#code_uri=` (or `#error=`) back into the terminal.

## Logging

Tracing output is controlled with `RUST_LOG`.

Example:

```bash
RUST_LOG=info cargo run -- node info
```
