## Context

The wallet now has the two main prerequisites for governance update submission: a local governance-key vault and live chain-parameter inspection. What is still missing is the command that turns an update payload plus local governance keys into a signed and submitted update instruction.

This change crosses several concerns:
- CLI payload ingestion for both structured and opaque inputs
- live node queries for chain parameters, key-index resolution, update-sequence numbers, submission, and optional finalization waiting
- signer selection UX built on top of the governance-key-management flows already implemented
- safe handling of future serialized update payloads that the wallet may not yet understand

The CLI should remain explicit and operator-oriented. Governance signing is more sensitive than ordinary account transaction submission, so the wallet should avoid silently choosing signers or silently inventing expiry values.

## Goals / Non-Goals

**Goals:**
- Add `governance update` that accepts JSON payload files and serialized hex payloads.
- Support interactive paste fallback for JSON and serialized payloads when the payload is omitted.
- Support Create PLT JSON authoring where `initializationParameters` may be written as JSON and converted to Concordium CBOR internally.
- Deserialize known payloads to determine update type, authorization family, threshold, and default sequence-number queue.
- Support explicit signer selection via repeatable `--key <VERIFY_KEY>` flags and interactive fuzzy multiselect when keys are omitted.
- Reuse governance-key presentation patterns for signer prompts, including compact verify keys and capability-aware row summaries.
- Support blind signing of unknown serialized payloads with strong warnings.
- Allow optional `--sign-as <AUTH_FAMILY>` as a helper for blind signing so the wallet can still derive threshold, eligible keys, and default sequence numbers when the operator knows the authorization family even if the payload is unknown.
- Support explicit `--sequence-number <N>` override.
- Support effective-time and timeout parsing from relative durations, RFC3339 timestamps, and unix seconds.
- Prompt for omitted effective time with default `0`, prompt for omitted timeout with a derived five-minute default, and wait for finalization by default, with `--no-wait` to return after submission.

**Non-Goals:**
- Adding non-interactive automation in this first cut.
- Supporting every possible future chain update type with first-class decoded display on day one.
- Hiding signer choice behind automatic key selection.
- Introducing a local cached authorization snapshot to avoid node queries.

## Decisions

### Create PLT JSON has a CBOR convenience path

Create PLT is a special case because its `initializationParameters` field is protocol-level-token module data represented on chain as raw CBOR bytes. JSON mode SHALL accept the SDK-native hex string representation for this field, and MAY also accept a JSON value in `initializationParameters` or `initializationParametersJson`. When JSON is supplied, the wallet SHALL convert only that field to CBOR using Concordium's CBOR implementation before deserializing the overall `UpdatePayload`.

This convenience SHALL NOT apply to other update types.

### One command, two payload modes, one internal submission pipeline

`governance update --json` and `governance update --serialized` SHALL feed the same internal pipeline:
1. ingest payload,
2. attempt deserialization,
3. resolve signer-selection context,
4. build update instruction,
5. sign,
6. submit,
7. optionally wait for finalization.

This keeps the command surface small while still allowing JSON authoring workflows and raw serialized workflows.

Alternatives considered:
- **Separate commands for decoded vs blind signing**: clearer internally, but duplicates most of the UX and submission pipeline.
- **Typed subcommands only**: nicer for common cases, but does not solve forward compatibility or payload reuse from external tooling.

### Explicit signer choice is the default UX

The wallet SHALL not silently auto-pick governance signers. The operator can provide repeatable `--key <VERIFY_KEY>` flags, or omit them in interactive mode and choose keys through a fuzzy multiselect prompt.

When the authorization structure and threshold are known, the prompt SHALL preselect authorized local keys up to the required threshold.

Alternatives considered:
- **Auto-select the first threshold-satisfying key set**: faster, but too implicit for governance.
- **Always require explicit CLI flags**: safe, but too cumbersome for interactive operators.

### Blind signing remains network-aware, not fully offline

Unknown serialized payloads SHALL still be signable, but the wallet may query the node for current chain parameters to resolve selected verify keys to governance key indices. Blind signing in this design means “payload semantics unknown”, not “zero network access”.

