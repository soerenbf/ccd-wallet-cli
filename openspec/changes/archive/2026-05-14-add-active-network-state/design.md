## Context

`config.json` currently holds durable, user-managed configuration such as named networks. The user explicitly does not want mutable operational state — especially the active network selection — to live in that file. At the same time, node commands should become more ergonomic by defaulting to a previously selected active network when neither `--network` nor `--node` is specified.

To make that behavior usable, this change also needs a way to set the active network in the first place. The smallest coherent surface is a separate state store plus a `config network use <NAME>` command.

## Goals / Non-Goals

**Goals:**
- Introduce a separate persistent state file for mutable operational state.
- Persist `active_network` outside `config.json`.
- Add a command to set the active network by name.
- Update node endpoint resolution to fall back to the active network when neither explicit selector is provided.

**Non-Goals:**
- Moving any existing durable config out of `config.json`.
- Supporting more state fields than `active_network` in this change beyond what is needed for schema/versioning.
- Applying active-network fallback to commands other than `node info` in this change.

## Decisions

### Separate `state.json` from `config.json`
The state file will be stored at `~/.config/ccd-wallet/state.json`, adjacent to `config.json`, and will contain mutable operational state only.

- **Why:** Keeps durable user intent and mutable selections clearly separated. This matches the user's stated preference and reduces churn in `config.json`.
- **Alternative considered:** Add `active_network` to `config.json`. Rejected by requirement and by design principle.

### State schema is versioned
The state file will use a small versioned schema such as:

```json
{
  "version": 1,
  "active_network": "local"
}
```

- **Why:** Symmetry with `config.json` and future migration support.

### `config network use <NAME>` sets active state
A new subcommand will validate that the named network exists in `config.json` and then persist it as the active network in `state.json`.

- **Why:** Without a setter command, there is no ergonomic way to create the active state the node command is supposed to use.
- **Alternative considered:** Infer active network automatically from the last successful command. Rejected because it is surprising and conflates user intent with incidental activity.

### Node resolution precedence is explicit
For node commands in this change, endpoint resolution order will be:
1. `--node`
2. `--network`
3. active network from `state.json`

- **Why:** Explicit command-line inputs should always win over stored state. Named network should still outrank active state because it is an explicit choice on this invocation.

### Missing active network is an actionable error
If neither `--node` nor `--network` is provided and no active network is set, the command exits with an actionable error instructing the user to either pass an explicit selector or run `config network use <NAME>`.

- **Why:** This preserves clarity and avoids surprising implicit behavior.

## Risks / Trade-offs

- **[State/config drift]** The active network in `state.json` may refer to a network name removed from `config.json`. → **Mitigation:** resolve through the config store every time and fail with a clear message if the active network no longer exists.
- **[Extra file complexity]** Introducing a second file adds implementation overhead. → **Mitigation:** keep the schema minimal and colocate helper code with the existing store module structure.
- **[User confusion about config vs state]** Two files may be conceptually heavier than one. → **Mitigation:** document the distinction in CLI help and README in a later documentation-focused update if needed.

## Migration Plan

- No migration is required. `state.json` is new.
- If `state.json` does not exist, the CLI treats active state as unset.

## Open Questions

- Should future commands share a generic `resolve_target_network` helper that returns a network name plus endpoint, rather than only an endpoint? Probably yes, but not necessary in this change.
