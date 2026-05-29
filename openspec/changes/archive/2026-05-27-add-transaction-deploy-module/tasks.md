## 1. Transaction command structure and shared rendering

- [x] 1.1 Refactor `crates/ccd-wallet/src/commands/transaction.rs` into a directory-backed module with separate files for `show` and shared transaction-summary rendering helpers.
- [x] 1.2 Extract the existing `transaction show` status rendering logic into a reusable helper without changing the visible `ccd-wallet transaction show` output.
- [x] 1.3 Add or update tests that keep `transaction show` output stable after the refactor and renderer extraction.

## 2. Neutral deploy-module core

- [x] 2.1 Add a non-connect-scoped Rust module for shared deploy-module preparation, including module parsing, module reference derivation, module size calculation, and duplicate-module validation.
- [x] 2.2 Add shared deploy submission and finalization helpers that can be used by both CLI and connect flows while keeping caller-specific prompting and error mapping outside the shared module.
- [x] 2.3 Add or update tests for shared deploy-module preparation, duplicate-module validation behavior, and finalized deploy outcome handling.

## 3. CLI deploy-module command

- [x] 3.1 Extend `crates/ccd-wallet/src/cli.rs` with a `contract deploy-module <file-path>` subcommand and supporting flags including `--no-validate` and `--no-wait`.
- [x] 3.2 Implement the CLI deploy-module flow in the contract command, including file reading, network resolution, signer-capable account resolution, deploy review prompt, approval handling, submission, and default inline waiting for finalization.
- [x] 3.3 Implement `--no-wait` so the deploy command terminates after successful submission with the transaction hash instead of waiting for finalization.
- [x] 3.4 Render finalized deploy outcomes through the shared transaction-summary renderer and include deploy-specific review details before submission.
- [x] 3.5 Add or update tests covering CLI parsing, invalid module-file rejection, user decline behavior, `--no-validate` behavior, `--no-wait` behavior, and inline finalization output.

## 4. Connect deploy-module reuse

- [x] 4.1 Adapt `crates/ccd-wallet/src/commands/connect/deploy_module.rs` to use the neutral shared deploy-module logic for preparation, validation, submission, and finalization data where practical.
- [x] 4.2 Preserve existing connect-specific behavior, including session-bound network/account handling, connect-specific prompt text, JSON-RPC error mapping, and background finalization reporting.
- [x] 4.3 Add or update tests to confirm connect deploy-module behavior remains unchanged while using the shared deploy-module core.
- [x] 4.4 Remove the explicit validation-status line from CLI and connect deploy approval prompts while preserving validation warnings and simulation output.
- [x] 4.5 Simplify duplicate-module validation messaging to `Validation warning: module already exists on chain for this network.`