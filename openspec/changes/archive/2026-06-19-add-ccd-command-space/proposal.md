## Why

The CLI currently has no dedicated user-facing command space for native CCD transfer authoring. Adding a `ccd` command space now gives plain CCD transfer flows a clear home before more detached transaction lifecycle features are introduced elsewhere.

## What Changes

- Add a top-level `ccd` command space for native CCD account-transaction authoring.
- Add `ccd transfer` for simple CCD transfers with an optional memo.
- Add `ccd schedule` for scheduled CCD transfers with an optional memo.
- Support seed-backed, imported, and Ledger-backed finalized local accounts as signers for the new `ccd` mutation commands.
- Update the canonical command taxonomy in `docs/commands.md` to document the new `ccd` command space and its planned structure.

## Capabilities

### New Capabilities
- `ccd-command-execution`: Submit native CCD transfer transactions through a dedicated `ccd` command space, including simple transfer and scheduled transfer flows with optional memos and Ledger-backed signing support.

### Modified Capabilities
- `command-taxonomy`: Document the new top-level `ccd` command space and its `transfer` and `schedule` commands in the canonical CLI taxonomy.

## Impact

- Affected code: `crates/ccd-wallet/src/cli.rs`, new `crates/ccd-wallet/src/commands/ccd/` command modules, shared signing/account-resolution helpers, and Ledger-backed account signing integration points.
- Affected docs: `docs/commands.md`.
- Affected systems: local account signer resolution, Concordium Ledger signing flows, transaction submission and finalization output for native CCD transfers.
