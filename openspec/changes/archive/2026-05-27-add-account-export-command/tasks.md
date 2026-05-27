## 1. CLI surface and command wiring

- [x] 1.1 Add an `account export` subcommand and argument structure in `crates/ccd-wallet/src/cli.rs`, including explicit output-file input and existing non-interactive/default-selection flags where needed.
- [x] 1.2 Wire the new subcommand through `crates/ccd-wallet/src/main.rs` and `crates/ccd-wallet/src/commands/account.rs`.
- [x] 1.3 Reuse the existing account-selection and network-disambiguation helpers so `account export` resolves one target account consistently with other account commands.

## 2. Source-aware export material generation

- [x] 2.1 Add or refactor a helper that builds minimal exportable signer data (`address` + `accountKeys`) from a derived account by unlocking the owning seed, decrypting the stored address, and deriving the signing key material.
- [x] 2.2 Add or refactor a helper that builds the same minimal signer data from an imported account by unlocking the imported accounts vault and decrypting the stored imported payload.
- [x] 2.3 Ensure both source paths serialize to JSON accepted by `concordium_rust_sdk::types::WalletAccount::from_json_file`.

## 3. File export flow and safety behavior

- [x] 3.1 Implement the export command handler so it prompts for or validates an explicit destination path and refuses to proceed without one in non-interactive mode.
- [x] 3.2 Write the exported signer JSON to the selected file path and report the exported account and destination clearly.
- [x] 3.3 Surface actionable errors for ambiguous account selection, unlock failure, JSON construction failure, and file-write failure without writing partial output.

## 4. Validation and documentation

- [x] 4.1 Add focused tests for CLI parsing, account-resolution behavior, derived and imported export paths, and SDK-compatibility of the emitted JSON shape.
- [x] 4.2 Add tests for destination-path requirements and failure behavior in non-interactive mode.
- [x] 4.3 Update `README.md` and any relevant command help text to document `account export`, its minimal output format, and the security implications of writing plaintext signing material to disk.
- [x] 4.4 Run the relevant Rust test and lint commands to confirm the new command integrates cleanly with the existing CLI.
