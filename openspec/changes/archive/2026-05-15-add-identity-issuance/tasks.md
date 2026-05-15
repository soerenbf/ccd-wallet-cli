## 1. Cargo workspace restructure

- [x] 1.1 Convert the repo root into a Cargo workspace: add `[workspace]` to the root `Cargo.toml` with `members = ["crates/ccd-wallet-core", "crates/ccd-wallet"]`
- [x] 1.2 Create `crates/ccd-wallet-core/` as a library crate (`lib.rs`); move `src/store/`, `src/config.rs` into it; update `mod` declarations and `use` paths
- [x] 1.3 Create `crates/ccd-wallet/` as the binary crate; move `src/main.rs`, `src/cli.rs`, `src/commands/` into it; add `ccd-wallet-core` as a path dependency
- [x] 1.4 Move all `[dependencies]` to the appropriate member `Cargo.toml`; hoist shared versions to workspace `[dependencies]` table with `workspace = true` where applicable
- [x] 1.5 Run `cargo build --workspace` and `cargo test --workspace` — all pass with no path errors

## 2. Dependencies

- [x] 2.1 Add `open` crate to `crates/ccd-wallet/Cargo.toml` for launching the system browser
- [x] 2.2 Confirm `concordium-rust-sdk` (already present) re-exports `concordium_base` as `concordium_rust_sdk::base` and that `concordium_rust_sdk::id::account_holder::generate_pio_v1` is accessible; no local path dependencies required
- [x] 2.3 Add `hmac`, `sha2`, `hkdf` to `ccd-wallet-core` if not already transitive — needed for SLIP-0010 and keygen_bls implementations
- [x] 2.4 Add `cliclack` to `crates/ccd-wallet/Cargo.toml` for arrow-key selection and styled interactive issuance output

## 3. Key derivation (`ccd-wallet-core`)

- [x] 3.1 Implement `wallet::slip10_derive(seed: &[u8; 64], path: &[u32]) -> [u8; 32]` — SLIP-0010 hardened-only derivation using HMAC-SHA512; master key from `HMAC-SHA512("ed25519 seed", seed)`, child keys by hardened CKD
- [x] 3.2 Implement `wallet::keygen_bls(key_seed: &[u8; 32]) -> Fr` — HKDF-SHA256 over the 32-byte seed to produce a BLS12-381 `Fr` scalar, following the Concordium derivation spec; use `concordium_rust_sdk::base` for the `Fr` type
- [x] 3.3 Implement `wallet::ConcordiumHdWallet::from_seed_phrase(phrase: &str, net: Net)` wrapping PBKDF2 (via `bip39::Mnemonic::to_seed`) + the SLIP-0010 root; expose `get_id_cred_sec`, `get_prf_key`, `get_blinding_randomness` using the paths `[ip, id, 2/3/4]` under `m/44'/<net>'`
- [ ] 3.4 Write tests against the Concordium-published key derivation test vectors: mainnet and testnet, at least two (ip, identity) pairs each, for all three key types

## 4. Network config and wallet proxy

- [x] 4.1 Extend the config model in `crates/ccd-wallet-core/src/store/config.rs` so `NetworkEntry` stores `wallet_proxy: String`
- [x] 4.2 Update `network add` CLI args and handler to require `--wallet-proxy <URL>` and persist it alongside `node_endpoint` and `genesis_hash`
- [x] 4.3 Update any config/storage tests and docs that inspect network entries

## 5. SQLite migration — identity tables (`ccd-wallet-core`)

- [x] 5.1 Create `crates/ccd-wallet-core/src/store/migrations/003_identities.sql` — `identities` table with `id`, `seed_label`, `ip_identity`, `identity_index`, `label`, `status` (`pending`/`done`/`error`), `code_uri`, `identity_object`, `created_at`; unique constraint on `(seed_label, ip_identity, identity_index)`; unique constraint on `(seed_label, label)`
- [x] 5.2 Bump `CURRENT_SCHEMA_VERSION` to `3` and register the migration in `migrations.rs`
- [x] 5.3 Write migration tests: fresh DB has `identities` table; v2→v3 migration succeeds and preserves existing rows

## 6. Identity storage layer (`ccd-wallet-core`)

