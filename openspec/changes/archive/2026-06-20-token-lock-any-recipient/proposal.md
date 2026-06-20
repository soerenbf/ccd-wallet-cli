## Why

The Concordium node and pinned Rust SDK now support a lock recipient configuration variant of `Any` in addition to an explicit recipient list. `ccd-wallet` still models lock recipients as a concrete list everywhere, which breaks the build and prevents the CLI and token composer from expressing or inspecting the new protocol behavior.

## What Changes

- Update `token lock create` to support any-recipient locks through an explicit `--any-recipient` flag that is mutually exclusive with repeated `--recipient` values.
- Update interactive lock-creation flows to prompt for recipient mode when neither `--any-recipient` nor `--recipient` is supplied.
- Update `token lock show` and lock-creation confirmation output to render the `Any` variant as `any eligible account`.
- Update `token lock send` wording and validation so existing-lock and same-plan any-recipient locks accept any explicit recipient instead of requiring membership in a finite configured list.
- Update `token compose` lock-create plan semantics to support `recipients = "any"` in addition to recipient arrays, while preserving existing array-based plans for limited-recipient locks.
- Update `token compose` interactive lock-send behavior so omitted recipients for any-recipient locks use a free-form account prompt instead of a fixed recipient selector.
- Update command docs, compose help text, and tests to cover the new any-recipient behavior.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `token-command-execution`: Lock create, lock show, and lock send behavior must support the protocol lock-recipient `Any` variant and its user-facing CLI representation.
- `token-composition`: Lock-create plans and lock-send composition flows must support any-recipient locks alongside limited-recipient locks.

## Impact

- Affected Rust code under `crates/ccd-wallet/src/cli.rs`, `crates/ccd-wallet/src/commands/token/lock.rs`, `crates/ccd-wallet/src/commands/token/shared.rs`, and `crates/ccd-wallet/src/commands/token/compose.rs`.
- Affected user-facing documentation in `docs/commands.md` and compose help/output text.
- Affected compatibility surface for saved compose plans: new plans may use `recipients = "any"`, while existing array-based plans remain supported.
- Driven by the updated pinned `concordium-rust-sdk` lock recipient type and lock-client validation behavior.
