## 1. CLI Surface and Dispatch

- [x] 1.1 Add `AccountSubcommand::Show` and `AccountShowArgs` with positional `<ACCOUNT>`, `--network`, `--node`, `--block`, `--json`, `--verbose`, `--no-defaults`, and `--non-interactive` flags.
- [x] 1.2 Dispatch `account show` from the account command runner to a dedicated implementation function or module.
- [x] 1.3 Update clap help text to describe local-label and raw-address targets clearly.

## 2. Target and Network Resolution

- [x] 2.1 Implement account-show network/node context resolution using existing network selection conventions.
- [x] 2.2 Implement target parsing that treats a valid account address as a raw address target and otherwise treats the target as a local account label.
- [x] 2.3 Implement network-scoped local account lookup for derived and imported account records.
- [x] 2.4 Implement finalized local account address resolution by unlocking the relevant seed or imported account vault.
- [x] 2.5 Implement pending local account handling without querying missing finalized account state.

## 3. Account Query and View Model

- [x] 3.1 Query `get_account_info` for finalized/raw address targets at the selected block, defaulting to last finalized.
- [x] 3.2 Build an internal account-show view model containing header context, CCD balances, release schedule entries, token balances, and optional verbose protocol details.
- [x] 3.3 Decode token account module state to compute token available and locked balances, defaulting available to total when unavailable.
- [x] 3.4 Compute CCD locked balance from total balance and available balance.

## 4. Rendering

- [x] 4.1 Implement default human rendering with optional bracketed local metadata prefix and `<address> @ <network-or-endpoint>` when applicable.
- [x] 4.2 Render CCD total balance, available balance, locked balance when non-zero, and release schedule entries when present.
- [x] 4.3 Render protocol-level token sections with total balances and available/locked lines when applicable.
- [x] 4.4 Implement `--verbose` rendering for nonce, account index, credential count, threshold, and other selected protocol details.
- [x] 4.5 Implement `--json` output using a stable wallet-owned schema for raw, local, finalized, and pending account targets.

## 5. Tests and Documentation

- [x] 5.1 Add unit tests for target classification, local metadata header formatting, CCD locked calculation, and token available/locked calculation.
- [x] 5.2 Add rendering tests covering default, verbose, JSON, pending local account, no-token, and release-schedule cases.
- [x] 5.3 Add resolution tests for raw address targets not requiring wallet unlock and local labels being constrained by selected network.
- [x] 5.4 Update `docs/commands.md` to list `account show` under implemented account commands.
- [x] 5.5 Run Rust formatting and the relevant Cargo test suite.
