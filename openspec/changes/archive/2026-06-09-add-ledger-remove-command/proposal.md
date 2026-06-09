## Why

Ledger key sources can be enrolled and used for recovery, but users currently have no Ledger-specific command to remove an enrolled Ledger key source from the local wallet. This leaves stale Ledger enrollment metadata, local vault state, and Ledger-owned recovered identities/accounts without a clear cleanup path.

## What Changes

- Add a `ccd-wallet ledger remove <LABEL>` command that removes an enrolled Ledger key source from local wallet state after explicit typed confirmation.
- Allow interactive selection of a Ledger key source when the label is omitted, while requiring an explicit label in `--non-interactive` mode.
- Warn users about the number of Ledger-owned identities and accounts that will be removed by cascade before deletion.
- Clear the active key-source wallet-state value when it points at the removed Ledger key source.
- Document that removal only affects local wallet state and does not modify the physical Ledger device.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ledger-signer-owner`: Adds user-facing Ledger key-source removal behavior.
- `command-taxonomy`: Adds `ledger remove` to the implemented Ledger command surface.

## Impact

- CLI argument definitions in `crates/ccd-wallet/src/cli.rs`.
- Ledger command handling in `crates/ccd-wallet/src/commands/ledger.rs`.
- Existing signer-owner storage deletion helpers and cascade semantics in `ccd-wallet-core` are reused; no schema migration is expected.
- Command taxonomy documentation in `docs/commands.md`.
