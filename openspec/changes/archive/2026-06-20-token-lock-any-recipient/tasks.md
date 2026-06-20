## 1. Direct token lock command support

- [x] 1.1 Update `crates/ccd-wallet/src/cli.rs` lock-create args to add `--any-recipient`, make it mutually exclusive with `--recipient`, and document the new wording for lock-send and lock-show flows.
- [x] 1.2 Update `crates/ccd-wallet/src/commands/token/shared.rs` helpers to model lock recipients as any-or-limited, including lock-config construction and human-readable rendering as `any eligible account`.
- [x] 1.3 Update `crates/ccd-wallet/src/commands/token/lock.rs` create flow to support explicit any-recipient creation and interactive recipient-mode prompting.
- [x] 1.4 Verify `token lock send` behavior and wording for any-recipient locks while preserving limited-recipient validation semantics.

## 2. Token compose any-recipient support

- [x] 2.1 Update `crates/ccd-wallet/src/commands/token/compose.rs` plan model and TOML serialization to support `recipients = "any"` alongside recipient arrays.
- [x] 2.2 Update compose lock-create parsing and prompting to support `--any-recipient` and interactive recipient-mode selection.
- [x] 2.3 Update compose lock-send recipient lookup, prompting, and validation so any-recipient locks use free-form recipient input and limited-recipient locks keep selector-based validation.
- [x] 2.4 Update compose preview/help text to describe any-recipient locks as `any eligible account` and document the new authoring syntax.

## 3. Docs and verification

- [x] 3.1 Update `docs/commands.md` to describe any-recipient lock creation and the related token compose behavior.
- [x] 3.2 Add or update Rust tests covering direct lock creation, lock rendering, compose plan parsing/serialization, compose recipient prompting, and any-recipient validation behavior.
- [x] 3.3 Run targeted Rust verification for the changed lock command paths (`cargo fmt`, `cargo test`, and `cargo build` or equivalent targeted checks) and fix any regressions.
