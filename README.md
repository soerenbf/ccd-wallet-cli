# ccd-wallet

A Rust command-line wallet project for the Concordium blockchain.

This repository currently provides the initial project bootstrap: a Cargo binary named `ccd-wallet`, foundational CLI structure, and a read-only node inspection command backed by the Concordium Rust SDK.

## Prerequisites

- Rust toolchain (Cargo + rustc)
- Access to a Concordium node gRPC endpoint

## Build

```bash
cargo build
```

## Lint

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
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
cargo run -- node info
```

### Example: explicit local endpoint

```bash
cargo run -- node info --node http://127.0.0.1:20001
```

### Example: public testnet TLS endpoint

```bash
cargo run -- node info --node https://grpc.testnet.concordium.com:20000
```

### Example: environment override

```bash
CCD_WALLET_NODE_ENDPOINT=https://grpc.testnet.concordium.com:20000 \
  cargo run -- node info
```

## Logging

Tracing output is controlled with `RUST_LOG`.

Example:

```bash
RUST_LOG=info cargo run -- node info
```
