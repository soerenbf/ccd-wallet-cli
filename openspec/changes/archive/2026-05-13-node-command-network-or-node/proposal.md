## Why

Node commands currently require an explicit `--node <ENDPOINT>` on every invocation. Now that named networks can be registered in the config store, node commands should be able to accept `--network <NAME>` as an alternative, resolving the endpoint from the saved config. This removes the need to repeatedly type or export node URLs for known networks.

## What Changes

- Add `--network <NAME>` as an alternative to `--node <ENDPOINT>` for `node info`.
- When `--network` is supplied, the CLI looks up the registered network in `config.json` and uses its stored endpoint.
- Exactly one of `--network` or `--node` must be provided; supplying both or neither is an error.
- **BREAKING**: `--node` is no longer the sole way to specify a target for node commands.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `node-connectivity`: The node endpoint resolution requirement is changing — the CLI SHALL now accept either a named network or an explicit endpoint, with the named network resolving its endpoint from the durable config store.

## Impact

- `src/cli.rs`: `NodeInfoArgs` gains a `--network` flag; the two flags become mutually exclusive.
- `src/commands/node.rs`: handler resolves a `v2::Endpoint` from either the direct flag or the config store lookup.
- `src/store/config.rs`: read path exercised by node commands for the first time.
- No changes to `config network add` or the config file schema.
