## Why

Protocol-level lock configurations can now carry immutable user-facing metadata, but `ccd-wallet` always creates locks without it and does not display it during lock inspection. Users need a constrained way to set and review the SDK metadata model's name and description fields in both standalone lock creation and composed MetaUpdate workflows.

## What Changes

- Add optional `--name` and `--description` inputs to `token lock create`.
- Prompt interactively for an optional lock name and description when either input is omitted, while keeping both optional in non-interactive mode.
- Encode only the supplied name and description into the SDK `LockMetadata` model; do not expose arbitrary additional metadata fields.
- Include lock metadata in standalone creation review output before confirmation.
- Support the same inline inputs and optional prompts for `lock create` in the token compose REPL.
- Persist optional lock name and description values in composition plans, include them in previews, and encode them when plans are submitted.
- Decode and display recognized lock name and description values in `token lock show` without making unrecognized raw metadata prevent inspection of the rest of the lock.
- Keep lock creation without metadata and existing version-1 composition plans compatible.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `token-command-execution`: Extend standalone lock creation and inspection with constrained lock metadata authoring, review, and display behavior.
- `token-composition`: Extend compose REPL lock creation, saved plans, previews, and submission with optional lock metadata.

## Impact

- Affects the Rust CLI argument model and token lock command flow in `crates/ccd-wallet/src/cli.rs` and `crates/ccd-wallet/src/commands/token/lock.rs`.
- Adds shared typed lock metadata encoding and rendering behavior in `crates/ccd-wallet/src/commands/token/shared.rs`.
- Extends compose parsing, prompting, completion, plan serialization, preview, and operation resolution in `crates/ccd-wallet/src/commands/token/compose.rs`.
- Uses the pinned Concordium Rust SDK's `LockMetadata` model and raw-CBOR encoding support; no new dependency is required.
- Updates command documentation and the affected OpenSpec capability requirements.
