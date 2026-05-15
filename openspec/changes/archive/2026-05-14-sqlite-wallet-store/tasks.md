## 1. Dependencies

- [x] 1.1 Add `rusqlite` with `bundled` feature to `Cargo.toml`
- [x] 1.2 Add `dirs` crate to `Cargo.toml` for OS data-directory resolution
- [x] 1.3 Add `argon2` crate to `Cargo.toml`
- [x] 1.4 Add `chacha20poly1305` crate to `Cargo.toml`
- [x] 1.5 Add `rand` crate to `Cargo.toml`
- [x] 1.6 Add `uuid` crate with `v4` feature to `Cargo.toml`
- [x] 1.7 Add `zeroize` crate with `derive` feature to `Cargo.toml`

## 2. Database Path Resolution

- [x] 2.1 Implement `store::db::db_path() -> Result<PathBuf>` resolving `{data_dir}/ccd-wallet/wallet.db`; fall back to error if `dirs::data_dir()` returns `None`
- [x] 2.2 Support `CCD_WALLET_DB_PATH` env-var override in `db_path()`
- [x] 2.3 Add `store::db::open() -> Result<Connection>` that creates parent directories if absent and returns a `rusqlite::Connection`

## 3. Schema and Migrations

- [x] 3.1 Create `store::migrations` module with a versioned migration list loaded from standalone SQL files via `include_str!`
- [x] 3.2 Implement migration 0→1: create `schema_version`, `wallet_state`, `seeds`, and `seed_vaults` tables
- [x] 3.3 Implement `store::migrations::run(conn: &Connection) -> Result<()>` that reads `schema_version`, applies pending migrations in order, and updates the version row

## 4. Remove state.json

- [x] 4.1 Delete `src/store/state.rs` and remove it from the `store` module
- [x] 4.2 Remove `~/.config/ccd-wallet/state.json` path references from the codebase

## 5. Wallet State Store

- [x] 5.1 Implement `store::wallet_state::get(conn: &Connection, key: &str) -> Result<Option<String>>`
- [x] 5.2 Implement `store::wallet_state::set(conn: &Connection, key: &str, value: &str) -> Result<()>`
- [x] 5.3 Replace all calls to `store::state::load()` / `store::state::save()` with the new `wallet_state` functions
- [x] 5.4 Remove `src/store/state.rs` and its public module declaration

## 6. Seed Encryption Primitives

- [x] 6.1 Implement `store::crypto::generate_dek() -> Zeroizing<[u8; 32]>` using `rand::rngs::OsRng`; return type ensures the key is zeroed on drop
- [x] 6.2 Implement `store::crypto::derive_kek(password: &str, salt: &[u8], params: &Argon2Params) -> Result<Zeroizing<[u8; 32]>>` using Argon2id; return type ensures the key is zeroed on drop
- [x] 6.3 Implement `store::crypto::aead_encrypt(key: &[u8; 32], plaintext: &[u8], aad: &[u8]) -> Result<(Vec<u8>, [u8; 12])>` (returns ciphertext + nonce) using ChaCha20-Poly1305
- [x] 6.4 Implement `store::crypto::aead_decrypt(key: &[u8; 32], nonce: &[u8; 12], ciphertext: &[u8], aad: &[u8]) -> Result<Zeroizing<Vec<u8>>>` using ChaCha20-Poly1305; wrap plaintext in `Zeroizing` so it is overwritten on drop
- [x] 6.5 Define AAD helper `store::crypto::object_aad(id: &str, kind: &str, cipher_version: u32) -> Vec<u8>`

## 7. Seed Storage

- [x] 7.1 Define `store::seeds::SeedRecord { id, label, created_at, updated_at }` struct
- [x] 7.2 Implement `store::seeds::add(conn: &Connection, label: &str, secret: &[u8], password: &str) -> Result<SeedRecord>`: generate UUID id, generate DEK, derive KEK from password with fresh salt, encrypt DEK with KEK, encrypt `secret` payload with DEK (using AAD), insert rows into `seeds` and `seed_vaults`
- [x] 7.3 Implement `store::seeds::list(conn: &Connection) -> Result<Vec<SeedRecord>>` reading from `seeds` table (no password needed)
- [x] 7.4 Implement `store::seeds::unlock(conn: &Connection, label: &str, password: &str) -> Result<Zeroizing<Vec<u8>>>`: load vault for the seed, derive KEK, decrypt DEK, decrypt and return the zeroizing secret payload
- [x] 7.5 Implement `store::seeds::change_password(conn: &Connection, label: &str, old_password: &str, new_password: &str) -> Result<()>`: unlock DEK with old password, re-encrypt DEK with new KEK, update `seed_vaults` row; payload ciphertext unchanged
- [x] 7.6 Validate uniqueness error on duplicate label insert and surface a clear error message

## 8. Wire Up DB Open in main

- [x] 8.1 Call `store::db::open()` (path resolution + migrations) early in `main()` and thread the `Connection` through to command handlers that need it
- [x] 8.2 Update `commands::config::network` to read/write `active_network` via `store::wallet_state` instead of `store::state`
- [x] 8.3 Update `commands::node` to read `active_network` via `store::wallet_state`

## 9. Tests

- [x] 9.1 Unit-test `store::crypto` round-trip: encrypt then decrypt returns original plaintext; wrong password fails
- [x] 9.2 Unit-test AAD binding: decryption with mismatched AAD returns error
- [x] 9.3 Unit-test `store::migrations::run` on a fresh in-memory SQLite DB reaches current schema version
- [x] 9.4 Unit-test `store::seeds::add` + `unlock` round-trip with correct and incorrect passwords
- [x] 9.5 Unit-test `store::seeds::change_password` — old password fails after change, new password succeeds
