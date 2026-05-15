## 1. Config Storage Layer

- [x] 1.1 Create `src/store/` module with a `ConfigFile` struct that owns the on-disk path, and implements load and save operations over the versioned config schema.
- [x] 1.2 Define the `AppConfig` model struct (`version`, `networks` map) with `serde` derive and a `NetworkEntry` struct (`node_endpoint`, `genesis_hash`).
- [x] 1.3 Implement config path resolution: expand `~/.config/ccd-wallet/config.json` from `$HOME`, failing with a clear error if the home directory cannot be determined.
- [x] 1.4 Implement config file initialization: create parent directories as needed, and write an empty config when the file does not exist.
- [x] 1.5 Surface a clear error when the home directory cannot be determined.

## 2. CLI Structure

- [x] 2.1 Add a `config` command group to the root CLI (`src/commands/config/mod.rs`) and wire it into `main.rs`.
- [x] 2.2 Add a `network` subcommand group under `config` and an `add` subcommand with `--name` and `--node` arguments.

## 3. Network Add Command

- [x] 3.1 Implement the `config network add` handler: connect to the node, call `get_consensus_info()`, extract `genesis_block` as the genesis hash.
- [x] 3.2 Check for duplicate network name before writing; return an actionable error if the name already exists.
- [x] 3.3 Persist the new network entry (normalized endpoint URI + genesis hash) to `config.json` and print a confirmation message on success.
- [x] 3.4 Ensure no config file write occurs if the node connection or consensus query fails.

## 4. Validation

- [x] 4.1 Run `cargo fmt --check` and `cargo clippy` clean.
- [x] 4.2 Manually validate: add a network against the local node, inspect `config.json` for correct schema.
- [x] 4.3 Manually validate: attempt to add the same network name a second time and confirm the duplicate error.
- [x] 4.4 Manually validate: attempt to add a network with an unreachable endpoint and confirm no file write occurs.
