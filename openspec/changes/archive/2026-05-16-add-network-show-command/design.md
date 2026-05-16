## Context

The wallet already has two nearby pieces of functionality:

- configured network management under `network ...`
- generic live node inspection under `node info`

What is missing is a focused way to inspect a network as a user-facing entity. There are two valid entry points for that inspection:

1. **config-first**: the user has a configured network alias and wants to inspect that network together with live consensus information
2. **node-first**: the user has a raw node endpoint and wants to know which configured network aliases, if any, match the observed chain identity

The Concordium node exposes the relevant network identity through consensus information, specifically the observed genesis hash. That makes the node the authoritative source for identifying the chain behind an endpoint. At the same time, `network show` still belongs in the network command family, so label-based invocations should preserve the configured network view as the primary entity shown to the user.

## Goals / Non-Goals

**Goals:**
- Add `network show` as a read-only inspection command for both configured-network and raw-node workflows.
- Support `network show`, `network show <LABEL>`, `network show --node <ENDPOINT>`, and `network show <LABEL> --node <ENDPOINT>`.
- Make bare `network show` use the active network only in config mode.
- Make `network show --node <ENDPOINT>` node-first and avoid silently deriving any configured network context from the active network.
- Render output in a human-oriented way with distinct sections for configured network details or matching configured aliases, plus consensus information from the queried node.
- In raw-node mode, keep network-match rendering compact and reserve richer configuration detail for explicit label mode.
- Surface mismatch diagnostics when a label-selected configured network does not match the observed genesis hash returned by the queried node.

**Non-Goals:**
- Redesigning `node info` or removing it.
- Adding machine-readable `--json` output in this change.
- Querying wallet proxy metadata beyond showing the configured `wallet_proxy` value for label-based mode.
- Adding account or identity inspection in the same change.

## Decisions

### 1. `network show` has two output modes based on how the target is resolved
The command will intentionally render differently depending on whether the user selected a configured network label or a raw node endpoint.

- **Config mode** (`network show`, `network show <LABEL>`, `network show <LABEL> --node <ENDPOINT>`): show `Network configuration` first, then `Consensus (<node endpoint>)`.
- **Node mode** (`network show --node <ENDPOINT>` with no explicit label): show `Network match (<genesis hash>)` or `Network matches (<genesis hash>)` first, then `Consensus (<node endpoint>)`.

**Rationale:** The user's entry point determines the primary entity being inspected. A label-based invocation is about a configured network alias; a node-only invocation is about discovering which network identity a raw endpoint belongs to.

**Alternatives considered:**
- **Single universal layout for all invocation forms**: rejected because it would either bury network configuration in label mode or over-emphasize config in raw-node mode.

### 2. Node resolution happens first, but rendering remains network-oriented
Regardless of rendering mode, the command first resolves a query endpoint and requests consensus information from that node. The observed genesis hash from the node is then used to identify matching configured networks.

- In config mode, the selected configuration remains the primary section shown.
- In node mode, the observed genesis hash drives the top section and configured aliases are shown only as matches.

**Rationale:** This keeps live chain identity authoritative while still letting the CLI present the most relevant user-facing entity first.

### 3. Active-network fallback applies only in config mode
Bare `network show` uses the active network when defaults are allowed. `network show --node <ENDPOINT>` does not use the active network implicitly because the user has chosen node-only inspection mode.

Resolution rules:
- explicit label => config mode
- explicit `--node` with no label => node mode
- neither => active network or interactive config selection, depending on defaults

**Rationale:** This preserves current-context ergonomics for the network family while preventing `--node` from quietly picking an unrelated configured network.

**Alternatives considered:**
- **Always require explicit input**: rejected because plain `network show` is a natural active-network inspection command.
- **Always combine `--node` with active network when no label is supplied**: rejected because it creates surprising mixed-mode behavior.

### 4. `--node` acts as a diagnostic query override in config mode
`network show <LABEL> --node <ENDPOINT>` is allowed. The command keeps the configured network section from `<LABEL>` but queries consensus from the explicit endpoint.

If the observed genesis hash does not match the configured network's stored genesis hash, the output includes a mismatch warning.

**Rationale:** This is useful for diagnosing stale config, wrong endpoints, and ambiguous infrastructure without needing a separate compare command.

### 5. Top-level network match rendering is compact in node-only mode
For node-only mode, the top section uses the observed genesis hash in the heading:

- `Network match (<genesis hash>)`
- `Network matches (<genesis hash>)`

If multiple configured aliases match, the command renders them compactly as:
- `testnet (<endpoint>)`
- `other_testnet (<endpoint>)`

If there is exactly one match, the same heading style can be singular while still keeping the row compact. Richer per-alias detail such as wallet proxy belongs to config mode via `network show <LABEL>`.

If no configured aliases match, the section should still render in the same place with an explicit no-match summary rather than omitting the network section entirely.

**Rationale:** Node-only mode is about identifying matching configured aliases, not fully rendering each one.

### 6. Consensus output should include the queried endpoint in the section heading
The command renders consensus as:

- `Consensus (<node endpoint>)`

The section then contains curated consensus fields, including at minimum the observed genesis hash and other human-useful fields such as protocol version and best/finalized block information. The queried endpoint is not shown as a separate top-level section.

**Rationale:** The node endpoint is provenance for the consensus view. Folding it into the heading avoids a redundant separate `Node used` section.

### 7. The command should favor a curated consensus summary over a raw debug dump
The first cut should extract and format a small set of meaningful consensus fields instead of printing the entire Rust debug representation.

Initial candidates:
- observed genesis hash
- protocol version
- best block
- best finalized block

Additional fields can be added later if they are clearly valuable.

**Rationale:** `network show` is a human-oriented inspection command, not a low-level dump.

## Risks / Trade-offs

- **[Consensus info may still feel too thin or too verbose]** → Mitigation: start with a curated minimal subset and leave room for a later `--verbose` mode if needed.
- **[Config mode plus `--node` can expose mismatches users did not expect]** → Mitigation: make mismatch warnings explicit and frame them as diagnostics, not silent failures.
- **[Multiple matching aliases may tempt richer rendering in node mode]** → Mitigation: keep node-only match output compact and reserve richer config detail for explicit label mode.
- **[Active-default behavior can still confuse users]** → Mitigation: keep `--node` node-only unless a label is explicitly supplied, and preserve existing actionable missing-active-network errors for bare `network show`.

## Migration Plan

1. Extend the CLI with `network show` arguments.
2. Reuse or extract shared node-endpoint resolution logic where it helps.
3. Add consensus querying and configured-network matching logic in the network command module.
4. Implement the two rendering modes and mismatch warnings.
5. Add command-level tests for active-default config mode, node-only mode, and explicit-label-plus-override mode.
6. Update README and command documentation.

## Open Questions

- Should the first cut include only a curated consensus summary, or also support an explicit raw/verbose mode?
- Which exact consensus fields beyond genesis hash should be included in the initial human-oriented summary?
