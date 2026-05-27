## Why

The wallet can import accounts and sign with them internally, but it cannot yet export an account as a standalone JSON signer file for use with the Concordium Rust SDK and related tools. Users need a simple way to extract a selected wallet account into a `WalletAccount::from_json_file`-compatible file without requiring full genesis-format export support.

## What Changes

- Add an `account export` command that exports a selected wallet account to JSON.
- Make the initial export format the minimal SDK-compatible shape containing `address` and `accountKeys`.
- Support export for both seed-derived and imported accounts by resolving account material through the account's source kind.
- Reuse existing account-selection and network-resolution behavior so exported account labels remain unambiguous within a network.
- Require the appropriate secret unlock before export and write plaintext signing material only to an explicitly chosen destination.
- Leave richer export formats, including full genesis-style or browser-wallet wrapper formats, out of scope for the initial version.

## Capabilities

### New Capabilities
- `account-export`: Export a stored wallet account as a minimal JSON signer file compatible with `concordium_rust_sdk::types::WalletAccount::from_json_file`.

### Modified Capabilities
- `account-signing-source`: Extend source-aware account material resolution so the wallet can build exportable signer JSON for both derived and imported accounts.

## Impact

- CLI surface in `crates/ccd-wallet/src/cli.rs` and account command routing in `crates/ccd-wallet/src/commands/account.rs`.
- Reuse of existing account lookup, network disambiguation, seed unlock, and imported-vault unlock flows.
- No SQLite schema changes are expected.
- Documentation updates for the new command and its security implications.
