## Why

The wallet CLI currently exposes smart contract module deployment but not the other contract operations already supported by the connect JSON-RPC API. Adding first-class contract init, update, read-only invoke, instance inspection, parameter-template generation, and module download commands gives CLI users parity with browser-facing contract workflows and a practical contract inspection surface.

## What Changes

- Add `ccd-wallet contract init` for wallet-approved smart contract initialization transactions.
- Add `ccd-wallet contract update` for wallet-approved smart contract receive-function update transactions.
- Add `ccd-wallet contract invoke` for non-mutating contract entrypoint calls without signing or account unlocking.
- Add `ccd-wallet contract show` for viewing contract instance metadata.
- Add `ccd-wallet contract parameter-template` for printing JSON parameter templates derived from embedded module schemas.
- Add `ccd-wallet contract download-module` for downloading module source bytes by module reference or by instance source module.
- Support raw `--parameter-hex`, inline schema-derived `--parameter-json`, and file-based `--parameter-json-file` inputs, using embedded module schemas from on-chain module sources when JSON encoding is requested.
- Accept user-facing CCD decimal amounts for contract operations and convert them to chain microCCD amounts internally.
- Preserve existing `contract deploy-module` behavior.

## Capabilities

### New Capabilities
- `contract-instance-execution`: CLI support for contract init/update transactions and read-only entrypoint invocation.
- `contract-instance-inspection`: CLI support for viewing contract instance metadata, printing JSON parameter templates, and downloading module source bytes.

### Modified Capabilities
- None.

## Impact

- Affects `crates/ccd-wallet/src/cli.rs` contract subcommands and parsing tests.
- Adds contract command handlers under `crates/ccd-wallet/src/commands/contract/`.
- Likely extracts shared init/update preparation, validation, submission, and finalization helpers from connect command code into `crates/ccd-wallet/src/smart_contracts/`.
- Reuses existing wallet account/network resolution, node client configuration, embedded-schema introspection, transaction summary rendering, and connect contract execution behavior where practical.
- No new external services or persistence schema changes are expected.
