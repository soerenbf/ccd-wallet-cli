## Context

The wallet already has several local entity types—networks, seeds, identities, and accounts—but the CLI is currently asymmetric: users can create or select some of them, yet there is no consistent way to inspect what exists or to relabel entries once created. The codebase also splits these entities across two storage models: networks live in the durable JSON config file, while seeds, identities, and accounts live in SQLite.

This change is cross-cutting because it touches CLI shape, interactive selection UX, config persistence, SQLite query/update APIs, and wallet-state handling for active selections. It also introduces a nuanced scope-and-filter model for `identity list` and `account list`: by default they follow the active seed and active network, but they can be broadened with explicit scope values such as `--seed all` and `--network all`, then narrowed further with entity-specific filters such as identity provider.

## Goals / Non-Goals

**Goals:**
- Add human-oriented `list` commands for networks, seeds, identities, and accounts.
- Add `rename` commands for networks, seeds, identities, and accounts.
- Keep identity/account list commands aligned with existing context-bearing behavior: active seed and active network are used by default, can be overridden explicitly, and are shown as resolved context.
- Support explicit `all` scope values for `--seed` and `--network` on identity/account list commands.
- Support additional relevant list filters, including identity provider id for identity listings and status filters for identity/account listings.
- Keep account addresses hidden by default and reveal them only when explicitly requested.
- Preserve stable underlying identities when renaming: only user-facing labels/names change.
- Update active network/seed state when renaming the active entry.

**Non-Goals:**
- Adding machine-readable `--json` output in this change.
- Redesigning creation flows (`add`, `new`, `use`) beyond any shared selector/context improvements needed for consistency.
- Adding machine-readable `--json` output in this change.
- Redesigning creation flows (`add`, `new`, `use`) beyond any shared selector/context improvements needed for consistency.
- Applying `all` wildcard scope values to rename commands.
- Requiring identity/account rename to follow active/default seed+network context when the source is omitted.

## Decisions

### 1. `list` and `rename` are entity-family commands, not a new global mode
The new behavior will be expressed as subcommands under each entity family:
- `network list`, `network rename`
- `seed list`, `seed rename`
- `identity list`, `identity rename`
- `account list`, `account rename`

**Rationale:** This matches the CLI's current mental model, keeps discoverability high, and avoids introducing a separate management namespace.

**Alternatives considered:**
- **Top-level generic commands (`list`, `rename`)**: rejected because it breaks the existing command hierarchy and forces entity selection into a second step.

### 2. Identity/account list use context-bearing scope resolution plus explicit scope values and filters
`identity list` and `account list` will resolve seed and network scope using the same default/override model as existing context-bearing commands. In addition to concrete labels, the list commands will accept explicit wildcard scope values such as `--seed all` and `--network all`. Once scope is resolved, the command may apply additional entity-specific filters.

For the first cut:
- `identity list` supports `--provider` and `--status`
- `account list` supports `--status`

**Rationale:** This preserves the current-context-first model while making broader queries concise and composable, e.g. “all identities on testnet from provider 0” or “all pending accounts on testnet”.

**Alternatives considered:**
- **Separate `--all-*` flags**: rejected because they multiply the scope surface without improving the core model.
- **Interactive-only `All ...` rows with no explicit argument form**: rejected because users also need the same scope broadening in non-interactive and argument-driven flows.

### 3. Identity/account rename use global fuzzy search when the source is omitted
`identity rename` and `account rename` will still support explicit source labels, but when the source is omitted they will use a global fuzzy selector instead of active/default seed+network scope resolution. The searchable text and displayed rows will include enough metadata to disambiguate results, including network and seed labels and other relevant fields such as provider id, identity index, account status, or credential counter.

The selector rows are label-first. A status badge is shown only when the entity is not in its normal happy state:
- identities: show `[pending]` or `[expired]`, but no badge for normal done-and-unexpired identities
- accounts: show `[pending]`, but no badge for finalized accounts

