## Why

`governance update` now supports Ledger-backed signing, but its current flow is still an all-in-one sign-and-submit command. That works for threshold-1 updates, but it cannot support the multi-signer governance thresholds needed to fully use Ledger-backed governance keys across operators or machines.

## What Changes

- Add a detached governance proposal flow so operators can create a proposal file from a governance update JSON payload.
- Add detached governance signing commands that sign an imported proposal with either a local governance key or a connected Governance Ledger app and write the result to a signature file.
- Add detached governance submission commands that revalidate proposal state against the node, merge provided signatures, and submit the resulting governance update.
- Keep all detached steps online and revalidating so proposal creation, signing, and submission each resolve the current network and governance authorization context from the node.
- Define stable JSON file formats for governance proposal files and detached signature files so multiple operators can exchange signing artifacts safely.

## Capabilities

### New Capabilities
<!-- None. -->

### Modified Capabilities
- `governance-update-submission`: extend governance update handling to support detached proposal creation, detached signature generation, and detached submission with online revalidation.
- `ledger-governance-cli-signing`: extend Ledger governance signing from the all-in-one update flow to detached proposal signing, including verify-key-to-index resolution during signing and submission.
- `command-taxonomy`: document the detached governance proposal command surface alongside the existing all-in-one `governance update` flow.

## Impact

- `crates/ccd-wallet/src/cli.rs`
- `crates/ccd-wallet/src/commands/governance.rs`
- `docs/commands.md`
- Governance update file I/O, validation, and signing/submission flow structure
- Detached signing interoperability between local governance key vaults and the Concordium Governance Ledger app
