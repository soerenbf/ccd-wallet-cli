## Why

The wallet can register, list, rename, select, reset, and delete networks, but it still lacks a focused way to inspect a configured network or to identify what network a raw node endpoint belongs to. Users currently have to piece this together from `network list` and `node info`, which is awkward when diagnosing mismatches, checking consensus state, or verifying whether a node corresponds to a known configured network.

## What Changes

- Add a `network show` command for inspecting network details.
- Support config-first `network show [NAME]` behavior that shows the selected network configuration together with consensus information queried from that network's configured node endpoint.
- Make bare `network show` use the active network when defaults are allowed.
- Support node-first `network show --node <ENDPOINT>` behavior that queries a raw node endpoint, derives the observed genesis hash, and reports matching configured network aliases, if any, before rendering consensus details.
- Support `network show [NAME] --node <ENDPOINT>` as a diagnostic override mode that keeps the selected network configuration visible while querying consensus from the explicit endpoint.
- Render the command in a human-oriented way with `Network configuration` in label/active mode, `Network match(es) (<genesis hash>)` in raw-node mode, and `Consensus (<node endpoint>)` in both modes.
- Surface mismatch diagnostics when a selected configured network does not match the observed genesis hash from the queried node.

## Capabilities

### New Capabilities
- `network-show`: Show configured network details and/or network matches together with consensus information from a queried node.

### Modified Capabilities
- `node-connectivity`: Extend node resolution behavior to cover `network show`, including active-network defaults for config mode, explicit `--node` override mode, and mutual-exclusion/override semantics.

## Impact

- Affected code: `crates/ccd-wallet/src/cli.rs`, `crates/ccd-wallet/src/commands/config/network.rs`, and shared node-resolution/query helpers where reuse makes sense.
- Affected systems: configured network lookup, active-network default resolution, node connectivity/query logic, and human-oriented CLI rendering.
- User-facing behavior: new `network show` command with active-default config mode, explicit-label config mode, and raw-node mode; explicit reporting of matching configured networks for a queried node endpoint; compact alias-plus-endpoint rows in raw-node mode; and clearer consensus-based network diagnostics.
