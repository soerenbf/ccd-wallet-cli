## 1. CLI surface and command wiring

- [x] 1.1 Add a top-level `transaction` command with a `show` subcommand and a required transaction hash argument in `crates/ccd-wallet/src/cli.rs`.
- [x] 1.2 Wire the new command through `crates/ccd-wallet/src/main.rs` and `crates/ccd-wallet/src/commands/mod.rs`.
- [x] 1.3 Reuse or extend existing endpoint-resolution helpers so `transaction show` supports `--network`, `--node`, active-network defaults, and `--no-defaults` consistently.

## 2. Transaction status query and rendering

- [x] 2.1 Implement a new `commands/transaction.rs` module that parses the supplied hash and queries the resolved node with the Concordium Rust SDK transaction-status API.
- [x] 2.2 Handle SDK `Received`, `Committed`, and `Finalized` transaction statuses and map node `NotFound` to a successful `Status: absent` render.
- [x] 2.3 Render the stable transaction shell for each status, including transaction hash, queried context, lifecycle status, and per-block fields for committed/finalized summaries.
- [x] 2.4 Serialize committed/finalized summary details into pretty JSON for the result section instead of using raw debug output.

## 3. Validation and documentation

- [x] 3.1 Add focused tests for command parsing, endpoint resolution behavior, absent handling, and status rendering paths.
- [x] 3.2 Update `README.md` with `transaction show` examples and behavior notes, including the node/network context and detailed output expectations.
- [x] 3.3 Run the relevant Rust test and lint commands to confirm the new command integrates cleanly with the existing CLI.
