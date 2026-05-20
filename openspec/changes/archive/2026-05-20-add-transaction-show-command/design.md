## Context

The CLI already supports several entity-oriented inspection commands such as `network show`, and it already submits transactions in account-creation and governance-update flows. Those flows surface transaction hashes, but there is no follow-up command that can inspect a transaction hash directly. The Concordium Rust SDK already exposes `get_block_item_status` and serde-backed transaction summary types, so the missing piece is mainly CLI integration and output design rather than new chain logic or local persistence.

This change also sits across a few existing command conventions:
- top-level command routing in the Clap CLI tree
- shared node/network resolution behavior
- human-oriented command output rather than raw debug dumps
- README examples and usage guidance

A key constraint is that transactions are not stored locally and should not become a new persisted wallet entity in this change. `transaction show` must therefore be a read-only node-backed inspection command whose meaning is "show what the selected node currently knows about this hash."

## Goals / Non-Goals

**Goals:**
- Add a top-level `transaction show <HASH>` command that inspects a transaction hash against a resolved Concordium node.
- Reuse the existing `--network`, `--node`, active-network defaulting, and `--no-defaults` behavior instead of inventing command-specific selection rules.
- Present stable fields such as transaction hash, query context, lifecycle status, block hash, block time, outcome, type, and energy in a curated human-readable layout.
- Render each concrete block-item summary variant explicitly and use JSON only for the nested non-static payloads within those variants.
- Treat node `not found` results as an `absent` transaction state in the command output.

**Non-Goals:**
- Persist transactions or transaction outcomes in SQLite.
- Add a separate `transaction status` command or machine-oriented output mode in this change.
- Fully normalize or reformat every possible Concordium transaction event variant inside the CLI.
- Infer transaction network from wallet-local state or from previously submitted transaction hashes.

## Decisions

### Add a top-level `transaction` command with a `show` subcommand
The command will be introduced as `ccd-wallet transaction show <HASH>` rather than as a nested node subcommand. This keeps the CLI aligned around entity-oriented verbs (`network show`, `seed show`) even though transaction inspection is node-backed rather than wallet-backed.

**Rationale:** users think in terms of inspecting a transaction entity, not invoking a low-level node RPC.

**Alternatives considered:**
- `node transaction status <HASH>`: technically accurate, but less aligned with the rest of the CLI's entity-oriented UX.
- `transaction status <HASH>`: a viable future brief command, but `show` better matches the desired detailed-by-default behavior.

### Reuse existing endpoint-resolution helpers and semantics
`transaction show` will use the same selector model as other node-backed commands: either `--network <NAME>`, `--node <ENDPOINT>`, or the active network by default unless `--no-defaults` is supplied. This keeps the transaction command behaviorally consistent with existing commands and avoids duplicating endpoint-selection logic.

**Rationale:** the repository already has tested resolution rules and user expectations around `--network`, `--node`, and `--no-defaults`.

**Alternatives considered:**
- Require `--network` or `--node` every time: safer, but inconsistent with existing CLI defaults.
- Try to infer the network from local transaction hashes: not reliable and out of scope because transactions are not stored locally.

### Treat transaction status as a CLI-owned shell with explicit variant rendering
The CLI will manually format only the stable outer structure:
- transaction hash
- queried network/node context
- lifecycle status (`received`, `committed`, `finalized`, `absent`)
- per-block block hash
- per-block block time
- per-block outcome headline
- per-block type and energy where available

For committed and finalized summaries, the CLI will fetch block info for each block hash so it can show block time as an RFC3339 UTC timestamp. It will then match on the concrete `BlockItemSummaryDetails` variant:
- account transactions: render static fields such as sender, sponsor, and cost, then show nested `events` or `rejectReason`
- credential deployments: render only static fields such as credential type, address, and registration id
- chain updates: render static fields such as effective time and update type, then show payload JSON
- token creation: render static fields from `CreatePlt`, then show token event JSON

This keeps the command explicit about the major transaction classes while still using JSON for the parts that are genuinely variable.

**Rationale:** this keeps the command readable while avoiding a large formatting surface that must track every Concordium transaction variant.

**Alternatives considered:**
- Raw `Debug` output of `TransactionStatus` or `BlockItemSummary`: easy, but noisy and not intentionally designed.
- Fully hand-format all transaction outcomes: high maintenance cost and brittle as SDK variants evolve.
- Emit only JSON: useful for automation, but weaker for normal CLI inspection.

### Model node `NotFound` as `absent` output instead of command failure
When the selected node does not know the hash, the command will render `Status: absent` and an explanatory note rather than surfacing the underlying query error directly.

**Rationale:** `transaction show` is an inspection command, and `absent` is a meaningful domain result. This also matches Concordium's documented transaction lifecycle language.

**Alternatives considered:**
- Bubble up the SDK `NotFound` error unchanged: simpler, but a worse user experience.

### Render committed transactions as a list of per-block outcomes
The SDK models committed and finalized statuses as maps from block hash to block summary. `transaction show` will preserve this shape conceptually by rendering one subsection per block in committed status, even if there is usually only one block.

**Rationale:** this matches the SDK's data model and avoids silently discarding edge-case information.

**Alternatives considered:**
- Flatten committed status to a single arbitrary block: simpler, but risks hiding useful information.

## Risks / Trade-offs

- **[Network ambiguity]** A hash queried against the wrong network can appear `absent` even if it exists elsewhere. → **Mitigation:** always show the queried network/node context and include a note for absent results.
- **[Verbose output]** Detailed-by-default output may be more than some users want for quick checks. → **Mitigation:** keep the stable shell concise and limit JSON dumping to committed/finalized cases only.
- **[SDK JSON shape drift]** Nested payload rendering depends on the SDK's serde representation of account-transaction events, update payloads, token events, and reject reasons. → **Mitigation:** keep the CLI responsible only for stable top-level and variant-level fields, and use JSON only for the nested payloads.
- **[Selector inconsistency risk]** Reimplementing endpoint resolution locally could diverge from other commands. → **Mitigation:** reuse existing endpoint-resolution helpers where possible.

## Migration Plan

- Add the new top-level CLI command and command handler without changing existing commands.
- Update README examples and command descriptions to document `transaction show`.
- No database migration or persisted-state migration is required.
- Rollback is straightforward: removing the command does not affect stored wallet data.

## Open Questions

- None. This change will render the four concrete block-item summary variants explicitly and use JSON only for their nested non-static payloads, while brief or machine-oriented modes remain explicitly out of scope.
