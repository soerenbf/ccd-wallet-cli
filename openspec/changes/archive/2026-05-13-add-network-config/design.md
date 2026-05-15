## Context

`ccd-wallet` is currently stateless: the node endpoint must be supplied on every invocation via a flag or environment variable. There is no way to register a known network once and refer to it by name later.

This change introduces the first persistent storage layer — a durable config file — and the first command that writes to it: `ccd-wallet config network add`. The config file will hold user-managed, long-lived settings. A separate mutable state store (for things like the active network selection) is explicitly out of scope here and will be addressed in a follow-on change.

## Goals / Non-Goals

**Goals:**
- Define a durable config file format, its on-disk location, and its versioned schema.
- Implement `ccd-wallet config network add --name <NAME> --node <ENDPOINT>` that derives network identity and persists a named network entry.
- Keep durable config clearly separated from future mutable operational state.
- Refactor `config.rs` so that "runtime defaults" and "persisted config" are clearly distinct concerns.

**Non-Goals:**
- Active network selection or any mutable state store (`state.json`).
- `config network list`, `show`, `remove`, or `use` subcommands.
- Using the stored network config to satisfy `--node` for other commands (that comes later).
- Migration tooling or backwards-compatibility guarantees for the config schema (v1 is new).

## Decisions

### Config file location is always `~/.config/ccd-wallet/config.json`
The config file is always resolved relative to the user's home directory as `~/.config/ccd-wallet/config.json`, using `$HOME` on Unix and its equivalent on Windows.

- **Why:** A fixed, predictable path is easier for users to find, inspect, back up, and reason about. This tool is a developer/operator CLI where knowing exactly where config lives is a feature, not a limitation. XDG flexibility is not a goal for this project.
- **Alternative considered:** `dirs` crate for platform-appropriate resolution (XDG on Linux, `~/Library/Application Support` on macOS, `%APPDATA%` on Windows). Rejected because it produces different paths on different platforms for no benefit to this tool's target audience, and adds an external dependency for no meaningful gain.

### Config schema: versioned, networks keyed by name
```json
{
  "version": 1,
  "networks": {
    "<name>": {
      "node_endpoint": "<url>",
      "genesis_hash": "<block-hash-hex>"
    }
  }
}
```

- **Why:** Flat map keyed by user-supplied name is simple to read and edit manually. A top-level `version` field enables forward migration without breaking older tooling.
- **Alternative considered:** An array of network objects with a `name` field. Rejected because a map provides O(1) lookup by name and prevents duplicate names by construction.

### Network identity is `consensus_info.genesis_block`
The genesis hash stored per network is `ConsensusInfo::genesis_block` from the Concordium SDK.

- **Why:** `genesis_block` is the hash of the very first block and is stable across protocol eras. `current_era_genesis_block` changes with each protocol update and is therefore unsuitable as a persistent network identity.
- **Alternative considered:** Storing `current_era_genesis_block`. Rejected — it drifts over time.

### Fail on duplicate name; no silent overwrite
If a network with the given name already exists in the config, the command exits with an error and a clear message.

- **Why:** Config registration is an intentional, explicit act. Silent overwrite risks losing a previously verified genesis hash.
- **Alternative considered:** `--force` flag to allow overwrite. Deferred to a future `config network update` command to keep this change minimal.

### Normalize and persist the endpoint URI string
The endpoint is stored as the normalized URI string produced by `Endpoint::uri().to_string()`, not the raw user input.

- **Why:** Normalization ensures consistent display and comparison. This is the same form the rest of the code uses to display endpoints.
- **Trade-off:** Users may notice a trailing slash added by URI normalization (e.g., `http://127.0.0.1:20001/`). Acceptable for v1.

### Separate config storage module from runtime config module
`src/config.rs` currently holds only runtime defaults and helpers. Persistent config loading/saving will live in a new `src/store/` module (or `src/app_config.rs`), keeping the two concerns explicit.

- **Why:** "Config" currently means "how this invocation resolves flags." It should not also silently mean "read/write files." Naming the boundary early avoids a growing ball of mud.

## Risks / Trade-offs

- **[Missing home dir]** `$HOME` (or equivalent) may be unset in unusual environments such as certain CI containers. → Mitigation: fail early with an actionable error: `"Could not determine home directory"`.
- **[Concurrent writes]** Two simultaneous `config network add` invocations could corrupt the file. → Mitigation: deferred; acceptable for a local developer tool at this stage.
- **[Node unreachable at registration time]** The command must connect to the node before it can derive the genesis hash, so offline registration is not supported. → Mitigation: this is by design; the whole point is to verify the node and derive identity.

## Migration Plan

No migration required. This change introduces the file; it does not exist yet. If the file is absent on load, the store initializes with an empty networks map.

## Open Questions

- Should the `config` command group live at `src/commands/config/` as a module directory from day one, anticipating `list`/`show`/`use`? (Likely yes — structure the subcommand mod layout with growth in mind.)
- When `state.json` is introduced in a follow-on change, should both files share a `FileStore<T>` abstraction, or remain independent? Worth deferring until the shape of `state.json` is known.
