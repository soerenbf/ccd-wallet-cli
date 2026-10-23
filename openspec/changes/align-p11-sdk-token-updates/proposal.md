# Proposal

## Why

The updated Concordium Rust SDK on `release/p11` removes MetaUpdate and changes lock operations, configuration, metadata inputs, and transaction events. Align the wallet with the pinned SDK so existing token and lock workflows remain usable.

## What Changes

- **BREAKING**: Rename canonical CLI and composer `lock return` commands and grant capability names to `release`; retain legacy spelling only when reading saved version-1 plans.
- Replace MetaUpdate construction with unscoped TokenUpdate operations for composition and locks; use scoped TokenUpdate APIs for standalone single-token commands.
- Adapt token metadata updates to URL and optional checksum inputs through `TokenMetadataUrlDetails`.
- Construct and inspect tagged `LockConfig::SimpleV0` configurations without changing supported lock creation fields.
- Render unified, individually forward-compatible TokenUpdate operation events, retaining concise known-event summaries and resilient unknown-event output.
- Keep existing version-1 composition plans readable, canonicalize legacy release spellings on save, and update command documentation and tests.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `token-command-execution`: Canonical release commands, scoped/unscoped submission, metadata inputs, and unified TokenUpdate event rendering.
- `token-composition`: Unscoped TokenUpdate composition, release naming, and saved-plan compatibility.
- `command-taxonomy`: Document release commands and current TokenUpdate composition terminology.

## Impact

Rust command definitions and token command modules, transaction rendering, relevant SDK construction/signing integrations, regression tests, and `docs/commands.md`. The dependency update already present in Cargo.toml and Cargo.lock remains the baseline; no new dependency is needed. No database, encryption, or TypeScript API changes are intended. Nodes must support the updated P11 transaction model; no legacy MetaUpdate wire fallback is included.