This avoids forcing the user to provide raw key indices while keeping the payload itself opaque to the wallet.

Alternatives considered:
- **Fully offline blind signing**: would require extra user-supplied key-index mapping and produces a much rougher UX.
- **Refuse unknown serialized payloads**: safer but loses forward compatibility.

### `--sign-as` is an optional helper, not a required routing flag

Blind signing SHALL support an optional `--sign-as <AUTH_FAMILY>` hint. If provided, the wallet can:
- filter to eligible local keys,
- validate threshold satisfaction,
- preselect threshold-sized signer sets,
- derive the next sequence number unless explicitly overridden.

If omitted, blind signing SHALL still work when the user supplies keys and an explicit sequence number, but the wallet cannot fully validate authorization-family-specific threshold behavior.

Alternatives considered:
- **Require `--sign-as` for every unknown payload**: simpler validation model, but too restrictive for expert users who just want to sign with selected keys and an explicit sequence number.

### Effective time and timeout are prompted when omitted

Effective time and timeout SHALL accept three formats:
- relative duration (`5m`, `30m`, `1h`, `15d`),
- RFC3339,
- unix seconds.

Effective time is optional. If the operator does not provide it, the CLI SHALL prompt for it with a default value of `0`, which is the protocol sentinel for immediate execution in update headers.

Timeout is also promptable. If the operator does not provide it, the CLI SHALL prompt for it with a default value derived from the effective time:
- if effective time is `0`, default timeout is five minutes from now,
- otherwise, default timeout is five minutes before the effective time.

The timeout prompt SHALL display the derived default in RFC3339 format even though unix seconds remain accepted as input.

The wallet SHALL validate:
- timeout is in the future,
- if effective time is nonzero, timeout is not after effective time.

Node-side semantic rejection for update-type-specific timing rules remains acceptable.

Alternatives considered:
- **Require timeout only through explicit CLI input**: simpler to reason about, but misses a good interactive default.
- **Treat every time input as RFC3339 only**: precise, but much worse CLI ergonomics.

### Wait for finalization by default

After successful submission, the command SHALL wait for finalization unless `--no-wait` is supplied. This mirrors the operator expectation that governance updates are important enough to track to completion by default.

Alternatives considered:
- **Always return immediately**: simpler, but makes the common operator path less useful.

## Risks / Trade-offs

- **Blind-signing unknown payloads reduces wallet-side safety** → Mitigation: require explicit blind-sign warnings and keep signer choice explicit.
- **Chain-state dependence can fail due to node issues** → Mitigation: surface actionable connectivity/query errors and allow explicit sequence-number override where appropriate.
- **Signer-selection UX can become complex** → Mitigation: reuse existing governance-key display conventions and interactive remove/list patterns rather than inventing a new signer UI.
- **Authorization validation is asymmetric between assisted and manual blind signing** → Mitigation: document clearly that threshold validation only happens when enough context exists.
- **Time parsing and derived defaults can be surprising across multiple accepted formats** → Mitigation: use deterministic parse order, document accepted formats clearly, show prompted defaults explicitly, and validate before submission.

## Migration Plan

1. Add the new `governance update` CLI surface and parsing helpers.
2. Reuse governance-key matching/rendering logic for signer prompts and key-index resolution.
3. Add node-query helpers for update-sequence lookup and submission/finalization tracking.
4. Add tests for decoded signing, blind signing, signer prompts, timing parsing, and wait / no-wait behavior.
5. Update README examples and command documentation.

Rollback is local to the wallet CLI code and documentation. No persistent schema migration is expected for this change.

## Open Questions

- Whether blind manual signing without `--sign-as` should validate anything beyond “selected keys exist locally and can be mapped to current key indices”.
- Whether known decoded flows should also allow a fully explicit sequence-number override even when node lookup succeeds (current direction: yes).
- Whether a later follow-up should introduce typed `governance update <type>` helpers on top of this generic payload-based command.
