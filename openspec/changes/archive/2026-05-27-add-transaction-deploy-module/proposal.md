## Why

The wallet can already review and submit deploy-module transactions through the browser connect JSON-RPC flow, but there is no first-class CLI command for the same workflow. Adding `ccd-wallet contract deploy-module <file-path>` lets terminal-first users deploy modules directly while reusing the same deploy semantics and producing a more consistent review and finalization experience.

## What Changes

- Add a top-level `ccd-wallet contract deploy-module <file-path>` command for smart contract module deployment from a local module file.
- Make the new command resolve network and signing account through the wallet CLI, run duplicate-module validation by default with `--no-validate` opt-out, show a concise warning when the same module already exists on chain, and require explicit user approval before submission.
- Make the CLI deploy flow wait for finalization inline by default, while supporting `--no-wait` to exit immediately after submission with the transaction hash.
- Make the finalized CLI output use the transaction summary rendering path already used by `transaction show` so deploy outcomes remain visually consistent.
- Extract deploy-module logic into a neutral shared module so the new CLI command and the existing connect deploy flow can reuse module parsing, derived module reference handling, validation behavior, submission, and finalization reporting.

## Capabilities

### New Capabilities
- `contract-module-deployment`: Deploy a smart contract module from the CLI with review, default duplicate-module validation, optional validation opt-out via `--no-validate`, submission, and either inline finalization reporting or immediate return via `--no-wait`.

### Modified Capabilities
- `connect-module-deployment`: remove the approval-prompt requirement to display whether validation was requested.

## Impact

- Affected Rust CLI surface in `crates/ccd-wallet/src/cli.rs`, `crates/ccd-wallet/src/commands/contract/`, and transaction summary rendering helpers.
- New contract deploy-module command implementation plus a neutral shared Rust module for deploy preparation, validation, submission, and finalization/result rendering.
- Existing connect deploy flow in `crates/ccd-wallet/src/commands/connect/deploy_module.rs` will be adapted to reuse the shared deploy behavior where practical.
- Tests will need coverage for the new CLI command behavior, shared deploy logic, and reused transaction summary rendering.