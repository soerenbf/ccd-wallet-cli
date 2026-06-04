## Why

The repository already documents `token` as the intended home for protocol-level token workflows, and the pinned Concordium Rust SDK branch now exposes the token and lock operations needed to implement that space. Implementing the `token` commands now turns the documented taxonomy into a usable CLI surface and establishes a protocol-aligned command family before adjacent transaction authoring areas grow around it.

## What Changes

- Add a new top-level `token` command space to `ccd-wallet` for protocol-level token workflows.
- Implement token holder and token-admin operations for transfer, mint, burn, allow-list changes, deny-list changes, pause, unpause, admin-role assignment and revocation, and metadata updates, with interactive prompt fallback for all required arguments.
- Implement token query and lock workflows for token inspection, lock creation, funding, lock-controlled sends, returns, cancellation, and lock inspection, with interactive prompt fallback and balance-aware selectors.
- Add shared token command argument parsing, interactive resolution helpers, transaction review, submission, and finalization handling that follows existing CLI network/account resolution patterns.
- Update `docs/commands.md` and the command taxonomy spec so the documented token branch matches the implemented user-facing names, including `show`, `transfer`, `admin-roles`, and `lock show`.
- Improve MetaUpdate transaction finalization output so token transfer and lock events render as concise human-readable lines instead of raw JSON arrays.

## Capabilities

### New Capabilities
- `token-command-execution`: submit and inspect protocol-level token and lock operations through the `token` command space.

### Modified Capabilities
- `command-taxonomy`: the documented token branch changes from planned placeholder names to the concrete implemented command names and includes token and lock inspection commands.

## Impact

- Affected code: `crates/ccd-wallet/src/cli.rs`, `crates/ccd-wallet/src/main.rs`, new `crates/ccd-wallet/src/commands/token/` modules, and any shared command helpers needed for token transaction flows.
- Affected docs/specs: `docs/commands.md`, `openspec/specs/command-taxonomy/spec.md`, and new token command execution specs.
- External dependencies and APIs: the existing pinned `concordium-rust-sdk` / `concordium_base` git branch, especially `protocol_level_tokens::token_client`, `protocol_level_tokens::lock_client`, and the underlying PLT and lock queries and submission APIs.
