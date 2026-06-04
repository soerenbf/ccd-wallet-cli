## Why

Users need a direct way to inspect the current on-chain state of a Concordium account from the wallet CLI, whether the account is stored locally or supplied as a public account address. Existing account commands manage local records, but there is no account-focused equivalent of `transaction show`, `contract show`, or `token show` for balances, release schedules, and token holdings.

## What Changes

- Add `ccd-wallet account show <ACCOUNT>` to query account state from a selected network or node.
- Accept either a local account label or a raw Concordium account address as `<ACCOUNT>`.
- For local account labels, resolve the account within the selected network, decrypt the stored finalized address, and annotate output with minimal local metadata using a bracketed prefix such as `[<seed-label> : <local-label>]` for derived accounts or `[<local-label>]` for imported accounts.
- Render a balance-oriented default view:
  - CCD total balance, available balance, locked balance, and release schedule entries.
  - Protocol-level token balances, available balances, and locked balances when applicable.
- Hide lower-level protocol details such as account nonce and account index behind `--verbose`.
- Support machine-readable output via `--json`.
- Update command taxonomy documentation to include `account show`.

## Capabilities

### New Capabilities
- `account-inspection`: Query and render on-chain account state for local account labels and raw account addresses.

### Modified Capabilities
- `command-taxonomy`: Add `account show` to the implemented account command surface.

## Impact

- Rust CLI command definitions in `crates/ccd-wallet/src/cli.rs`.
- Account command dispatch and helpers in `crates/ccd-wallet/src/commands/account.rs` or a new account inspection module.
- Node/network resolution and account address resolution paths.
- Human and JSON rendering for account balances, CCD release schedules, token balances, and verbose protocol details.
- `docs/commands.md` command taxonomy documentation.
