## Why

The wallet currently persists state in a plain JSON file (`state.json`) and has no mechanism for storing sensitive wallet objects such as seed phrases. Introducing a SQLite database gives the wallet a proper local store with relational capabilities, a migration path for structured data, and a foundation for future encrypted object storage — while immediately replacing the ad-hoc JSON state file.

## What Changes

- Add a SQLite database (`wallet.db`) stored in the OS-appropriate application-data directory (distinct from the existing config directory that holds `config.json`).
- Define the initial DB schema: `seeds` table (plaintext label, encrypted secret capsule, per-seed KDF parameters) and a `wallet_state` table (key/value application state).
- Implement per-seed password-protected encryption: each seed phrase is its own independently-locked secret; unlocking one seed gives access to that seed's encrypted payload only.
- Design the DB schema to be forward-compatible with future encrypted object types (accounts, identities, credentials) that will reference a seed as their encryption domain.
- **BREAKING**: Remove `state.json`; store `active_network` in the new `wallet_state` SQLite table going forward. No migration is required because the app has no existing users.
- The `config.json` file (network registry) is unchanged by this change.

## Capabilities

### New Capabilities

- `sqlite-store`: Local SQLite database — location resolution, connection setup, schema creation, and versioned migrations.
- `seed-storage`: Representation of seed phrases in the DB: plaintext label, per-seed envelope encryption (Argon2id KDF + ChaCha20-Poly1305 AEAD), and a schema that is forward-compatible with encrypted child objects (accounts, credentials, identities).
- `wallet-state`: Key/value application state in SQLite, replacing `state.json` — initially holds `active_network`.

### Modified Capabilities

<!-- none -->

## Impact

- **New dependencies**: `rusqlite` (bundled), `argon2`, `chacha20poly1305`, `rand`, possibly `dirs` for OS data-dir resolution.
- **Removed**: `src/store/state.rs` and the `state.json` file on disk. No migration is implemented.
- **Modified**: `src/store/` module restructured to expose the SQLite-backed store; existing callers of `store::state::load/save` updated to use the new wallet-state API.
- **No change** to the network config path or `config.json` format.
