## 1. Governance Vault Storage

- [x] 1.1 Add SQLite schema migration for governance vault metadata and encrypted governance key payloads scoped by network genesis hash.
- [x] 1.2 Implement governance vault store helpers for create-or-find by network, unlock, empty-vault cleanup, and password-based DEK handling.
- [x] 1.3 Implement encrypted storage for raw governance keypair JSON payloads without storing plaintext verify keys or derived governance levels in the database.
- [x] 1.4 Implement governance key duplicate detection by decrypted `verifyKey` within a network vault.
- [x] 1.5 Add store tests for governance vault encryption/decryption, duplicate key rejection, empty-vault deletion, and migration behavior.

## 2. Governance Key Import and Removal

- [x] 2.1 Add a `governance` CLI command tree with `governance keys import`, `governance keys list`, and `governance keys remove` subcommands.
- [x] 2.2 Implement single-file governance key import with network resolution, governance vault setup/unlock, keypair JSON validation, and encrypted raw-payload storage.
- [x] 2.3 Implement `--dir` governance key import that scans recognized keypair files, ignores aggregate snapshot files such as `governance-keys.json`, and imports all valid keypair files under one vault unlock.
- [x] 2.4 Implement `governance keys remove <verify-key>`, interactive `governance keys remove`, and `governance keys remove --all`, including governance-vault unlock and empty-vault cleanup.
- [x] 2.5 Add CLI tests for malformed-key rejection, duplicate-key rejection, directory import ignoring aggregate files, explicit verify-key removal, interactive removal selection, and `--all` removal.

## 3. Live Chain-State Inspection

- [x] 3.1 Implement governance key listing as an unlock-and-query flow that decrypts local key payloads and queries live chain parameters from the resolved node, using the active network by default.
- [x] 3.2 Implement matching logic from decrypted local `verifyKey`s to live root, level1, and level2 authorization structures derived from chain parameters.
- [x] 3.3 Render governance key list rows using public key identity plus live-derived authorization status, including stored-but-not-authorized keys.
- [x] 3.4 Add tests for live-state matching logic, including root/level1/level2 matches and stored keys that are no longer authorized.
- [x] 3.5 Add actionable error handling tests for governance key list when node connectivity or chain-parameter queries fail.

## 4. Network Lifecycle and Documentation

- [x] 4.1 Update `network reset` storage pruning so governance vault data for the target network partition is fully removed.
- [x] 4.2 Update README and command documentation with governance key import/list/remove examples, vault behavior, and the fact that live chain state—not aggregate snapshot files—is authoritative.
- [x] 4.3 Run `cargo fmt`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace`.

## 5. Governance Key Listing UX Refinements

- [x] 5.1 Amend the governance-key proposal, design, and spec artifacts to cover missing-vault preflight checks and tag-first list output.
- [x] 5.2 Implement `governance keys list` preflight behavior so missing governance vaults fail before password entry.
- [x] 5.3 Render governance key list rows in tag-first aligned format with operator-oriented sorting and capability summaries.
- [x] 5.4 Add tests for missing-vault preflight behavior, tag-first row formatting, capability summaries, and sort order.
- [x] 5.5 Update README governance-key examples/documentation for the refined list behavior and run focused formatting/tests.
- [x] 5.6 Implement interactive `governance keys remove` as a fuzzy multiselect that reuses authorization-aware rows with compact verify-key display.
- [x] 5.7 Add tests for compact verify-key rendering and interactive remove row formatting.
- [x] 5.8 Make `governance keys list` abbreviate verify keys by default and add `--show-full` for full-key rendering.
- [x] 5.9 Add tests and README updates for compact-vs-full governance key list display.
- [x] 5.10 Amend the active change artifacts to cover bracket-first `account list` row formatting.
- [x] 5.11 Update `account list` row rendering and tests to show `[<seed label>|imported] <label> (<optional address>)` with compact metadata suffixes.
- [x] 5.12 Amend the active change artifacts so `account list` defaults to all accounts on the resolved network and requires explicit `--seed` for `--show-addresses`.
- [x] 5.13 Update `account list` seed-scope resolution, tests, and docs for network-wide default listing plus explicit-seed address reveal.
- [x] 5.14 Remove governance-key list verify-key alignment padding and update focused tests/spec text.
