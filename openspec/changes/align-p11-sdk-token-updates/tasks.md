# Tasks

## 1. Shared SDK models and standalone token operations

- [x] 1.1 Audit current construction, signing, lock-config, and event consumers against the pinned SDK changelog/source; verify the inventory includes token commands, composer, transaction rendering, and the Ledger SDK adapter without altering the user's dependency update.
- [x] 1.2 Adapt shared lock construction and all lock-info consumers to tagged SimpleV0 configuration; verify focused tests preserve limited/any-recipient selection, grants, expiry, tokens, keep-alive, memo, and absent/recognized/unrecognized metadata behavior.
- [x] 1.3 Migrate standalone single-token commands to scoped TokenUpdate APIs and update metadata input construction to TokenMetadataUrlDetails without narrowing state inspection; verify transaction payload tests cover a representative transfer plus metadata URL with and without checksum and invalid checksum rejection.
- [x] 1.4 Adapt any affected Ledger SDK payload conversions using SDK serialization; verify adapter tests cover the updated TokenUpdate payload and preserve transaction signing bytes.

## 2. Release naming and standalone lock execution

- [x] 2.1 Rename CLI lock-return definitions, dispatch, prepared values, client calls, capabilities, prompts, and output to release; verify parsing tests accept release commands/grants, reject new return input, and lock submission tests use ReleaseTokens and current validation APIs.
- [x] 2.2 Update docs/commands.md for canonical lock release, release grants, unchanged --token inputs, and updated-node compatibility; verify documented commands match clap help and no implemented lock-return command remains advertised.

## 3. Composer migration and saved-plan compatibility

- [x] 3.1 Replace MetaUpdate resolution/construction with operations::Operations and unscoped TokenUpdate; verify payload tests preserve ordered multi-token and lock operations, same-plan @N lock references, and a single-token plan's unscoped form.
- [x] 3.2 Rename composer commands, help, completions, operation variants, summaries, and grant handling to release; verify REPL tests accept release and reject newly entered return commands/capabilities before saving.
- [x] 3.3 Add backward-reading support at the saved-plan boundary for existing version-1 lock-return tags and return grant capabilities; verify an actual legacy TOML fixture loads/previews/resolves without file mutation, saves canonical release spellings atomically, and preserves version, order, amounts, and references.
- [x] 3.4 Update compose help and docs/commands.md to describe unscoped TokenUpdate submission and saved-plan compatibility; verify no current compose submission is described as MetaUpdate and examples use release.

## 4. Unified TokenUpdate events

- [x] 4.1 Replace MetaUpdate event rendering and fixtures with per-event Upward<OperationEvent> handling; verify scoped and unscoped TokenUpdate summaries render known token/lock events in order with existing one-line formats and lock annotations.
- [x] 4.2 Reuse resilient unknown-event formatting for unified events; verify a fixture with an unknown event between known events retains raw details and continues rendering both known siblings without failure.

## 5. Integration verification

- [x] 5.1 Run cargo fmt --check, cargo check --workspace --all-targets --all-features, cargo test --workspace --all-features, and cargo clippy --workspace --all-targets --all-features; verify success or explicitly record unrelated/environmental blockers with command output.
- [x] 5.2 Review the final diff against all three spec deltas and search affected Rust/docs for obsolete MetaUpdate, lock-return, and ReturnTokens usage; verify remaining legacy strings are confined to compatibility parsing/tests or intentional historical descriptions, with no database/encryption or unrelated TypeScript changes.