The metadata hint line carries the disambiguating fields used for visual scanning, while fuzzy search also indexes hidden search tokens so queries like `testnet`, `seed:test`, or `provider:0` can match rows even when those words are not part of the primary label.

**Rationale:** Rename is a “find one thing and mutate it” workflow, not a “browse the current working set” workflow. A global fuzzy search is a better fit than hidden scope defaults and lets searches like `testnet` surface all matching identities/accounts.

**Alternatives considered:**
- **Context-scoped rename selection**: rejected because it hides valid matches outside the current context and makes rename less discoverable in larger wallets.
- **`All ...` scope values for rename**: rejected because fuzzy search is a better search model than broadening rename scope with wildcard arguments.

### 4. Renames update labels only; stable identities stay unchanged
Rename operations will change only human-facing labels/names. Stable identities remain intact:
- network rename moves the config entry key but preserves the stored value
- seed rename preserves `seed_id`
- identity rename preserves `(network_genesis_hash, seed_id, ip_identity, identity_index)`
- account rename preserves `(network_genesis_hash, seed_id, ip_identity, identity_index, credential_counter)`

**Rationale:** This keeps rename safe and intuitive.

### 5. Active state follows active entry renames
If the user renames the active seed or active network, the corresponding wallet-state key will be updated to the new label/name.

**Rationale:** Active state is user-facing context and should continue to point at the same logical entity after a rename.

### 6. Account addresses stay hidden unless explicitly requested
`account list` will display only plaintext metadata by default. A flag such as `--show-addresses` will opt into address display. If addresses are requested, the wallet may need to unlock one or more seed domains depending on the chosen scope.

`account rename` may also support `--show-addresses`, but unlike list it cannot remain fully global in that mode. Because addresses are encrypted under the seed password domain, enabling address display for account rename requires a concrete seed scope chosen either by explicit argument or by an interactive seed selector before the fuzzy account picker is shown.

**Rationale:** This preserves the current privacy boundary while still allowing a deliberate reveal path. For rename, requiring a single-seed scope avoids awkward multi-seed decryption and multiple unlock prompts inside one fuzzy search flow.

**Alternatives considered:**
- **Always show addresses**: rejected because account addresses are intentionally stored inside encrypted payloads.
- **Never show addresses in list output or rename selectors**: rejected because explicit reveal is useful for human inspection.
- **Allow address display across all seeds during account rename**: rejected because it would force confusing cross-seed unlock behavior.

## Risks / Trade-offs

- **[Interactive list scope becomes too implicit]** → Mitigation: always show the resolved context before rendering results, including `seed: all` / `network: all` when selected.
- **[Global fuzzy rename selection can surface many similar entries]** → Mitigation: include network, seed, and other disambiguating metadata in the searchable row text and displayed picker labels.
- **[Account address reveal across `All seeds` scope can require multiple unlocks]** → Mitigation: make address reveal opt-in and keep default list output metadata-only.
- **[Network rename is structurally different from SQLite renames]** → Mitigation: treat config-key move semantics as a first-class design decision and cover active-state updates explicitly.

## Migration Plan

1. Extend CLI subcommand enums for `list` and `rename` across all four entity families.
2. Add store/config helpers for list and rename operations.
3. Implement list scope parsing for concrete labels and explicit `all` values, plus entity-specific filters.
4. Implement fuzzy searchable rename selection for identities and accounts.
5. Add human-oriented output formatting for list commands.
6. Update active wallet state on active seed/network renames.
7. Add documentation and tests.

Rollback is limited to local CLI behavior and local data stores. SQLite schema changes are not necessarily required for this change; most work should be query/update logic on existing structures.

## Open Questions

- Should interactive list scope pickers also include visible `All ...` rows, or is explicit `--seed all` / `--network all` sufficient for the first cut?
- How should `account list --show-addresses` behave when the result set spans multiple seeds: prompt once per seed lazily, or require a narrower scope first?
- Should the fuzzy selector metadata hint be rendered as a single compact line or a two-line layout if the chosen fuzzy UI implementation supports it?
