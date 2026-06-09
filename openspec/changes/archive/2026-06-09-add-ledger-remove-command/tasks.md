## 1. CLI Surface

- [x] 1.1 Add `LedgerSubcommand::Remove` and `LedgerRemoveArgs` with optional `LABEL` and `--non-interactive` to `crates/ccd-wallet/src/cli.rs`.
- [x] 1.2 Add CLI parsing tests for `ccd-wallet ledger remove <LABEL>` and `ccd-wallet ledger remove --non-interactive`.

## 2. Ledger Removal Flow

- [x] 2.1 Dispatch `LedgerSubcommand::Remove` from `crates/ccd-wallet/src/commands/ledger.rs`.
- [x] 2.2 Implement Ledger-only target resolution for explicit labels and omitted interactive labels, reusing the Ledger selector behavior where practical.
- [x] 2.3 Count identities and accounts owned by the selected Ledger signer owner before deletion.
- [x] 2.4 Add typed confirmation that warns about local owned identities/accounts and states that the physical Ledger device is not modified.
- [x] 2.5 Delete the Ledger signer owner with existing signer-owner deletion semantics and clear `active_key_source` when it matches the removed label.
- [x] 2.6 Return actionable errors for missing non-interactive labels, non-Ledger labels, absent Ledger key sources, and confirmation mismatches.

## 3. Tests

- [x] 3.1 Add command tests covering successful Ledger removal, cascade behavior for Ledger-owned local state, and success output path where feasible.
- [x] 3.2 Add command tests covering confirmation mismatch preserving the Ledger owner and vault/details rows.
- [x] 3.3 Add command tests covering active key-source clearing for the removed Ledger owner and preserving unrelated active key sources.
- [x] 3.4 Add command tests covering rejection of seed key sources and missing `--non-interactive` labels.

## 4. Documentation and Validation

- [x] 4.1 Update `docs/commands.md` to list `ledger remove` and describe local-only removal semantics.
- [x] 4.2 Run Rust formatting for the affected crate/workspace.
- [x] 4.3 Run targeted Rust tests for CLI parsing and Ledger command behavior.
- [x] 4.4 Run `OPENSPEC_TELEMETRY=0 openspec validate add-ledger-remove-command --strict`.
