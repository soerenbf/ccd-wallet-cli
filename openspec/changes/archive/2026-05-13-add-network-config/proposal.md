## Why

The CLI is currently stateless: every command requires an explicit node endpoint, either via flag or environment variable. To support named network identities with verified provenance, the wallet needs a durable configuration store where known networks — along with their node endpoint and genesis hash — can be registered once and reused across commands.

## What Changes

- Introduce a durable configuration store at a platform-appropriate path (`~/.config/ccd-wallet/config.json`) that persists user-managed settings across invocations.
- Add a `ccd-wallet config network add` command that accepts a user-chosen name and a node endpoint, connects to the node, derives the genesis block hash from `consensus_info.genesis_block`, and writes a named network entry to the config store.
- The config file stores only durable user intent (known networks). Mutable operational state such as the active network is explicitly out of scope and will live in a separate state store in a future change.

## Capabilities

### New Capabilities
- `config-storage`: Provide a versioned durable configuration file for user-managed wallet settings, with a defined schema and load/save lifecycle.
- `network-config-add`: Allow the user to register a named Concordium network by providing a node endpoint; derive and persist the genesis hash as network identity.

### Modified Capabilities
- None.

## Impact

- Adds a new `config` command group and `config network add` subcommand to the CLI.
- Introduces first disk I/O in the CLI; requires determining a platform-appropriate config directory.
- Requires a new dependency for platform config directory resolution (e.g., `dirs` crate).
- Existing `node info` command and endpoint-resolution logic are unaffected.
- The internal `config.rs` module will be refactored to distinguish runtime defaults from the new persisted config layer.
