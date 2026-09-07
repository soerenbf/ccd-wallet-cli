## 1. Shared Lock Metadata Support

- [x] 1.1 Add a shared constrained lock metadata builder that maps optional name and description values to SDK `LockMetadata`, leaves additional fields empty, encodes raw CBOR, and returns no payload when both values are absent.
- [x] 1.2 Extend shared lock configuration construction to accept the constrained metadata input and set `LockConfig.metadata` consistently for standalone and compose callers.
- [x] 1.3 Extend human-readable lock rendering to show absent, recognized, and unrecognized metadata without failing rendering of recipients, controller, or funds.
- [x] 1.4 Add focused shared-helper tests for name-only, description-only, both-fields, empty/absent, empty additional fields, and tolerant malformed-metadata rendering.

## 2. Standalone Lock Creation

- [x] 2.1 Add documented optional `--name` and `--description` arguments to `TokenLockCreateArgs` and extend CLI parsing tests for both values and omission.
- [x] 2.2 Carry the optional metadata fields through prepared lock-create input and resolve omitted fields with empty-allowed `cliclack` prompts only in interactive mode.
- [x] 2.3 Include resolved lock name and description in the standalone pre-submission review and pass them through shared lock configuration construction.
- [x] 2.4 Add lock-create tests covering metadata construction, metadata-free non-interactive behavior, and the absence of an arbitrary metadata authoring option.

## 3. Compose REPL and Plans

- [x] 3.1 Add optional Serde-defaulted, omission-on-serialization name and description fields to compose `LockCreate` operations while retaining plan version 1.
- [x] 3.2 Preserve the new fields through all lock-create matching and transformation paths, including account-reference resolution, validation, and MetaUpdate operation construction.
- [x] 3.3 Parse inline `--name` and `--description` values in REPL lock creation and add empty-allowed prompts for each omitted value.
- [x] 3.4 Add `--name` and `--description` to context completion and composer help, and include supplied metadata in lock-create previews.
- [x] 3.5 Add compose tests for inline parsing, completion/help, metadata plan round trips, metadata-free version-1 compatibility, field preservation, preview output, and encoded submission configuration.

## 4. Documentation and Validation

- [x] 4.1 Update `docs/commands.md` with standalone and compose lock metadata syntax, optional prompt behavior, plan persistence, inspection output, and the name/description-only authoring boundary.
- [x] 4.2 Run Rust formatting and targeted lock, compose, and CLI tests, then run the project-appropriate broader `cargo clippy` and `cargo test` checks.
- [x] 4.3 Run strict OpenSpec validation for `support-lock-metadata` and resolve any artifact validation errors.
