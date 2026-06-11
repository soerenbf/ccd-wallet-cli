## 1. Command Surface and Dependencies

- [x] 1.1 Add the Reedline dependency and any small REPL tokenization helper to the Rust workspace.
- [x] 1.2 Add `token compose <PLAN>`, `token compose preview <PLAN>`, and `token compose submit <PLAN>` clap definitions with sender/network/node/no-wait/non-interactive/no-defaults submission flags where applicable.
- [x] 1.3 Route the new compose commands from `commands::token` into a dedicated token composition module.
- [x] 1.4 Update `docs/commands.md` to document the new token compose command family.

## 2. Plan Model and Persistence

- [x] 2.1 Implement a versioned token composition plan model with typed operation variants for all token and lock MetaUpdate operation families.
- [x] 2.2 Implement TOML deserialization and canonical serialization for `version = 1` plans.
- [x] 2.3 Implement atomic plan writes after successful mutations.
- [x] 2.4 Implement human-readable operation-list rendering used by both interactive and top-level preview.
- [x] 2.5 Add unit tests for plan parsing, serialization, preview rendering, and invalid plan errors.

## 3. Lock Reference Handling

- [x] 3.1 Implement lock reference parsing for existing lock IDs, `@`, and explicit `@N` references.
- [x] 3.2 Canonicalize interactive `@` shorthand to the most recent preceding explicit `@N` reference before saving.
- [x] 3.3 Reject unresolved or out-of-range same-plan lock references without mutating the saved plan.
- [x] 3.4 Add tests for single-lock, multi-lock, shorthand, explicit, and invalid lock reference cases.

## 4. Interactive Composer

- [x] 4.1 Implement the Reedline command loop for `token compose <PLAN>` with top-level operation commands, `preview`, `submit`, `help`, `?`, and `exit` commands.
- [x] 4.2 Add grouped command help text and examples for all supported operation commands.
- [x] 4.3 Implement inline argument parsing for operation and submit commands entered inside the composer.
- [x] 4.4 Reuse existing `cliclack` prompt helpers to collect missing operation fields and confirmations after a REPL line has been submitted.
- [x] 4.5 Ensure Ctrl-C exits or interrupts the composer cleanly without corrupting the plan file.
- [x] 4.6 Add tests for REPL command parsing and command dispatch logic independent of terminal IO.

## 5. Operation Resolution and Submission

- [x] 5.1 Implement conversion from plan operations to SDK MetaUpdate operations for token transfers, mint/burn, pause/unpause, allow/deny-list, admin-role, and metadata updates.
- [x] 5.2 Implement conversion from plan lock operations to SDK MetaUpdate operations for lock create, fund, send, return, and cancel.
- [x] 5.3 Resolve sender, network/node context, local account references, token decimals, existing locks, and `@N` lock references during submit.
- [x] 5.4 Implement static validation and chain preflight only for transaction-external invariants; avoid validating balances, permissions, or other conditions that earlier operations in the same composition can change.
- [x] 5.5 Show a final composed-operation confirmation summary before signing and submitting.
- [x] 5.6 Submit the full plan as one MetaUpdate account transaction and report the submitted transaction hash.
- [x] 5.7 Reuse existing finalization waiting behavior unless `--no-wait` is supplied.

## 6. Validation and Integration Tests

- [x] 6.1 Add clap parsing tests for the new top-level compose commands.
- [x] 6.2 Add integration-style tests for previewing saved plans.
- [x] 6.3 Add tests for non-interactive submit errors when required submission details are missing.
- [x] 6.4 Run `cargo fmt` for the Rust workspace.
- [x] 6.5 Run relevant `cargo test` targets for `ccd-wallet`.
- [x] 6.6 Run OpenSpec validation for `add-token-compose`.

## 7. Structured Lock Grant Composition

- [x] 7.1 Introduce shared unresolved lock grant parsing, rendering, validation, and guided prompt helpers.
- [x] 7.2 Make `token lock create --grant` optional and reuse the guided grant composition flow when omitted interactively.
- [x] 7.3 Store token composition lock-create grants as structured account/capability fields and canonicalize inline `--grant ACCOUNT:ROLE,ROLE` into that shape.
- [x] 7.4 Reject unknown lock grant capabilities during compose add and token lock create before saving or submitting.
- [x] 7.5 Add tests for structured grant serialization, inline grant validation, prompted grant helper parsing, and optional `--grant` clap behavior.
- [x] 7.6 Run `cargo fmt`, `cargo test -p ccd-wallet`, and `openspec validate add-token-compose`.

## 8. Portable Network-Bound Plans

- [x] 8.1 Store the target network genesis hash in token composition plans.
- [x] 8.2 Resolve local account labels to raw account addresses before saving added operations.
- [x] 8.3 Reject submit attempts when the selected network genesis hash differs from the plan genesis hash.
- [x] 8.4 Make token compose submit always prompt for sender account selection in interactive mode when no sender is supplied.
- [x] 8.5 Run `cargo fmt`, `cargo test -p ccd-wallet`, and `openspec validate add-token-compose`.
- [x] 8.6 Exit the interactive composer after an in-session submit command completes successfully.
- [x] 8.7 Prompt for explicit network selection and save the network genesis hash when creating a new composition plan.
- [x] 8.8 Validate token existence, lock token configuration, and amount decimals before saving added compose operations.
- [x] 8.9 Present a token selector for compose lock fund/send/return when the token is omitted, using local lock-create tokens or queried on-chain lock tokens.

## 9. Sender References and Lock Recipient Selection

- [x] 9.1 Add `@sender` as a preserved symbolic account reference in composition plans.
- [x] 9.2 Resolve `@sender` to the selected sender account during submit for all account-reference fields.
- [x] 9.3 Present a recipient selector for compose lock send when recipient is omitted, using local lock-create recipients or queried on-chain lock recipients.
- [x] 9.4 Validate explicit lock send recipients against the referenced lock's configured recipient set before saving.
- [x] 9.5 Add tests for `@sender` preservation/resolution and lock recipient validation helpers.
- [x] 9.6 Run `cargo fmt`, `cargo test -p ccd-wallet`, and `openspec validate add-token-compose`.
- [x] 9.7 Reload the plan from disk before executing each non-empty interactive composer command.
- [x] 9.8 Prompt for keep-alive in both composer lock create and token lock create when `--keep-alive` is omitted interactively.
