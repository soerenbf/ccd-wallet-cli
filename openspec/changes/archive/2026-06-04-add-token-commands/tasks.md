## 1. Command surface and shared token infrastructure

- [x] 1.1 Add the top-level `token` clap command tree in `crates/ccd-wallet/src/cli.rs` with subcommands for `show`, `transfer`, `mint`, `burn`, `allow-list`, `deny-list`, `pause`, `unpause`, `admin-roles`, `metadata`, and `lock`.
- [x] 1.2 Add a `commands::token` module tree and wire it into `crates/ccd-wallet/src/main.rs` and `crates/ccd-wallet/src/commands/mod.rs`.
- [x] 1.3 Implement shared token command helpers for network resolution, signer account resolution/unlocking, token amount parsing from on-chain decimals, review output, submission, and optional finalization waiting.

## 2. Token query, holder, and token-admin command implementation

- [x] 2.1 Implement `token show` using the pinned SDK branch's token-info query support and render a human-readable token summary.
- [x] 2.2 Implement `token transfer`, `token mint`, and `token burn` using the pinned SDK branch's `TokenClient` APIs.
- [x] 2.3 Implement `token allow-list add/remove`, `token deny-list add/remove`, `token pause`, and `token unpause` using `TokenClient`.
- [x] 2.4 Implement `token admin-roles assign/revoke` with protocol-near role parsing and `TokenClient` admin-role APIs.
- [x] 2.5 Implement `token metadata update` using `TokenClient` metadata update support.

## 3. Token lock command implementation

- [x] 3.1 Implement `token lock create`, including lock configuration parsing and submission through the SDK lock creation support.
- [x] 3.2 Implement `token lock fund`, `token lock send`, `token lock return`, and `token lock cancel` using `LockClient`.
- [x] 3.3 Implement `token lock show` using the SDK lock-info query path and render a human-readable lock summary.

## 4. Documentation and verification

- [x] 4.1 Update `docs/commands.md` so the `token` branch is marked implemented and uses the final user-facing names `show`, `transfer`, `admin-roles`, and `lock show`.
- [x] 4.2 Add or update clap parsing and command-focused tests for the new `token` command space, including representative holder/admin commands and lock commands.
- [x] 4.3 Run the relevant Rust formatting, lint, and test commands for the touched code paths and address any failures.

## 5. Interactive UX refinements (post-initial-implementation)

- [x] 5.1 Make all token mutation command arguments optional with prompt fallback; error in `--non-interactive` mode when required values are missing.
- [x] 5.2 Add `always_prompt_account` support so all token mutation commands always present the account selector.
- [x] 5.3 Implement `resolve_token_from_balances` for `token transfer`: query account token balances and populate an interactive selector with available amounts as hints.
- [x] 5.4 Implement `resolve_lock_token` for lock fund/send/return: populate an interactive selector from the lock's configured token set with locked/available balance hints.
- [x] 5.5 Implement `resolve_token_amount` with balance hints for lock fund (available) and lock send/return (locked).
- [x] 5.6 Change lock fund/send/return `token_id` from positional to `--token` named flag.
- [x] 5.7 Render MetaUpdate transaction events as human-readable one-line summaries: `Transfer <amount> <token>: <from> -> <to>` with optional `(locked @ <lock-id>)`, `Lock created: <id>`, `Lock destroyed: <id>`, and compact inline JSON fallback for unknown events.
