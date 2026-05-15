## 1. Database Foreign Keys and Migration

- [x] 1.1 Enable `PRAGMA foreign_keys = ON` in `store::db::open()` immediately after opening the SQLite connection
- [x] 1.2 Add migration SQL file `src/store/migrations/002_seed_vault_cascade.sql` that recreates `seed_vaults` with `REFERENCES seeds(id) ON DELETE CASCADE`
- [x] 1.3 Add migration version 2 to the migrations list and update `CURRENT_SCHEMA_VERSION` to 2
- [x] 1.4 Add migration tests proving version 1 databases migrate to version 2 while preserving seed vault rows
- [x] 1.5 Add cascade test proving deleting a seed deletes its seed vault when foreign keys are enabled

## 2. Seed Storage Removal

- [x] 2.1 Implement `store::seeds::remove(conn: &Connection, label: &str) -> Result<()>` that deletes a seed row by label and errors if the seed is not configured
- [x] 2.2 Add tests for removing an existing seed by label
- [x] 2.3 Add tests for attempting to remove an unknown seed label

## 3. Random Seed Generation

- [x] 3.1 Add `--random` flag to `SeedAddArgs`
- [x] 3.2 Implement random 24-word BIP39 mnemonic generation using OS randomness
- [x] 3.3 Update `seed add` so `--random` skips seed phrase prompting
- [x] 3.4 Store generated phrases through the existing encrypted seed storage path
- [x] 3.5 Reveal generated seed phrases through the existing temporary reveal helper after successful storage
- [x] 3.6 Add tests proving `seed add --random` generates, stores, and can reveal/unlock a valid mnemonic
- [x] 3.7 Add tests proving duplicate labels are rejected before random phrase generation/storage

## 4. Seed Remove CLI

- [x] 4.1 Add `SeedSubcommand::Remove` with positional `<LABEL>` argument
- [x] 4.2 Extend the seed prompt abstraction with a remove-confirmation prompt
- [x] 4.3 Implement `ccd-wallet seed remove <LABEL>`: validate seed exists, ask user to type the label, delete seed on exact match, and print confirmation
- [x] 4.4 Ensure mismatched confirmation aborts removal without deleting seed or vault rows
- [x] 4.5 Clear `wallet_state.active_seed` if it equals the removed label
- [x] 4.6 Leave `wallet_state.active_seed` unchanged if it points to a different seed
- [x] 4.7 Add tests for successful removal, confirmation mismatch, active seed clearing, and inactive seed removal

## 5. Documentation

- [x] 5.1 Update README with `ccd-wallet seed add <LABEL> --random`
- [x] 5.2 Update README with `ccd-wallet seed remove <LABEL>` and confirmation behavior
- [x] 5.3 Document that generated seed phrases are temporarily revealed and can later be shown with `seed show <LABEL>`

## 6. Validation

- [x] 6.1 Run `cargo fmt --check`
- [x] 6.2 Run `cargo clippy --all-targets --all-features -- -D warnings`
- [x] 6.3 Run `cargo test`
