## Context

The CLI already uses a transaction-style flow for several submission commands: resolve missing input, validate it, show a human-readable summary, and require explicit approval before sending anything to the node. Governance update submission currently resolves payload, timing, network, chain authorization context, and signers, then signs and submits immediately. Detached proposal signing similarly prepares and validates a proposal, then signs immediately.

Governance has three safety-sensitive entry points for this change: the all-in-one `governance update` path, the detached `governance proposal sign` path, and the detached `governance proposal submit` path. All build or load a prepared governance update, which provides a natural source for review details.

## Goals / Non-Goals

**Goals:**

- Add a transaction-style review and explicit approval gate to interactive governance signing and submission flows.
- Place approval before local or Ledger signing in `governance update` so declined updates do not produce signatures.
- Show enough resolved context for operators to validate what will be submitted: network, update payload identity, parsed payload details, sequence, timing, signer/signature context, and blind payload warnings.
- Keep `--non-interactive` prompt-free and preserve existing validation and submission behavior there.
- Keep detached proposal validation intact before proposal signing review prompts, and keep detached signature verification intact before proposal submission review prompts.

**Non-Goals:**

- Do not change governance payload formats, proposal files, signature files, or database state.
- Do not change threshold, authorization, Ledger, or local vault signing rules.
- Do not add a new force/yes flag unless a later requirement asks for non-interactive approval semantics beyond existing `--non-interactive` behavior.

## Decisions

### Prompt after input resolution and signer/signature validation, before node submission

`governance update` will resolve payload, timing, chain context, and signer selection first, then render the review and ask for approval before invoking local or Ledger signing. This includes the selected local verify keys or Ledger key index/path-derived signer context in the review without creating signatures for declined updates.

Alternative considered: prompt after assembling the signed block item. That would be closest to reviewing the final submitted bytes, but it would require signing before the user approves CLI submission and would duplicate Ledger device approval in an awkward order.

### Apply the same gate to detached proposal signing and submission

`governance proposal sign` will load the proposal, revalidate live chain context, resolve the selected local or Ledger signer context, render the prepared update including parsed payload details, and ask for approval before producing a detached signature file. This is especially important for Ledger detached signing because the operator should compare the CLI-parsed proposal details with the device display before approving the signature.

`governance proposal submit` will continue to load the proposal, revalidate live chain context, load detached signatures, verify signatures, and ensure threshold before rendering the review and asking for approval. This lets the review include the accepted signature indices/verify keys and avoids asking the user to approve a proposal that later fails local validation.

Alternative considered: only prompt before node submission. That would leave detached signatures as a blind spot, particularly for Ledger signing where producing a detached signature is the safety-sensitive action even before any node submission occurs.

### Use the existing cliclack yes/no confirmation prompt

The implementation should add small governance-specific render/approval helpers in `commands/governance.rs` and use the existing `cliclack::confirm` yes/no prompt for final approval. This preserves the current governance prompt style where users can move to the Yes option with keyboard arrows instead of typing `yes`.

Alternative considered: use the string-input `Type y to approve` pattern from several transaction commands. That would align with those commands textually, but it is worse for this governance flow because the existing yes/no confirm interaction is easier and already used in governance validation fallback prompts.

### Render parsed payload details when available

For decoded governance update payloads, the review should include structured payload details derived from the parsed `UpdatePayload`, not only the update type and byte size. This is especially important for Ledger signing because operators should be able to compare the CLI-parsed details with the details displayed on the Ledger device before approving.

The renderer can start with a stable pretty JSON representation of the parsed governance update payload, plus summary fields such as update type, payload size, sequence, and timing. Blind serialized payloads cannot be semantically rendered, so the review must clearly label them as blind and show raw-payload identifiers such as byte length.

Alternative considered: show only high-level payload identity in the first implementation. That is simpler but does not satisfy the operator validation goal for Ledger signing.

### Non-interactive mode remains prompt-free

When `--non-interactive` is supplied, governance signing and submission skip the approval prompt. Existing required argument validation remains responsible for making machine-oriented flows explicit.

Alternative considered: require an additional `--yes` flag in non-interactive mode. That would be safer for automation, but it changes established command semantics and is not needed for this change's stated goal of aligning interactive governance UX with transaction flows.

## Risks / Trade-offs

- Payload detail rendering may be verbose for large governance updates. → Prefer readable pretty JSON or sectioned output and keep high-level summary fields before the full details.
- Prompt placement before signing means the review does not include final signatures for `governance update`. → Include selected signer context and rely on existing signing validation after approval.
- Detached proposal sign prompts before a detached signature exists, so the review cannot include final signature output. → This is intentional: approval should happen before the safety-sensitive signing action.
- Detached proposal submit prompts after signature validation, so invalid signatures fail before approval. → This is intentional: approval should be for a submit-ready update.
- Adding a prompt may affect tests or scripted interactive usage. → Gate it behind `!non_interactive` and use `cliclack::confirm` with an initial No value.
