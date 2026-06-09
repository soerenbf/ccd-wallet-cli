## Why

Governance update submission currently assumes locally stored governance keypairs in the encrypted governance key vault, which prevents operators from signing on-chain governance updates with keys held by the Concordium Governance Ledger app. Adding a Ledger-backed signing mode enables safer hardware-held governance signing now and creates a foundation for future detached multi-machine multi-Ledger signing flows.

## What Changes

- Add an exclusive Ledger-backed signing mode to `ccd-wallet governance update`, exposed through a `--ledger` flag and Ledger signer selection arguments that derive the Governance Ledger path from the update authorization family while allowing a governance key-index override.
- Route Ledger governance signing through the existing `ccd-wallet-ledger-governance` integration crate instead of decrypting local governance key material.
- Assemble governance update signatures returned by connected Ledger devices into the existing signed update submission/finalization flow.
- Reject unsupported combinations such as Ledger signing for blind/unknown serialized payloads and mixed local-vault plus Ledger signer selection.
- Structure the implementation around an internal prepared-update/signature-output boundary so future detached prepare/sign/submit workflows can reuse the same concepts without changing the all-in-one command semantics.
- Update command documentation for the new Ledger governance signing mode.

## Capabilities

### New Capabilities
- `ledger-governance-cli-signing`: Ledger-backed governance update signing through the CLI, including signer selection, device-backed signature acquisition, unsupported-mode rejection, and signed update assembly.

### Modified Capabilities
- `governance-update-submission`: Extend governance update submission so typed updates can be signed through an exclusive Ledger backend as an alternative to local governance key vault signing.
- `command-taxonomy`: Document the `governance update --ledger` command surface and its relationship to local governance key signing.

## Impact

- Affected code: `crates/ccd-wallet/src/cli.rs`, `crates/ccd-wallet/src/commands/governance.rs`, `crates/ccd-wallet/Cargo.toml`, and focused governance update tests.
- Affected docs: `docs/commands.md`.
- Affected dependencies: the wallet CLI will depend on `ccd-wallet-ledger-governance`, likely with its SDK conversion feature enabled where useful.
- Affected systems: governance update signing and submission orchestration. Governance key vault storage/import/list/remove behavior remains unchanged.
- Future compatibility: the internal signing boundary should be suitable for later detached multi-machine signing, but this change does not expose detached prepare/sign/submit commands.
