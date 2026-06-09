## Why

Ledger-backed key sources can already be enrolled and used for identity issuance, but they cannot yet run the same recovery flows that seed-backed key sources support through `seed add --restore` and `seed sync`. That leaves Ledger users without a first-class way to recover existing identities and accounts into the local wallet, even though the Ledger app now exposes the export material needed for those recovery workflows.

## What Changes

- Add `ccd-wallet ledger sync [LABEL]` to recover identities and accounts for an enrolled Ledger-backed key source on a selected network.
- Extend `ccd-wallet ledger setup [LABEL]` with `--restore <NETWORK>` so a newly enrolled Ledger key source can immediately run recovery after successful setup.
- Reuse the existing recovery UX shape where practical, including network resolution, provider filtering, progress reporting, and recovery summaries.
- Add explicit Ledger secret-export approval semantics for Ledger recovery so interactive and non-interactive behavior matches other security-sensitive Ledger flows.
- Update command taxonomy documentation to describe the expanded `ledger` command space.

## Capabilities

### New Capabilities
- `ledger-recovery-sync`: Recover identities and accounts for Ledger-backed key sources by exporting recovery-critical material from a matching Concordium Ledger device and importing discovered entities into the local wallet.

### Modified Capabilities
- `ledger-signer-owner`: Extend Ledger enrollment and owner flows so `ledger setup` can optionally trigger immediate recovery after enrollment.
- `command-taxonomy`: Document `ledger sync` and the `ledger setup --restore <NETWORK>` recovery-oriented Ledger command surface.

## Impact

- Affected code: `crates/ccd-wallet/src/cli.rs`, `crates/ccd-wallet/src/commands/ledger.rs`, shared recovery/orchestration code currently in `crates/ccd-wallet/src/commands/seed.rs`, and related tests.
- Affected docs: `docs/commands.md`.
- Affected integrations: Concordium Ledger app export flows, wallet-proxy identity recovery endpoints, and node-backed account discovery.
- Security impact: Ledger recovery will export sensitive recovery material transiently in host memory and must require explicit approval semantics consistent with existing Ledger export flows.
