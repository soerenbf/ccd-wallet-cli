## 1. Command surface and taxonomy

- [x] 1.1 Add the top-level `ccd` clap command space with `transfer` and `schedule` subcommands in `crates/ccd-wallet/src/cli.rs`
- [x] 1.2 Add `crates/ccd-wallet/src/commands/ccd/` command modules and wire dispatch from the main command runner
- [x] 1.3 Update `docs/commands.md` to document the new `ccd` command space and its relationship to `transaction`

## 2. Shared CCD signing and submission support

- [x] 2.1 Add shared native-CCD command input and review helpers for sender resolution, recipient resolution, confirmation, and finalization behavior
- [x] 2.2 Implement local signing and submission for seed-backed and imported accounts for native CCD payloads
- [x] 2.3 Implement Ledger-backed signing for simple transfer, transfer-with-memo, scheduled transfer, and scheduled transfer-with-memo payloads

## 3. CCD transfer command

- [x] 3.1 Implement `ccd transfer` argument resolution for sender, recipient, amount, optional memo, and network/node context
- [x] 3.2 Build and submit simple transfer or transfer-with-memo payloads based on whether `--memo` is supplied
- [x] 3.3 Add focused behavioral tests for transfer parsing, non-interactive required-value validation, shared recipient account-label resolution paths, and transfer-signing request selection without adding CLI output snapshot coverage

## 4. CCD schedule command

- [x] 4.1 Implement `ccd schedule` argument resolution for sender, recipient, repeated `--release <RFC3339=CCD>` entries, optional memo, and network/node context
- [x] 4.2 Validate and convert repeated release entries into the scheduled-transfer payload and submit scheduled transfer or scheduled transfer with memo as appropriate
- [x] 4.3 Add focused behavioral tests for release-entry parsing, invalid or missing release validation, memo-aware scheduled-transfer request construction, and Ledger-backed scheduled-transfer signing paths without adding CLI output snapshot coverage

## 5. Finalization and transaction output

- [x] 5.1 Reuse or extend transaction finalization rendering so successful CCD transfer and scheduled-transfer submissions produce clear submitted/finalized output
- [x] 5.2 Intentionally skip dedicated CCD CLI output snapshot tests; rely on existing shared transaction-rendering coverage instead of adding command-specific format assertions
