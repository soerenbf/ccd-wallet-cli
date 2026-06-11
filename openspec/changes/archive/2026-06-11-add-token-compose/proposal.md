## Why

Token and protocol-level lock workflows often require several related MetaUpdate operations that should be reviewed and submitted atomically. Today users must submit one command per operation, which is cumbersome for lock setup/funding flows and prevents composing dependent operations such as creating a lock and funding it in the same transaction.

## What Changes

- Add a `ccd-wallet token compose <PLAN>` interactive composer that creates or continues a serialized token composition plan on disk.
- Add `ccd-wallet token compose preview <PLAN>` to list the operations recorded in a plan without resolving or constructing the final chain transaction.
- Add `ccd-wallet token compose submit <PLAN> --sender <LABEL> ...` to resolve, confirm, and submit a plan as one protocol-level token MetaUpdate transaction.
- Support all user-facing token and lock MetaUpdate operation families in composition plans: transfers, mint/burn, pause/unpause, allow/deny-list updates, admin-role updates, metadata updates, lock create/fund/send/return/cancel.
- Use a Reedline-backed command loop for the composer while continuing to use `cliclack` prompts for missing operation fields and final confirmations.
- Persist the plan after each successful `add` command; cancelled or invalid additions do not mutate the saved plan.
- Support in-plan references to locks created by earlier operations through explicit `@N` references, with `@` accepted interactively as shorthand for the most recent created lock and canonicalized to `@N` on disk.

## Capabilities

### New Capabilities
- `token-composition`: Defines interactive token composition plans, plan preview, plan submission, supported operation families, autosave behavior, and same-plan lock references.

### Modified Capabilities
- `token-command-execution`: Add the `token compose` command family to the token command space and require composed plans to submit as protocol-level MetaUpdate transactions.
- `command-taxonomy`: Document `token compose`, `token compose preview`, and `token compose submit` under the token command space.

## Impact

- Rust CLI command definitions and token command orchestration in `crates/ccd-wallet`.
- New token composition plan model, TOML serialization, parser/renderer, Reedline command loop, and MetaUpdate operation builder.
- Existing account-reference, network/node resolution, token amount parsing, lock configuration, confirmation, finalization, and transaction rendering helpers.
- Cargo dependencies: add a line-editor dependency such as `reedline` and any small parsing helper needed for REPL command tokenization.
- Documentation: update `docs/commands.md` for the new command surface.
