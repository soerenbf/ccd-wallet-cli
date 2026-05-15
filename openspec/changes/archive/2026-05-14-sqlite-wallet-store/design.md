## Context

The wallet currently stores mutable runtime state in `~/.config/ccd-wallet/state.json` (a plain JSON file). There is no facility for storing sensitive material. The next phase of the wallet requires persisting seed phrases with password-based encryption, and future changes will extend this to accounts, identities, and credentials — all of which must relate back to a parent seed.

SQLite gives us a local relational store with no external daemon, good Rust support via `rusqlite`, and a clear migration path. The store must live in the OS application-data directory (not the config directory), following platform conventions.

## Goals / Non-Goals

**Goals:**
- Introduce a single `wallet.db` SQLite file in the OS-appropriate data directory.
- Define an initial schema: `schema_version`, `wallet_state` (KV), `seeds`, and `seed_vaults`.
- Implement per-seed password envelope encryption using Argon2id + ChaCha20-Poly1305.
- Schema must be forward-compatible with future encrypted child-object tables (accounts, credentials, identities) that reference `seeds.id` as their encryption domain.
- Replace `state.json` by storing `active_network` in `wallet_state` going forward, with no migration.

**Non-Goals:**
- Account, identity, or credential storage (subsequent changes).
- Sync, export, or multi-device support.
- Changing `config.json` or the network registry.
- Any GUI or interactive TUI.

## Decisions

### D1: Use `rusqlite` with the `bundled` feature

**Decision**: Depend on `rusqlite` with the `bundled` feature flag, which compiles SQLite from source as part of the crate.

**Rationale**: Eliminates the need for a system SQLite library, ensures a known version is used, and simplifies cross-platform builds. Size cost is acceptable for a CLI binary.

**Alternative considered**: Link against the system `libsqlite3`. Rejected because version inconsistencies across macOS / Linux distributions can cause subtle issues.

### D2: DB path via `dirs` crate (data directory, not config directory)

**Decision**: Resolve the DB path as `{data_dir}/ccd-wallet/wallet.db` using `dirs::data_dir()`.

**Rationale**: Separates mutable/binary wallet data from human-readable config (`config.json`). This matches XDG Base Directory conventions on Linux (`~/.local/share`), macOS (`~/Library/Application Support`), and Windows (`%APPDATA%`). Config stays in `{config_dir}/ccd-wallet/config.json` as today.

```
macOS:   ~/Library/Application Support/ccd-wallet/wallet.db
Linux:   ~/.local/share/ccd-wallet/wallet.db
Windows: %APPDATA%\ccd-wallet\wallet.db
```

**Alternative considered**: Use the same directory as `config.json`. Rejected to avoid mixing concerns and to follow platform conventions.

### D3: Per-seed envelope encryption (Argon2id + ChaCha20-Poly1305)

**Decision**: Each seed has its own independent encryption domain. A password is stretched via Argon2id into a Key Encryption Key (KEK). The KEK decrypts a stored Data Encryption Key (DEK). The DEK (ChaCha20-Poly1305) encrypts the seed's secret payload.

```
password
  │
  └─Argon2id(password, salt, params)──▶ KEK
                                          │
                                          └─AEAD decrypt──▶ DEK
                                                              │
                                                              └─AEAD decrypt──▶ seed payload
```

**Two-layer rationale**: The password does not directly encrypt the payload. This allows password rotation by re-encrypting only the DEK (one small row), leaving all payload ciphertext unchanged. It also sets up the pattern for child objects to share the same DEK.

**AEAD AAD**: Every encryption operation includes Associated Additional Data binding ciphertext to its object identity:
```
aad = "{object_id}:{object_kind}:v{cipher_version}"
```
This prevents ciphertext from being silently transplanted between rows.

**Alternative considered**: Password directly encrypts the seed payload (no DEK layer). Rejected because password changes would require re-encrypting every object.

**Alternative considered**: SQLCipher (whole-DB encryption). Rejected because it would encrypt the structural metadata (seed labels, relation edges, counts) that we explicitly want to keep queryable without a password.

### D4: Seeds table split into `seeds` (plaintext) + `seed_vaults` (encrypted key material)

