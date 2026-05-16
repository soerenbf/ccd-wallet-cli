## Why

The wallet can accumulate long-lived local state across seeds and networks, but it still lacks a coherent destructive lifecycle for cleaning that state up. Today seed removal exists under `seed remove`, but the command family is inconsistent, the delete target can still feel underspecified, and there is no first-class way to prune all wallet-local data for a network while keeping the network configured.

This becomes more important now that recovery can import many identities and accounts across multiple providers. Users need a clean way to:
- delete a stored seed and all seed-owned wallet data
- reset a network's wallet-local identities and accounts without unregistering the network
- delete one or more network aliases without silently touching unrelated local data
- target destructive actions explicitly rather than through active/default context

## What Changes

- Replace `seed remove` with `seed delete`.
- Make `seed delete` always resolve its target from an explicit label or an interactive selector, never from active seed state.
- Define `seed delete` as destructive removal of the seed plus all seed-owned identities and accounts.
- Add `network reset` to prune wallet-local identities and accounts for a resolved network while keeping config entries intact.
- Add `network delete` to remove one or more network config aliases by label.
- Support network reset targeting by either configured label or explicit `--genesis-hash <HASH>`.
- Allow interactive network reset selection to include configured network-data partitions and orphaned partitions discoverable only by genesis hash.
- Render interactive network reset rows as partition-oriented entries that always show the genesis hash, followed by matching aliases when present, or `(orphan)` when none exist.
- Make `network delete` alias-oriented: it removes only the selected config aliases, never prunes identities/accounts, and warns when the deletion will orphan existing network-local wallet data.
- Add explicit destructive warnings and confirmation prompts for seed/network delete/reset flows, including identity/account counts.
- Clear active seed/network state only as cleanup after deletion when the deleted alias/seed had been active; never use active state to infer destructive targets.

## Capabilities

### New Capabilities
- `network-reset-delete`: Explicit destructive lifecycle commands for network-scoped wallet data and network config aliases.

### Modified Capabilities
- `seed-command`: Replace `seed remove` with `seed delete` and tighten destructive target resolution semantics.
- `seed-storage`: Seed deletion semantics continue to cascade through seed-owned rows, now as part of the `seed delete` contract.
- `config-storage`: Network config now supports deleting one or more aliases by label.
- `identity-storage`: Identity rows can be pruned by network partition in addition to existing seed-owned cascade deletion.
- `account-storage`: Account rows can be pruned by network partition in addition to existing seed-owned cascade deletion.
- `interactive-cli-prompts`: Destructive reset/delete flows use cliclack warnings, confirmations, and selectors that can include orphaned network hashes.
- `wallet-state`: Destructive deletion flows clear stale active seed/network pointers when needed.

## Impact

- Affected code: `crates/ccd-wallet/src/cli.rs`, `crates/ccd-wallet/src/commands/{config/network,seed}.rs`, and store helpers in `crates/ccd-wallet-core/src/store/{config,seeds,identities,accounts,wallet_state}.rs`.
- Affected systems: JSON config alias management, SQLite network-partition pruning, seed-owned cascade deletion, and interactive destructive confirmation UX.
- User-facing behavior: new `network reset`, new `network delete`, `seed delete` replacing `seed remove`, partition-oriented reset targeting, alias-oriented network deletion, and stronger destructive warnings with count summaries and orphaning notices.
