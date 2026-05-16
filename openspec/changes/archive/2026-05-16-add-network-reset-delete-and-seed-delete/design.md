## Context

The wallet stores different kinds of local state in different places:

- network aliases live in `config.json`
- seeds, identities, and accounts live in SQLite
- identities and accounts are partitioned by `network_genesis_hash`
- seed-owned rows already cascade from `seeds(id)`

This means “remove a network” is not a simple row deletion. There are really two distinct layers:

```text
network alias (config)   !=   network data partition (SQLite)
name in config                rows keyed by genesis hash
```

This change formalizes that split instead of hiding it.

## Goals / Non-Goals

**Goals:**
- Provide a first-class `network reset` command for pruning wallet-local identities/accounts by network partition.
- Provide a first-class `network delete` command for deleting one or more network config aliases by label.
- Replace `seed remove` with `seed delete`.
- Ensure destructive commands never silently derive their target from active/default state.
- Make destructive UX explicit and informative, including warnings and affected-row counts.
- Support interactive reset targeting for configured networks and orphaned network partitions, and interactive delete targeting through alias multiselect.

**Non-Goals:**
- Adding a general-purpose trash/undo model.
- Redesigning non-destructive `network use`, `seed use`, or listing flows beyond selector reuse where helpful.
- Introducing a relational `networks` table in SQLite.
- Automatically deleting orphaned data during unrelated commands.

## Decisions

### 1. Distinguish network config aliases from network data partitions
The design adopts two separate operations because the storage model already has two separate concepts:

- **config alias**: a named entry in `config.json`
- **data partition**: identities/accounts whose `network_genesis_hash` matches the target hash

Commands therefore map cleanly to those layers:

- `network reset` => data partition only
- `network delete` => config alias deletion only

### 2. `network reset` targets a network partition, not an alias object
`network reset` accepts either:
- a configured network label, or
- `--genesis-hash <HASH>`

Resolution rules:
- label => resolve config entry, then use its `genesis_hash`
- hash => target that partition directly, even if no config entry exists

Effects:
- delete all identities for the resolved `network_genesis_hash`
- delete all accounts for the resolved `network_genesis_hash`
- keep all config aliases intact
- keep active-network state intact

### 3. `network delete` is alias-oriented and never prunes network data
`network delete` accepts one or more configured network labels.

Effects:
- remove the selected config aliases
- never delete identities/accounts as part of the delete command
- warn when the deletion will leave stored identities/accounts for a hash with no remaining aliases

This keeps the command family clean:
- `network reset` handles data partitions
- `network delete` handles config aliases

It also avoids hidden last-alias behavior and makes full cleanup a deliberate two-step action when needed: delete aliases, then reset the orphaned partition.

### 4. `seed delete` replaces `seed remove`
The CLI surface is standardized on `delete` for destructive removal of stored entities.

`seed delete`:
- accepts an explicit label or interactive selection
- never silently defaults to the active seed
- deletes the seed row
- relies on SQLite cascade semantics to delete seed vaults, identities, accounts, and their private payloads

No `seed reset` command is introduced in this change.

### 5. Destructive commands clean up active state but never consult it for target resolution
Target resolution and active-state cleanup are intentionally separate:

```text
target resolution  !=  active-state cleanup
```

Rules:
- no destructive command may use active seed/network as an implicit target
- deleting the active seed clears `active_seed`
- deleting the active network alias clears `active_network`
- `network reset` does not clear `active_network` because the config alias remains valid
- deleting any network alias clears `active_network` if that exact alias had been active

### 6. Interactive destructive selection should distinguish partitions from aliases
`network reset` and `network delete` should intentionally present different selector shapes.

`network reset` selects a network partition. Its selector should:
- include configured partitions and orphaned partitions
- always show the genesis hash
- append matching alias labels when present
- append `(orphan)` when no aliases exist
- show identity/account counts in the row hint

Example row shapes:

```text
6f8c…ab12 - testnet
6f8c…ab12 - testnet, staging-testnet
6f8c…ab12 (orphan)
```

`network delete` selects aliases, not partitions. Its interactive fallback should therefore use an alias multiselect, with each row showing the alias name and its genesis hash.

An orphaned partition is any `network_genesis_hash` appearing in identities/accounts that is not present in any configured network entry.

### 7. Destructive UX should be explicit, count-backed, and use cliclack warnings
Destructive flows should:
- emit a `cliclack` warning before confirmation
- describe whether the action removes config, data, or both
- include counts for affected identities and accounts where data may be removed or orphaned
- warn when `network delete` will orphan stored identities/accounts for one or more hashes
- require typed confirmation using the resolved label, labels, or genesis hash as appropriate

Examples:
- `network reset testnet` => warns that local wallet data for that network partition will be removed
- `network delete testnet staging-testnet` => warns that the selected aliases will be removed and that any resulting orphaned data must be cleaned up with `network reset`
- `seed delete main_seed` => warns that seed-owned identities/accounts will also be removed

### 8. Cross-store mutation complexity now lives primarily in `network reset`
Because `network delete` no longer prunes SQLite data, cross-store mutation becomes simpler.

- `network reset` mutates only SQLite data partitions
- `network delete` mutates config aliases and wallet-state cleanup

This reduces partial-failure risk for delete flows and keeps the config-vs-data split explicit.

### 9. Storage helpers should expose network-partition pruning directly
Because network partitions are not represented as a SQLite table, the store layer should expose explicit prune helpers for:
- deleting identities by `network_genesis_hash`
- deleting accounts by `network_genesis_hash`
- discovering distinct known hashes from stored identities/accounts
- discovering which config aliases map to a hash so reset rows can render `hash - alias1, alias2`

These helpers let CLI logic implement partition-oriented reset flows without faking relational ownership that does not exist in the schema.

## Risks / Trade-offs

- **[Deleting aliases can leave orphaned wallet data behind]** → Mitigation: warn explicitly, show affected identity/account counts, and make orphan cleanup discoverable via `network reset`.
- **[Interactive hash selection can become noisy]** → Mitigation: show configured aliases first and orphaned hashes in a separate labeled section or with explicit hints.
- **[Cross-store mutation is not globally atomic]** → Mitigation: choose a consistent mutation order and make partial-failure behavior explicit.
- **[Users may expect `network delete` to remove data too]** → Mitigation: keep reset/delete semantics sharply separated and reinforce them in warnings and docs.

## Migration Plan

1. Add CLI subcommands for `network reset`, `network delete`, and `seed delete`.
2. Remove or retire `seed remove` from the public CLI surface.
3. Add store helpers for pruning identities/accounts by `network_genesis_hash` and for discovering orphaned hashes.
4. Add config helpers for deleting aliases by name and listing aliases by genesis hash.
5. Implement confirmation/warning UX with affected-row counts and orphaning notices.
6. Add tests for multi-label delete semantics, alias multiselect behavior, partition-row rendering for reset, orphan-hash selection, and active-state cleanup.
7. Update docs and examples.

## Open Questions

- For `network delete` typed confirmation of multiple aliases, should the user confirm with a fixed token such as `delete`, or with an exact comma-joined alias list?
- For long alias lists in `network reset`, should the selector render every alias inline or collapse after a threshold such as `label1, label2 (+N more)`?
