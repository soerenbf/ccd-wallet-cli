## Context

`node info` currently takes a mandatory `--node <ENDPOINT>` flag. The config store introduced in the previous change allows users to register named networks with a stored endpoint. This change makes node commands network-aware by accepting `--network <NAME>` as an alternative to `--node`, resolving the endpoint from config.

## Goals / Non-Goals

**Goals:**
- Accept `--network <NAME>` or `--node <ENDPOINT>` on `node info` (exactly one required).
- Look up the network name in `~/.config/ccd-wallet/config.json` and extract its `node_endpoint`.
- Fail clearly when `--network` names an unregistered network.
- Fail clearly when neither or both flags are provided.

**Non-Goals:**
- Applying this pattern to commands other than `node info` in this change.
- Adding a global `--network` / `--node` flag at the root CLI level.
- Any changes to the config file schema or the `config network add` command.

## Decisions

### Use clap's `conflicts_with` to make the flags mutually exclusive
`--network` and `--node` will each carry `#[arg(conflicts_with = "...")]` so clap enforces mutual exclusivity and produces a clear error message at parse time.

- **Why:** Keeps validation in the CLI layer rather than the handler; clap's conflict error messages are user-friendly out of the box.
- **Alternative considered:** An enum argument (`NodeOrNetwork`). Rejected as unnecessarily complex for two flags — conflicts_with is idiomatic clap for this pattern.

### Both flags are `Option<_>`; the handler requires exactly one
Each flag is `Option<T>`. The handler uses a `match (args.network, args.node)` to enforce that exactly one is present, returning an error if neither is supplied.

- **Why:** clap's `required_unless_present` combined with `conflicts_with` handles the "at least one" constraint at parse time, but expressing it through Option makes the handler logic explicit and testable.
- **Alternative considered:** `required = true` on both with group logic. Rejected because clap argument groups for "one of" are more verbose and harder to read than explicit Option matching.

### Endpoint resolution is a shared helper
A free function `resolve_endpoint(network: Option<String>, node: Option<v2::Endpoint>) -> Result<v2::Endpoint>` in `src/commands/node.rs` (or a shared location) centralises the lookup logic so future commands can reuse it.

- **Why:** This pattern will recur for every command that needs a node connection. Extracting it now keeps handlers lean.

## Risks / Trade-offs

- **[Unknown network name]** The user supplies `--network foo` but `foo` is not in config. → Mitigation: fail with a clear error listing that the network is not registered, prompting the user to run `config network add`.
- **[Stale endpoint in config]** The stored `node_endpoint` may no longer be reachable. → Mitigation: connection errors already surface actionable messages; no special handling needed.
