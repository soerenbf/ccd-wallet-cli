## Why

Seed management currently supports importing an existing phrase but not creating a new one, and it lacks a safe way to delete seeds. Adding random seed generation and removal makes the seed lifecycle more complete while preparing the database for future seed-owned records that should be deleted when their parent seed is deleted.

## What Changes

- Add `ccd-wallet seed add <LABEL> --random` to generate a new BIP39 seed phrase instead of prompting the user to enter one.
- Reuse the existing password prompt, encrypted storage, and temporary reveal flow for generated phrases.
- Add `ccd-wallet seed remove <LABEL>` to remove a configured seed after an explicit confirmation.
- Configure SQLite foreign-key enforcement on every wallet DB connection.
- Add a schema migration so `seed_vaults.seed_id` references `seeds(id) ON DELETE CASCADE`.
- Ensure seed removal clears `wallet_state.active_seed` when the removed seed was active.
- Establish the pattern that future seed-owned objects SHALL reference `seeds(id)` with `ON DELETE CASCADE`.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `seed-command`: Extend seed commands with `seed add <LABEL> --random` and `seed remove <LABEL>`.
- `seed-storage`: Add seed deletion semantics and cascade behavior for seed-owned DB rows.
- `sqlite-store`: Enable SQLite foreign keys and add a migration for cascade-on-delete on `seed_vaults`.
- `wallet-state`: Clear `active_seed` when the active seed is removed.

## Impact

- **CLI surface**: Adds `--random` to `seed add` and adds `seed remove <LABEL>`.
- **Storage**: Adds a schema migration from version 1 to 2 to recreate `seed_vaults` with `ON DELETE CASCADE`.
- **Runtime DB connection**: `PRAGMA foreign_keys = ON` must be enabled for every opened connection.
- **Code**: updates `src/commands/seed.rs`, `src/store/seeds.rs`, `src/store/db.rs`, migrations, and README.
- **Tests**: add tests for generated mnemonic storage/reveal, remove confirmation, cascade deletion, active-seed clearing, and foreign-key enforcement.
