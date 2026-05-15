## Why

The CLI can now resolve a node either from `--node` or from a saved `--network`, but it still requires the user to specify one of them every time. To make common workflows less repetitive, the wallet needs a separate persistent state store for the active network so commands can default to that selection without mixing mutable state into `config.json`.

## What Changes

- Introduce a persistent mutable state store separate from `config.json` to hold the active network selection.
- Add a `ccd-wallet config network use <NAME>` command that validates the named network exists in the durable config store and sets it as the active network in the state store.
- Update `node info` so it resolves the target in this priority order:
  1. `--node <ENDPOINT>`
  2. `--network <NAME>`
  3. active network from the state store
- Keep `config.json` as durable user-managed configuration only; active network MUST NOT be stored there.

## Capabilities

### New Capabilities
- `state-storage`: Provide a separate persistent state file for mutable operational state, including the active network.
- `active-network-selection`: Allow the user to set and persist the active network by name.

### Modified Capabilities
- `node-connectivity`: Update node command resolution so it defaults to the active network when neither `--network` nor `--node` is supplied.

## Impact

- Adds a new state persistence layer and schema distinct from `config.json`.
- Adds a `config network use` command to the existing CLI tree.
- Changes endpoint resolution in `src/commands/node.rs` to consult the state store.
- Uses the durable config store to validate named networks before storing active state.
