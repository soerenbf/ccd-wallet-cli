## Context

`ccd-wallet transaction show` currently resolves a node endpoint, calls `get_block_item_status`, fetches block times for committed/finalized results, and renders a human-oriented summary from `TransactionStatus` plus `BlockItemSummary`. That flow is intentionally summary-oriented: it tells the user what happened, but not the exact block item that was submitted.

The requested change adds an opt-in way to inspect the original transaction payload without disturbing the default output. The main technical constraint is that transaction status and transaction payload come from different node APIs. `get_block_item_status` provides lifecycle state and per-block summaries, while payload retrieval requires fetching block items from the relevant block and matching by hash. This means payload display is only available once the transaction is present in one or more blocks.

The change touches clap argument parsing, transaction query orchestration, and human-oriented rendering. It also needs careful output design because the current renderer already uses a `Payload:` section for some summary-derived data such as chain updates.

## Goals / Non-Goals

**Goals:**
- Add an explicit `--show-payload` flag to `transaction show`.
- Keep existing `transaction show` behavior unchanged when the flag is omitted.
- Retrieve the original submitted block item payload for committed and finalized transactions.
- Present the original payload in a way that is distinguishable from summary-derived sections already shown today.
- Provide actionable output when payload retrieval is unavailable, such as absent or received transactions.
- Keep the implementation forward-compatible by tolerating undecodable or unfamiliar payload formats.

**Non-Goals:**
- Changing the default `transaction show` layout for users who do not opt in.
- Replacing the existing summary-oriented renderer with a raw node dump.
- Guaranteeing a fully human-friendly decode for every possible block item type.
- Adding new top-level transaction inspection commands or JSON output modes in this change.
- Retrieving payloads for transactions that are only known as `received` and not yet attached to a block.

## Decisions

### Use `get_block_item_status` to discover candidate blocks, then fetch block items from those blocks
The command should continue to use `get_block_item_status` as the primary query because it is already the canonical source for transaction lifecycle state and summary output. When `--show-payload` is requested and the status is committed or finalized, the command should additionally fetch block items for the block hashes named by the status and find the matching block item by hash.

This keeps one status-oriented entrypoint and avoids inventing a separate payload-only lookup path.

**Alternatives considered:**
- **Use only `get_finalized_block_item` for finalized transactions**: simpler for finalized-only support, but it does not cover committed transactions and would force split logic.
- **Always fetch block items even without `--show-payload`**: unnecessary extra node work for the default case.

### Support payload display for both committed and finalized transactions
If the node reports committed or finalized status, the transaction is already present in one or more blocks, so the command has enough information to attempt block-item retrieval. The flag should therefore work for both states rather than only finalized.

For committed transactions with multiple candidate blocks, the command should show the payload for each matching block section so the output remains aligned with the existing per-block rendering model.

**Alternatives considered:**
- **Finalized-only payload support**: simpler, but unnecessarily limits a diagnostic flag that can often work earlier.

### Keep payload display opt-in and introduce a distinct section label
The default output should remain unchanged to preserve the current concise behavior. When the flag is present, the renderer should add a clearly named section such as `Submitted payload:` or `Original transaction payload:` rather than reusing the existing `Payload:` label.

This prevents confusion between:
- summary-derived payload information already available for some transaction summaries, and
- the original submitted block item payload now being retrieved from block contents.

**Alternatives considered:**
- **Always show payload**: increases noise and may surprise users.
- **Reuse `Payload:` everywhere**: risks ambiguity because the command already emits a summary-derived `Payload:` section for chain updates.

### Prefer structured rendering when decoding succeeds, with stable fallback when it does not
For account transactions, the block item payload should be decoded from encoded bytes when possible and rendered in a structured form. If decoding fails, the implementation should still show a stable fallback such as hex-encoded bytes instead of failing the whole command.

For other block item kinds, the renderer should show the richest stable representation available under the current SDK types. The command should treat payload display as best-effort diagnostic output layered on top of the existing status query, not as a prerequisite for the base command to succeed.

**Alternatives considered:**
- **Fail the command if payload decoding fails**: too brittle for a diagnostic flag.
- **Always show only raw hex**: robust but much less useful for common account transactions.

### Keep summary rendering and payload rendering as separate concerns
The existing summary renderer already handles status metadata, block metadata, and summary-specific detail sections. The new payload logic should be added as a separate rendering input rather than folding block-item retrieval into the summary model.

This keeps responsibilities clear:
- status query + block times
- optional block-item lookup
- rendering of summary sections
- rendering of optional submitted-payload sections

**Alternatives considered:**
- **Merge payload lookup directly into summary rendering internals**: makes the renderer responsible for more node-derived state than it needs and complicates testing.

## Risks / Trade-offs

- **Extra node queries when `--show-payload` is used** → Limit block-item fetching to the opt-in path and only for blocks named by the returned status.
- **Committed transactions may appear in multiple blocks** → Preserve the existing per-block output model and attach the retrieved payload to the matching block section rather than forcing a single combined payload view.
- **Some payloads may be hard to decode or noisy to render** → Use structured output when possible and fall back to a stable raw representation when not.
- **Terminology confusion between submitted payload and summary payload** → Use distinct section headings and document the behavior in the spec/tests.
- **Forward-compatibility with newer node payload variants** → Avoid assuming every payload can be decoded into known SDK structures; tolerate unknown or undecodable forms.

## Migration Plan

No data migration is required.

Implementation can ship as a normal CLI enhancement:
1. Add the new flag to `transaction show`.
2. Extend the query flow to retrieve matching block items only when the flag is set.
3. Extend rendering to show submitted payload sections and unavailability notes.
4. Add command parsing tests and rendering/query tests for absent, received, committed, and finalized cases.

Rollback is low risk because the feature is additive and opt-in. Removing the flag and optional lookup path would restore the previous behavior.

## Open Questions

- What exact section heading should be used in the final UX: `Submitted payload:` or `Original transaction payload:`?
- For committed transactions with multiple matching block sections, should repeated identical payload rendering be shown in every block section, or should one shared payload section be printed after the summaries?
- For credential deployments and update instructions, is pretty-printed structured output already available under the project’s current SDK feature set, or will those types need a simpler fallback renderer?
