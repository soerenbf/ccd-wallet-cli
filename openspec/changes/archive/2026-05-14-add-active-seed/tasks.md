## 1. Wallet State Support

- [x] 1.1 Add `ACTIVE_SEED_KEY` constant to `store::wallet_state`
- [x] 1.2 Add unit tests or command tests proving `active_seed` can be written/read via existing wallet-state helpers

## 2. Seed Store Lookup Helpers

- [x] 2.1 Ensure `store::seeds::find_by_label` is public and returns seed metadata without decrypting payloads
- [x] 2.2 Add tests for successful seed lookup by label and missing seed lookup

## 3. Seed CLI: use

- [x] 3.1 Add `SeedSubcommand::Use` with positional `<LABEL>` argument
- [x] 3.2 Implement `ccd-wallet seed use <LABEL>`: validate the seed exists, write `active_seed` to `wallet_state`, and print confirmation
- [x] 3.3 Ensure unknown seed labels are rejected without writing `active_seed`

## 4. Seed CLI: show

- [x] 4.1 Add `SeedSubcommand::Show` with optional positional `[LABEL]` argument
- [x] 4.2 Implement a temporary reveal helper that enters the terminal alternate screen, displays the seed phrase, and hides it when any key is pressed or after 30 seconds, whichever happens first
- [x] 4.3 Implement `ccd-wallet seed show <LABEL>` to prompt for the seed password, unlock that seed, and display the decrypted seed phrase through the temporary reveal helper
- [x] 4.4 Implement `ccd-wallet seed show` to resolve `wallet_state.active_seed`, prompt for that seed's password, unlock it, and display the decrypted seed phrase through the temporary reveal helper
- [x] 4.5 Ensure missing `active_seed` produces an actionable error advising `ccd-wallet seed use <LABEL>` or an explicit label
- [x] 4.6 Ensure stale `active_seed` produces an actionable error indicating the active seed is no longer configured
- [x] 4.7 Add tests proving `seed show` displays the seed phrase with the correct password and does not display it with the wrong password
- [x] 4.8 Add tests for temporary reveal timeout/keypress behavior using an injectable clock/input abstraction where practical

## 5. Documentation

- [x] 5.1 Update README with examples for `ccd-wallet seed use <LABEL>` and `ccd-wallet seed show [LABEL]`
- [x] 5.2 Document that `seed show` reveals the seed phrase after a password prompt, hides it on keypress or after 30 seconds, and warn about residual exposure risks such as screenshots, terminal logging, tmux/screen behavior, and clipboard history

## 6. Validation

- [x] 6.1 Run `cargo fmt --check`
- [x] 6.2 Run `cargo clippy --all-targets --all-features -- -D warnings`
- [x] 6.3 Run `cargo test`