**Decision**: The `seeds` table holds only plaintext columns (id, label, timestamps). The `seed_vaults` table holds the KDF parameters, salt, encrypted DEK, and encrypted seed payload. These are separate rows joined by `seed_id`.

**Rationale**: Makes the separation between plaintext metadata and sensitive material explicit and visible in the schema. Future child-object tables (`accounts`, `credentials`) will reference `seeds.id` as their encryption-domain key.

**Schema (forward-compatible form)**:

```sql
CREATE TABLE schema_version (
    version  INTEGER NOT NULL
);

CREATE TABLE wallet_state (
    key    TEXT PRIMARY KEY NOT NULL,
    value  TEXT NOT NULL
);

CREATE TABLE seeds (
    id         TEXT PRIMARY KEY NOT NULL,   -- UUIDv4
    label      TEXT UNIQUE NOT NULL,        -- plaintext user label
    created_at INTEGER NOT NULL,            -- Unix epoch seconds
    updated_at INTEGER NOT NULL
);

CREATE TABLE seed_vaults (
    seed_id           TEXT PRIMARY KEY NOT NULL REFERENCES seeds(id),
    kdf_algorithm     TEXT NOT NULL,         -- "argon2id"
    kdf_params_json   TEXT NOT NULL,         -- {"m_cost":..,"t_cost":..,"p_cost":..}
    salt              BLOB NOT NULL,         -- 16 bytes random
    encrypted_dek     BLOB NOT NULL,         -- ChaCha20-Poly1305 ciphertext of DEK
    dek_nonce         BLOB NOT NULL,         -- 12 bytes random nonce for DEK encryption
    cipher_version    INTEGER NOT NULL DEFAULT 1,
    payload_ciphertext BLOB NOT NULL,        -- ChaCha20-Poly1305 ciphertext of seed secret
    payload_nonce     BLOB NOT NULL          -- 12 bytes random nonce for payload encryption
);
```

`wallet_state` replaces `state.json`. Initial key: `"active_network"`.

### D5: Schema versioning via `schema_version` table

**Decision**: A single-row `schema_version` table is checked and updated at DB open time. Migrations run in sequence using a match on the current version integer.

**Rationale**: Lightweight, no external migration framework needed for a CLI tool. Keeps the migration path explicit in code.

**Alternative considered**: No versioning (schema assumed correct). Rejected because the DB will evolve across multiple changes.

### D6: `state.json` is simply deleted

**Decision**: Remove `state.json` and `src/store/state.rs` with no migration. The app has no existing users.

**Rationale**: No users means no data to preserve. Migration logic would be dead code.

## Risks / Trade-offs

- **Bundled SQLite binary size**: ~700 KB added to the binary. Acceptable for a CLI wallet.
- **`dirs` crate surface**: Small, well-maintained crate. Low risk.
- **Argon2id parameters**: Need to be set high enough for password security but low enough for CLI usability. Starting with OWASP-recommended minimums (`m_cost = 65536`, `t_cost = 3`, `p_cost = 1`); can be increased without breaking existing stored data since params are stored per vault.
- **Key material in memory**: DEK and plaintext seed secret are wrapped in types that implement `zeroize::Zeroizing`, ensuring the memory is explicitly overwritten when dropped. OS-level `mlock` (preventing the pages from being swapped to disk) is out of scope and noted as a future hardening item.
- **Single DB file**: Any corruption loses all data. This is a local wallet CLI — the user is expected to have seed phrase backups. A future backup/export change can mitigate this.

## Migration Plan

1. On startup, resolve `wallet.db` path via `dirs::data_dir()`.
2. If the file does not exist, create it and run all migrations from version 0.
3. If it exists, read `schema_version`; apply any pending migrations in order.
4. Existing `config.json` is untouched. `state.json` is not read; remove it from the codebase.

Rollback: not applicable for a local CLI. Users can restore `state.json` from a backup if needed, but this is considered unlikely to be needed.

## Open Questions

- Should Argon2id parameters be user-configurable (e.g., via `config.json`) to support slower hardware? Currently hardcoded to safe defaults.
- Future: should `wallet.db` path be overridable via an env var (e.g., `CCD_WALLET_DB_PATH`) for testing and CI? Likely yes — worth adding from the start.
