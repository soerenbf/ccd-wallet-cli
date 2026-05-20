## Why

The wallet can submit transactions and surface transaction hashes, but it does not yet provide a direct way to inspect what happened to a specific transaction later. Users need a simple, entity-oriented command that can show the current node-known status of a transaction hash without requiring local transaction storage.

## What Changes

- Add a top-level `transaction show <HASH>` command that queries a Concordium node for the current status of a transaction hash.
- Render stable transaction properties in a human-oriented format, including the queried network/node context, lifecycle status (`received`, `committed`, `finalized`, or `absent`), and per-block RFC3339 UTC block time.
- For committed and finalized transactions, show per-block outcome details together with pretty-printed SDK-derived `events` for successful results or `rejectReason` for rejected results instead of manually formatting every transaction variant.
- Treat unknown hashes as an `absent` show result rather than surfacing a raw node `not found` error.
- Reuse the existing network/node resolution model (`--network`, `--node`, active network defaults, and `--no-defaults`) for transaction inspection.

## Capabilities

### New Capabilities
- `transaction-show`: Show node-known details for a transaction hash, including lifecycle status and outcome details.

### Modified Capabilities
- `node-connectivity`: Extend existing node resolution requirements so `transaction show` uses the same network-or-endpoint selection behavior as other node-backed commands.

## Impact

- Affected CLI surface in `crates/ccd-wallet/src/cli.rs` and command routing in `crates/ccd-wallet/src/main.rs` / `crates/ccd-wallet/src/commands/`.
- Reuse of Concordium Rust SDK transaction-status queries and existing endpoint resolution helpers.
- README command documentation and examples for transaction inspection.
- No local transaction storage or schema changes are required.
