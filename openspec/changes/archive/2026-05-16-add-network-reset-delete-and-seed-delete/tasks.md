## 1. Storage and query foundations

- [x] 1.1 Add config-store helpers to delete one or more network aliases by label and list aliases by genesis hash for reset-row rendering and orphan detection.
- [x] 1.2 Add identity/account store helpers to prune rows by `network_genesis_hash` and to discover distinct stored network hashes for orphan detection.
- [x] 1.3 Ensure network-partition pruning deletes private payload rows transitively and add store-level tests covering partition-prune behavior.
- [x] 1.4 Keep seed deletion semantics aligned with existing seed-owned cascade deletion and add tests covering deletion of owned identities/accounts.
- [x] 1.5 Add wallet-state tests for clearing `active_seed` and `active_network` only when a deleted seed or deleted network alias had been active.

## 2. CLI command surface

- [x] 2.1 Extend CLI enums to add `network reset`, `network delete`, and `seed delete`, and remove `seed remove` from the public CLI surface.
- [x] 2.2 Implement `seed delete` with explicit label-or-selector resolution, typed confirmation, destructive warnings, and active-seed cleanup.
- [x] 2.3 Implement `network reset` for label and `--genesis-hash` targeting, including interactive partition selection that can surface orphaned stored hashes and render rows as `hash - aliases` or `hash (orphan)`.
- [x] 2.4 Implement `network delete <LABEL>...` as alias-only config deletion, with repeated positional labels and interactive alias multiselect fallback when labels are omitted.
- [x] 2.5 Add orphaning-aware delete warnings and active-network cleanup for deleted aliases, without pruning network identities/accounts.
- [x] 2.6 Add actionable errors for missing required targets in `--non-interactive` mode and other invalid destructive-input combinations.

## 3. UX, documentation, and validation

- [x] 3.1 Add cliclack warning/confirmation flows that clearly state whether an action removes config or data, including identity/account counts and orphaning notices where relevant.
- [x] 3.2 Add interactive tests or command-level tests for selector fallback, alias multiselect, partition-row rendering, orphan-hash presentation, delete orphaning warnings, and active-state cleanup semantics.
- [x] 3.3 Update README and command docs for `network reset`, `network delete`, `seed delete`, repeated delete labels, reset hash targeting, and destructive effects.
- [x] 3.4 Run `cargo fmt`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace`.
