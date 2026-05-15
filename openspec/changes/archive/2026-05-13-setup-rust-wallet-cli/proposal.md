## Why

This repository is starting from scratch, but the project needs a reliable foundation for a Concordium wallet CLI in Rust. Creating the initial Cargo project, wiring in the Concordium Rust SDK, and proving basic node connectivity now will let future changes focus on wallet features instead of repeated setup work.

## What Changes

- Create the initial Rust Cargo project for a `ccd-wallet` binary.
- Add the core dependencies needed for an async CLI application that integrates with Concordium nodes, including the Concordium Rust SDK.
- Establish a minimal command structure with a read-only node command that verifies SDK-based connectivity.
- Add configuration and error-handling foundations for selecting a Concordium node endpoint.
- Add baseline developer guidance for building, linting, and running the CLI locally.

## Capabilities

### New Capabilities
- `wallet-cli-bootstrap`: Provide the initial Rust project scaffold, binary entrypoint, and dependency baseline for the wallet CLI.
- `node-connectivity`: Allow the CLI to connect to a Concordium node through the Concordium Rust SDK and execute a basic read-only command.

### Modified Capabilities
- None.

## Impact

- Adds the first project structure under Cargo for this repository.
- Introduces foundational Rust dependencies such as `concordium-rust-sdk`, async runtime support, CLI parsing, and diagnostics tooling.
- Establishes the initial CLI surface area and conventions that later wallet commands will build on.
- Creates the baseline documentation and validation workflow for local development.