- [x] 6.1 Implement `identities::next_index(conn, seed_label, ip_identity) -> Result<u32>` — returns next available identity index (0 for first)
- [x] 6.2 Implement `identities::insert_pending(conn, seed_label, ip_identity, identity_index, label, code_uri) -> Result<i64>`
- [x] 6.3 Implement `identities::set_done(conn, id, identity_object_json) -> Result<()>` and `identities::set_error(conn, id) -> Result<()>`
- [x] 6.4 Write unit tests: next_index auto-increments per (network, seed, IP); duplicate `(network_genesis_hash, label)` rejected; duplicate `(network_genesis_hash, seed_label, ip_identity, identity_index)` rejected; done/error status transitions work

## 7. Wallet proxy + identity provider HTTP client (`ccd-wallet-core`)

- [x] 7.1 Add wallet proxy metadata client support for fetching IDP metadata from the selected network's `wallet_proxy` endpoint
- [x] 7.2 Create `ccd-wallet-core::identity_provider::client`
- [x] 7.3 Implement `client::start_issuance(base_url, redirect_uri, id_object_request_json) -> Result<String>` — sends `GET` with `scope`, `response_type`, `redirect_uri`, `state` query params; follows redirects but stops and returns the location when it contains `redirect_uri`; errors on non-redirect response
- [x] 7.4 Implement `client::poll_code_uri(code_uri) -> Result<PollResult>` — `PollResult` is `Pending | Done(serde_json::Value) | ProviderError(String)`
- [x] 7.5 Write unit tests: redirect returned correctly; non-redirect errors; done/pending/error poll variants deserialised correctly

## 8. Callback receiver abstraction (`ccd-wallet-core`)

- [x] 8.1 Define `trait CallbackReceiver { fn receive(&self, browser_url: &str) -> Result<String>; }` — `browser_url` is the URL to open; returns extracted `code_uri`
- [x] 8.2 Implement `ManualPasteReceiver`: prints `browser_url`, prompts user to paste final redirect URL, parses `#code_uri=` and `#error=` fragments; returns `code_uri` or error
- [x] 8.3 Write unit tests: valid `#code_uri=` extracted; `#error=` returns error; unrecognisable input errors

## 9. Identity request construction (`ccd-wallet-core`)

- [x] 9.1 Implement `identity_provider::build_request(wallet, ip_info, ar_infos, global_context, identity_index) -> Result<String>` — calls `get_id_cred_sec`/`get_prf_key`/`get_blinding_randomness` then `concordium_rust_sdk::id::account_holder::generate_pio_v1`; returns serialised `idObjectRequest` JSON
- [ ] 9.2 Write unit test: request construction succeeds with a known test mnemonic and fixture IP/AR/global context data

## 10. CLI command (`ccd-wallet`)

- [x] 10.1 Create `crates/ccd-wallet/src/commands/identity/mod.rs` and `new.rs`
- [x] 10.2 Add `IdentitySubcommand::New(IdentityNewArgs)` to `cli.rs`; wire `identity` top-level subcommand in `commands/mod.rs` and `main.rs`
- [x] 10.3 Update `IdentityNewArgs` resolution rules so `--network` selects the network config and `--node` resolves or validates the configured network by matching `genesis_hash`
- [x] 10.4 Prompt for identity label (hidden validation: non-empty, ASCII alphanumeric/dash/underscore, unique within the resolved network); error early if label already exists on that network before any network call
- [x] 10.5 Resolve `wallet_proxy` from the selected or inferred network config, and fail early if no configured network matches the supplied node / no active network is available
- [x] 10.6 Implement provider resolution using on-chain provider identities plus wallet proxy metadata; interactive path presents an arrow-key selection prompt showing provider name and id
- [x] 10.7 Implement seed resolution: use `--seed` if provided else resolve active seed; unlock seed to obtain phrase; construct `ConcordiumHdWallet`
- [x] 10.8 Wire full issuance flow: `build_request` → wallet proxy metadata lookup → `client::start_issuance` → open browser URL (via `open` crate, fall back to printing URL) → `CallbackReceiver::receive` → `identities::insert_pending` → poll loop → `identities::set_done/set_error`
- [x] 10.9 Implement polling loop: 5-minute timeout, progress output on each poll, graceful Ctrl-C cancellation that prints the stored `code_uri` for future retry

## 11. Quality and documentation

- [x] 11.1 Run `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --workspace` — all pass
- [x] 11.2 Update `README.md` and network command docs for `wallet_proxy`, workspace structure, and `identity new` usage
