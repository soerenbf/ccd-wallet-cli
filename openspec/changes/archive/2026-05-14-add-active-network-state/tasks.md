## 1. State Storage Layer

- [x] 1.1 Add `src/store/state.rs` with a versioned `AppState` model and load/save helpers for `~/.config/ccd-wallet/state.json`.
- [x] 1.2 Implement state file initialization and parent directory creation on first write.

## 2. Active Network Command

- [x] 2.1 Extend the existing `config network` command tree with a `use <NAME>` subcommand.
- [x] 2.2 Implement the handler for `config network use <NAME>` so it validates the named network exists in `config.json` before writing `active_network` to `state.json`.
- [x] 2.3 Print a confirmation message on successful active-network selection.

## 3. Node Resolution Fallback

- [x] 3.1 Update node endpoint resolution so `node info` falls back to the active network when neither `--network` nor `--node` is provided.
- [x] 3.2 Ensure explicit precedence remains `--node` > `--network` > active network.
- [x] 3.3 Surface actionable errors when no active network is set or when the active network is stale (present in `state.json` but missing from `config.json`).

## 4. Validation

- [x] 4.1 Run `cargo fmt --check` and `cargo clippy` clean.
- [x] 4.2 Validate: `config network use local` writes `state.json` with `version: 1` and `active_network: local`.
- [x] 4.3 Validate: `node info` with no flags uses the active network successfully.
- [x] 4.4 Validate: `config network use unknown` exits non-zero and does not write invalid active state.
- [x] 4.5 Validate: `node info` with no flags and no active network exits non-zero with a clear error.
- [x] 4.6 Validate: `node info --node ...` and `node info --network ...` still override active state.
