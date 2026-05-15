## 1. Dependencies

- [x] 1.1 Add `bip39` crate to `Cargo.toml` for mnemonic validation
- [x] 1.2 Add `rpassword` crate to `Cargo.toml` for hidden terminal prompts

## 2. Seed Phrase Input and Validation

- [x] 2.1 Create a seed command module (e.g., `src/commands/seed.rs`)
- [x] 2.2 Implement `normalize_seed_phrase(input: &str) -> String` that trims leading/trailing whitespace and collapses internal whitespace to single spaces
- [x] 2.3 Implement `validate_seed_phrase(normalized: &str) -> Result<()>` using the `bip39` crate
- [x] 2.4 Implement `validate_seed_label(label: &str) -> Result<()>` requiring non-empty ASCII alphanumeric, dash, and underscore characters only
- [x] 2.5 Add unit tests for valid mnemonic validation, invalid mnemonic rejection, whitespace normalization, and seed label validation

## 3. Prompt Abstraction

- [x] 3.1 Define a small prompt abstraction/function layer for reading sensitive inputs so command logic can be tested without a real terminal
- [x] 3.2 Implement production prompts using hidden input for seed phrase, password, and password confirmation
- [x] 3.3 Ensure seed phrase and password are never accepted as CLI arguments
- [x] 3.4 Add tests for password confirmation mismatch without touching the DB

## 4. Seed CLI

- [x] 4.1 Add top-level `Seed` command group to `src/cli.rs`
- [x] 4.2 Add `SeedSubcommand::Add` with positional `<LABEL>` argument
- [x] 4.3 Route `Command::Seed` from `main.rs` to the seed command handler
- [x] 4.4 Implement `ccd-wallet seed add <LABEL>`: validate label, reject duplicate labels before prompting for secrets, prompt for seed phrase/password, validate phrase, call `store::seeds::add`, and print success
- [x] 4.5 Ensure failed validation and password mismatch do not write seed rows

## 5. Network CLI Cleanup

- [x] 5.1 Promote `NetworkCommand` from `ConfigSubcommand::Network` to top-level `Command::Network`
- [x] 5.2 Remove the `Config` command group and `ConfigSubcommand` routing if no other config commands remain
- [x] 5.3 Update command handlers so `ccd-wallet network add --name <NAME> --node <ENDPOINT>` behaves like the former `ccd-wallet config network add ...`
- [x] 5.4 Update command handlers so `ccd-wallet network use <NAME>` behaves like the former `ccd-wallet config network use <NAME>`
- [x] 5.5 Update active-network error messages to reference `ccd-wallet network use <NAME>`

## 6. Documentation

- [x] 6.1 Update README network examples from `config network` to top-level `network`
- [x] 6.2 Add README example for `ccd-wallet seed add <LABEL>` and document that seed phrase/password are prompted interactively and hidden

## 7. Validation

- [x] 7.1 Run `cargo fmt --check`
- [x] 7.2 Run `cargo clippy --all-targets --all-features -- -D warnings`
- [x] 7.3 Run `cargo test`
