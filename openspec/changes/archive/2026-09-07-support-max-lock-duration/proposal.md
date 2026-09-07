## Why

Protocol version 11 adds a maximum protocol-level token lock duration chain parameter, a governance update for changing it, and a corresponding level-2 authorization structure. The wallet must understand these additions so operators can inspect the live parameter and safely prepare, authorize, sign, and submit the new update without falling back to blind signing.

## What Changes

- Add `ccd-wallet node chain-parameters` to query and display all normalized chain parameters at a selectable block, with human-readable and machine-readable JSON output.
- Add `ccd-wallet node chain-updates` to display pending governance updates and every next update sequence number from one consistent block snapshot.
- Keep both new query commands aligned with existing read-only commands for network/node resolution, block parsing, `--json`, `--no-defaults`, prompting, output, and actionable gRPC errors.
- Support `MaxLockDuration` governance payload decoding, local signing, sequence-number resolution, review rendering, detached proposals, and submission.
- Recognize the P11 `token_parameters` level-2 authorization capability when listing keys and selecting or validating governance signers.
- Recognize V3 level-2 authorization updates locally while rejecting unsupported Max Lock Duration and V3 Ledger signing before device interaction until the Governance Ledger APDU protocol defines those operations.
- Preserve existing lock-creation behavior under the updated SDK by emitting no lock metadata.
- Update command and governance documentation for the new command and update family.

## Capabilities

### New Capabilities
- `chain-parameter-inspection`: Query all normalized chain parameters through gRPC with read-only CLI conventions and human or JSON output.
- `chain-update-inspection`: Query pending governance updates and next update sequence numbers through gRPC at a consistent block snapshot.

### Modified Capabilities
- `governance-update-submission`: Add typed Max Lock Duration update preparation, authorization, sequence resolution, local signing, proposal, and submission behavior.
- `governance-key-management`: Report and use the token-parameters governance capability from live chain state.
- `ledger-governance-cli-signing`: Reject typed update families that are known to the wallet but unsupported by the Governance Ledger protocol before opening a device flow.
- `node-connectivity`: Apply existing node endpoint, active-network, and no-defaults resolution behavior to chain-parameter inspection.
- `command-taxonomy`: Document `node chain-parameters` as part of the implemented command surface.

## Impact

- Affects the `ccd-wallet` node and governance command definitions, handlers, output models, streaming query handling, tests, and documentation.
- Affects compatibility handling for the updated Concordium Rust SDK `ChainParameters`, `UpdatePayload`, `UpdateType`, authorization V3, sequence-number, and lock configuration types.
- Does not add a new Governance Ledger APDU command or change persisted wallet/database formats.
