## 1. SDK Compatibility

- [x] 1.1 Update existing lock configuration construction to set the new metadata field to `None` and preserve current lock encoding behavior
- [x] 1.2 Update test fixtures and explicit SDK structures for the new token-parameters and maximum-lock-duration fields

## 2. Chain-Parameter Command Surface and Input Resolution

- [x] 2.1 Add `node chain-parameters` with `--network`, `--node`, `--block`, `--json`, and `--no-defaults` matching comparable read-only command spelling, conflicts, environment behavior, and help text
- [x] 2.2 Reuse existing node endpoint resolution, active-network fallback, no-defaults network picker, and shared block-identifier parsing with last-final as the default
- [x] 2.3 Query all normalized chain parameters through gRPC, retain the resolved block hash, and add endpoint- and operation-specific error context
- [x] 2.4 Add clap and input-resolution tests covering explicit network/node conflicts, block selection, JSON mode, active-network defaults, and no-defaults behavior

## 3. Chain-Parameter Output

- [x] 3.1 Define documented wallet-owned serializable output types covering every normalized SDK chain-parameter, governance-key, and access-structure field with camel-case JSON names and explicit null protocol-version fields
- [x] 3.2 Implement precise conversion of SDK durations, amounts, rates, ratios, addresses, keys, thresholds, and indices into the output model, including `maxLockDuration` and `tokenParameters`
- [x] 3.3 Implement sectioned human rendering from the same output model and mode-pure pretty JSON output with endpoint and resolved block-hash context
- [x] 3.4 Add output-model, human-rendering, and JSON snapshot/shape tests for pre-P11 and P11 responses, including exact maximum lock duration and token-parameters authorization values

## 4. Maximum Lock Duration Governance Support

- [x] 4.1 Add token-parameters governance capability labels, access-structure lookup, key listing, signer eligibility, threshold resolution, and blind-sign family aliases
- [x] 4.2 Add the maximum-lock-duration sequence queue and map it to the node's dedicated next sequence number
- [x] 4.3 Map typed Max Lock Duration payloads to token-parameters authorization and maximum-lock-duration sequencing across all-in-one updates and detached proposals
- [x] 4.4 Recognize root and level-1 V3 level-2 authorization update variants in typed local signing paths
- [x] 4.5 Add actionable pre-device rejection for Max Lock Duration and authorization V3 in all-in-one and detached Ledger signing flows
- [x] 4.6 Add governance tests for JSON and serialized payload decoding, authorization and threshold selection, sequence resolution, review rendering, detached proposals, local signing, V3 recognition, and Ledger rejection
- [x] 4.7 Add `examples/governance-updates/max-lock-duration.json` with an editable Max Lock Duration payload and list it in the governance-update examples README

## 5. Documentation and Verification

- [x] 5.1 Update `docs/commands.md` with `node chain-parameters`, its exact flags, input/default behavior, block selection, and output modes
- [x] 5.2 Update README governance and node-query documentation for Max Lock Duration, token-parameters authorization, and current Ledger limitations
- [x] 5.3 Run `cargo fmt --check`, `cargo check`, focused tests, the full Rust test suite, and `cargo clippy --all-targets --all-features`

## 6. Chain-Update Inspection

- [x] 6.1 Add `node chain-updates` with the same `--network`, `--node`, `--block`, `--json`, and `--no-defaults` clap shape, conflicts, environment behavior, and help conventions as `node chain-parameters`
- [x] 6.2 Reuse endpoint resolution and block parsing, query next update sequence numbers at the requested selector, then query pending updates at the concrete block hash returned by that query
- [x] 6.3 Define documented wallet-owned output types containing endpoint, block hash, every next sequence-number queue, and ordered pending updates with effective times and typed or explicitly unknown effects
- [x] 6.4 Implement complete pending-update stream collection, mode-pure pretty JSON, scannable human sections, explicit empty-state output, and dataset-specific query errors
- [x] 6.5 Add clap, snapshot-consistency helper, output conversion, empty-state, P11 Max Lock Duration, unknown-effect, and rendering tests
- [x] 6.6 Update `docs/commands.md` and README with the exact `node chain-updates` command surface, snapshot semantics, and output contents
- [x] 6.7 Run formatting, checks, focused tests, workspace tests, clippy, and strict OpenSpec validation
