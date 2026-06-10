## 1. Command Surface and Dependencies

- [ ] 1.1 Add the Reedline dependency and any small REPL tokenization helper to the Rust workspace.
- [ ] 1.2 Add `token compose <PLAN>`, `token compose preview <PLAN>`, and `token compose submit <PLAN>` clap definitions with sender/network/node/no-wait/non-interactive/no-defaults submission flags where applicable.
- [ ] 1.3 Route the new compose commands from `commands::token` into a dedicated token composition module.
- [ ] 1.4 Update `docs/commands.md` to document the new token compose command family.

## 2. Plan Model and Persistence

- [ ] 2.1 Implement a versioned token composition plan model with typed operation variants for all token and lock MetaUpdate operation families.
- [ ] 2.2 Implement TOML deserialization and canonical serialization for `version = 1` plans.
- [ ] 2.3 Implement atomic plan writes after successful mutations.
- [ ] 2.4 Implement human-readable operation-list rendering used by both interactive and top-level preview.
- [ ] 2.5 Add unit tests for plan parsing, serialization, preview rendering, and invalid plan errors.

## 3. Lock Reference Handling

- [ ] 3.1 Implement lock reference parsing for existing lock IDs, `@`, and explicit `@N` references.
- [ ] 3.2 Canonicalize interactive `@` shorthand to the most recent preceding explicit `@N` reference before saving.
- [ ] 3.3 Reject unresolved or out-of-range same-plan lock references without mutating the saved plan.
- [ ] 3.4 Add tests for single-lock, multi-lock, shorthand, explicit, and invalid lock reference cases.

## 4. Interactive Composer

- [ ] 4.1 Implement the Reedline command loop for `token compose <PLAN>` with `add`, `preview`, `submit`, `help`, `?`, and `exit` commands.
- [ ] 4.2 Add command help text and examples for all supported add operations.
- [ ] 4.3 Implement inline argument parsing for add and submit commands entered inside the composer.
- [ ] 4.4 Reuse existing `cliclack` prompt helpers to collect missing operation fields and confirmations after a REPL line has been submitted.
- [ ] 4.5 Ensure Ctrl-C exits or interrupts the composer cleanly without corrupting the plan file.
- [ ] 4.6 Add tests for REPL command parsing and command dispatch logic independent of terminal IO.

## 5. Operation Resolution and Submission

- [ ] 5.1 Implement conversion from plan operations to SDK MetaUpdate operations for token transfers, mint/burn, pause/unpause, allow/deny-list, admin-role, and metadata updates.
- [ ] 5.2 Implement conversion from plan lock operations to SDK MetaUpdate operations for lock create, fund, send, return, and cancel.
- [ ] 5.3 Resolve sender, network/node context, local account references, token decimals, existing locks, and `@N` lock references during submit.
- [ ] 5.4 Implement static validation and chain preflight only for transaction-external invariants; avoid validating balances, permissions, or other conditions that earlier operations in the same composition can change.
- [ ] 5.5 Show a final composed-operation confirmation summary before signing and submitting.
- [ ] 5.6 Submit the full plan as one MetaUpdate account transaction and report the submitted transaction hash.
- [ ] 5.7 Reuse existing finalization waiting behavior unless `--no-wait` is supplied.

## 6. Validation and Integration Tests

- [ ] 6.1 Add clap parsing tests for the new top-level compose commands.
- [ ] 6.2 Add integration-style tests for previewing saved plans.
- [ ] 6.3 Add tests for non-interactive submit errors when required submission details are missing.
- [ ] 6.4 Run `cargo fmt` for the Rust workspace.
- [ ] 6.5 Run relevant `cargo test` targets for `ccd-wallet`.
- [ ] 6.6 Run OpenSpec validation for `add-token-compose`.
