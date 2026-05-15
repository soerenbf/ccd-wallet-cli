## Context

Seed storage and seed commands currently support importing, selecting, and showing existing seed phrases. Users still need a built-in way to generate a new seed phrase and a safe way to remove seed material from the wallet.

The database already models `seed_vaults` as child rows of `seeds`, but the foreign key does not currently specify `ON DELETE CASCADE`, and SQLite foreign key enforcement must be enabled explicitly for each connection.

## Goals / Non-Goals

**Goals:**
- Add `ccd-wallet seed add <LABEL> --random` to generate a new BIP39 mnemonic phrase.
- Reuse the existing encrypted seed storage path for generated phrases.
- Reveal generated seed phrases through the existing temporary reveal UI.
- Add `ccd-wallet seed remove <LABEL>` with explicit confirmation.
- Delete seed vault rows automatically when their seed row is deleted.
- Enable SQLite foreign-key enforcement for every wallet DB connection.
- Clear `wallet_state.active_seed` when the removed seed was active.

**Non-Goals:**
- Account/identity/credential deletion, except defining the cascade pattern for future seed-owned tables.
- Clipboard integration.
- Secure deletion from disk at the SQLite page level.
- Seed phrase generation customization such as 12 vs 24 words. The generated phrase SHALL be 24 words.

## Decisions

### D1: `seed add <LABEL> --random` generates a 24-word BIP39 phrase

Use the existing `bip39` crate to generate an English 24-word mnemonic using OS randomness.

Flow:

```text
ccd-wallet seed add main_seed --random
  │
  ├─ validate label and duplicate status
  ├─ generate 24-word BIP39 phrase
  ├─ prompt password + confirmation
  ├─ encrypt/store generated phrase
  └─ reveal generated phrase temporarily in alternate screen
```

The generated phrase is shown after successful storage so the command can report one clear success path. Users can still recover it later with `seed show <LABEL>`.

### D2: `--random` replaces seed phrase prompting

When `--random` is set, the command MUST NOT prompt for seed phrase input. Without `--random`, behavior stays unchanged: prompt for an existing phrase, normalize, validate, then store.

### D3: `seed remove <LABEL>` requires typing the label as confirmation

Deletion is destructive, so removal requires explicit confirmation by typing the seed label.

```text
This will remove seed 'main_seed' and all seed-owned data.
Type 'main_seed' to confirm:
```

This avoids accidental deletion while not requiring the seed password, which a user may not know if they are removing a broken/imported seed.

### D4: Foreign keys are enabled at connection open

SQLite requires `PRAGMA foreign_keys = ON` per connection. The DB open path will enable it immediately after opening the connection and before running migrations or normal operations.

### D5: Add schema version 2 migration for seed vault cascade

Add migration `002_seed_vault_cascade.sql` that recreates `seed_vaults` with:

```sql
seed_id TEXT PRIMARY KEY NOT NULL REFERENCES seeds(id) ON DELETE CASCADE
```

Even though the app has no broad user base, using a versioned migration exercises the migration system and avoids rewriting history.

### D6: Active seed state is cleared manually

`wallet_state.active_seed` stores a seed label and is not a foreign key. After deleting a seed, if `active_seed` equals the removed label, delete that key from `wallet_state`.

### D7: Future seed-owned tables use `ON DELETE CASCADE`

Future tables scoped to a seed (accounts, identities, credentials) should reference `seeds(id) ON DELETE CASCADE` unless there is a strong reason not to. This keeps seed removal semantics consistent.

## Risks / Trade-offs

- **Generated seed phrase exposure**: Generated phrases must be shown so users can back them up. Mitigation: use the existing temporary reveal flow and document that the phrase can be shown again with `seed show`.
- **Accidental deletion**: Removing a seed is destructive. Mitigation: require typing the exact seed label to confirm.
- **SQLite secure deletion**: Deleting rows does not guarantee old page contents are overwritten on disk. Mitigation: out of scope; this wallet relies on encrypted payloads at rest and user seed backups. A future hardening change may consider `secure_delete`.
- **Migration complexity**: Recreating a table is required to add `ON DELETE CASCADE`. Mitigation: keep migration small and covered by tests.

## Migration Plan

1. On DB open, enable `PRAGMA foreign_keys = ON`.
2. Add migration version 2:
   - Disable foreign keys during table recreation.
   - Rename `seed_vaults` to `seed_vaults_old`.
   - Create new `seed_vaults` with `ON DELETE CASCADE`.
   - Copy rows from old table.
   - Drop old table.
   - Update `schema_version` to 2.
   - Re-enable foreign keys.
3. No data transformation is needed.

## Open Questions

None.
