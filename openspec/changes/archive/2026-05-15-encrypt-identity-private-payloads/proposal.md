## Why

Identity objects contain personal data and are currently stored as plaintext JSON in SQLite. Since identities are owned by a seed and issued only after the seed password is provided, identity private data should be encrypted in the same seed password domain rather than left readable in the wallet database.

## What Changes

- Consolidate the wallet database schema into a new initial migration during development.
- Treat this as a development reset: existing local wallet databases are not migrated in-place and may be deleted/recreated.
- Replace plaintext identity `code_uri` and `identity_object` storage with encrypted identity private payload storage.
- Store identity public metadata separately from encrypted private payloads.
- Scope identities to `seeds.id` instead of only `seed_label`, with `ON DELETE CASCADE` from seeds to identities and from identities to private payloads.
- Use the existing per-seed DEK, unlocked by the seed password, to encrypt/decrypt identity private payloads.
- Keep identity labels unique per network.
- Keep the identity issuance UX unchanged apart from no longer storing identity private data in plaintext.

## Capabilities

### New Capabilities
<!-- None. This change modifies existing storage and issuance capabilities. -->

### Modified Capabilities
- `sqlite-store`: The development schema is consolidated into a new initial schema that includes encrypted identity private payload tables.
- `seed-storage`: Seed-owned identity rows reference `seeds(id)` and cascade on seed deletion.
- `identity-storage`: Identity private payloads are encrypted under the owning seed's password domain instead of stored as plaintext.
- `identity-issuance`: Issuance stores `code_uri` and issued identity object as encrypted private payload data after the seed has been unlocked.

## Impact

- Existing development `wallet.db` files must be reset/recreated.
- SQLite migrations are squashed into a new `001_initial_schema.sql` and current schema version is reset accordingly.
- `identities` stores public metadata only.
- New `identity_private_payloads` table stores encrypted private payload blobs.
- Seed unlock logic exposes an internal unlock context sufficient to decrypt the seed phrase and encrypt identity-owned private payloads with the seed DEK.
- Identity storage tests and issuance tests must be updated for encrypted payload behavior.
