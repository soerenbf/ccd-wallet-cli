## Context

The wallet database currently stores seed phrases encrypted in `seed_vaults`, but identity issuance results are stored as plaintext JSON in `identities.identity_object`. The `code_uri` used to poll identity issuance status is also stored as plaintext. Identity objects can contain personal data, and `code_uri` can act as a temporary capability URL. Both should be treated as private identity payload data.

The current database is still development data. The user is comfortable resetting the local database, which avoids a complex legacy encryption path that would require prompting for seed passwords during a repair operation.

## Goals / Non-Goals

**Goals:**
- Encrypt identity private payloads under the owning seed password domain.
- Squash database migrations into a clean new initial schema.
- Store public identity metadata separately from encrypted private data.
- Make identity rows seed-owned by referencing `seeds(id)` with cascade delete.
- Preserve existing identity issuance UX and behavior where possible.

**Non-Goals:**
- No in-place migration of existing plaintext identity rows.
- No legacy plaintext compatibility path.
- No repair command for old identity rows.
- No change to identity provider protocol behavior.
- No attempt to hide public metadata such as identity label, provider id, network hash, status, or creation time.

## Decisions

### Development reset instead of compatibility migration

The change will consolidate database migrations into a new `001_initial_schema.sql`. Existing development databases may be deleted and recreated.

Rationale:
- Encrypting existing identity objects requires seed passwords and cannot be done by a normal SQLite migration.
- Avoids mixed plaintext/encrypted storage paths.
- Keeps the implementation simpler and safer at this stage.

### Use seed DEK as identity private payload key

Identity private payloads will be encrypted with the existing per-seed DEK from `seed_vaults`.

```text
password ──Argon2id──▶ KEK ──decrypt──▶ seed DEK
                                      │
                                      ├─ decrypt seed phrase
                                      └─ encrypt/decrypt identity private payloads
```

Rationale:
- The seed DEK is already random, per-seed, and password-protected.
- Password changes only rewrap the seed DEK, so identity payloads do not need re-encryption.
- It matches the desired seed password domain semantics.

### Introduce an unlock context

Seed storage should expose a controlled unlock context rather than exposing only the seed phrase. The context should provide the decrypted seed phrase for request construction and the seed DEK for encrypting/decrypting seed-owned private child objects.

Rationale:
- Identity issuance already prompts for the seed password.
- Unlock once, use the seed phrase for identity request construction and seed DEK for identity private payload encryption.
- Keeps KDF/seed vault details centralized in seed storage.

### Public metadata and private payload split

Use two identity tables:

```sql
identities (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  seed_id TEXT NOT NULL REFERENCES seeds(id) ON DELETE CASCADE,
  network_genesis_hash TEXT NOT NULL,
  ip_identity INTEGER NOT NULL,
  identity_index INTEGER NOT NULL,
  label TEXT NOT NULL,
  status TEXT NOT NULL CHECK(status IN ('pending', 'done')),
  created_at INTEGER NOT NULL,
  UNIQUE(network_genesis_hash, seed_id, ip_identity, identity_index),
  UNIQUE(network_genesis_hash, label)
)

identity_private_payloads (
  identity_id INTEGER PRIMARY KEY REFERENCES identities(id) ON DELETE CASCADE,
  cipher_version INTEGER NOT NULL,
  ciphertext BLOB NOT NULL,
  nonce BLOB NOT NULL
)
```

The encrypted plaintext payload is JSON, initially shaped like:

```json
{
  "code_uri": "https://...",
  "identity_object": null
}
```

When issuance completes:

```json
{
  "code_uri": "https://...",
  "identity_object": { }
}
```

Rationale:
- Public metadata remains queryable without passwords.
- Private data is stored in one encrypted blob, making future private fields easy to add.
- Cascade deletion prevents orphaned encrypted identity payloads after seed deletion.

### AAD binding

Identity private payload encryption should use AEAD AAD that binds the ciphertext to stable identity metadata. A suitable AAD is:

```text
identity:<identity_id>:<network_genesis_hash>:<seed_id>:<ip_identity>:<identity_index>:private_payload:v<version>
```

Rationale:
- Prevents accidental or malicious ciphertext transplantation across identity rows.
- Binds the encrypted payload to the identity row and owning seed.

## Risks / Trade-offs

- **Risk:** Existing local wallet DBs stop working.  
  **Mitigation:** This is an intentional development reset; document deleting/recreating `wallet.db`.

- **Risk:** Seed DEK use expands beyond seed phrase encryption.  
  **Mitigation:** Use strict AAD separation and treat the seed DEK as the seed-owned private-data domain key.

- **Risk:** Labels and metadata remain plaintext.  
  **Mitigation:** This change only protects identity private payloads. Document that labels/network/provider/status metadata are not private.

- **Risk:** A status `done` row could theoretically miss a private payload if a write fails midway.  
  **Mitigation:** Insert/update identity status and encrypted payload in transactions where practical.

## Migration Plan

- Replace existing migration files with a consolidated `001_initial_schema.sql`.
- Set current schema version to `1`.
- Do not attempt to migrate old database files.
- During development, delete/recreate the local `wallet.db` after this change.

## Open Questions

None currently. The agreed direction is to reset the development database and avoid legacy plaintext compatibility.
