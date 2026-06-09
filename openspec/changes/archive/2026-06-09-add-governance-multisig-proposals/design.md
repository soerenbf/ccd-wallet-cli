## Context

The current governance update implementation in `crates/ccd-wallet/src/commands/governance.rs` prepares a governance update, signs it immediately, and submits it in one invocation. Local governance keys can already satisfy multi-signer thresholds in that single process because the wallet can access multiple imported keys at once. The newly added Ledger-backed path is intentionally narrower: it signs with one connected Governance Ledger app and rejects updates whose threshold is greater than one.

This change introduces a detached workflow for operators who need multiple signatures from different machines, different custody models, or a mix of local governance keys and Governance Ledger devices. The detached workflow must preserve the existing safety model of the all-in-one command: each step resolves the selected network from config, talks to a node, and validates the current authorization context before proceeding.

The user has already constrained the exchange formats:

- proposal files contain only a version number, the target network genesis hash, the update header, and the governance update payload JSON
- signature files contain only a version number, the signing verify key, and a JSON representation matching the logical `UpdateInstructionSignature` shape

That means detached files are intentionally coordination artifacts rather than authoritative snapshots of chain authorization state.

## Goals / Non-Goals

**Goals:**
- Add a detached governance proposal workflow that supports create, sign, and submit stages.
- Support detached signing with both local governance keys and the Concordium Governance Ledger app.
- Keep all detached stages online and revalidating against the selected node.
- Freeze the exact signing material in proposal files by storing the full update header plus the update payload JSON.
- Keep detached file formats minimal and stable enough for exchange between operators.
- Preserve the existing `governance update` all-in-one flow for users who do not need detached signing.

**Non-Goals:**
- Supporting offline signing from detached files without node access.
- Allowing blind signing of unknown serialized governance payloads through the detached Ledger flow.
- Persisting authorization snapshots, threshold data, or signer-index mappings inside detached files.
- Changing on-chain governance authorization semantics or Ledger derivation semantics.
- Replacing the existing all-in-one `governance update` command.

## Decisions

### Introduce a dedicated `governance proposal` command family
The detached workflow will use a separate noun-oriented command family:

- `ccd-wallet governance proposal create`
- `ccd-wallet governance proposal sign`
- `ccd-wallet governance proposal submit`

This keeps `governance update` focused on the convenience path that signs and submits in one invocation, while `governance proposal` becomes the explicit coordination surface for multi-party signing.

**Alternatives considered:**
- Extend `governance update` with `create-proposal`, `sign-proposal`, and `submit-proposal` subcommands. Rejected because it overloads `update` with both an action and a resource model.
- Replace `governance update` entirely. Rejected because the current flow remains useful for threshold-1 and single-operator cases.

### Proposal files freeze the signing bytes through header + payload JSON
Proposal files will store:

- `version`
- `genesisHash`
- `header`
- `payload`

`payload` is stored as decoded governance update JSON, not serialized hex. On read, the CLI parses the JSON into `UpdatePayload`, encodes it with `EncodedUpdatePayload::encode`, and checks that the resulting payload size matches `header.payloadSize`. The signing hash is then recomputed from the stored header and encoded payload bytes.

This keeps the file readable while still freezing the exact signing material that detached signers must use.

**Alternatives considered:**
- Store only payload JSON and re-resolve timing or sequence number later. Rejected because detached signers would not be signing a fixed proposal.
- Store serialized hex instead of JSON. Rejected because the user explicitly prefers payload JSON only and decoded JSON is easier to review.
- Store additional authorization snapshots in the proposal. Rejected because the user wants each stage to revalidate online and derive its own live chain context.

### Signature files remain minimal but carry the resolved signer index
Signature files will store:

- `version`
- `verifyKey`
- `signature`

The `signature` field will use a wallet JSON wrapper matching the logical `UpdateInstructionSignature` structure:

```json
{
  "signatures": {
    "<index>": "<hex-signature>"
  }
}
```

`sign` resolves the current governance authorization context from the node, maps the selected signer verify key to its current `UpdateKeysIndex`, signs the proposal hash, and writes a single-entry signature map. `submit` re-derives the expected index from `verifyKey` and rejects mismatches rather than trusting the stored map blindly.

**Alternatives considered:**
- Store only a raw signature without an index. Rejected because the operator specifically wants the detached signature structure to match the expected key index.
- Store sign-hash, threshold, or authorization-family metadata in the signature file. Rejected because proposal files already define the signing material, and submission will revalidate live chain state.

### Detached stages revalidate against the node instead of trusting file metadata
Each detached command will resolve the selected network and query the node:

- `create` resolves the network and next sequence number, then writes a proposal with a frozen header.
- `sign` resolves the network, confirms the proposal genesis hash matches, re-derives the update authorization family from the proposal payload, resolves the currently authorized governance keys, and signs only if the selected key is currently authorized.
- `submit` resolves the network, confirms the proposal genesis hash matches, re-derives the update authorization family, resolves the currently authorized keys and threshold, verifies that the proposal header is still valid for submission, validates each detached signature against both the proposal hash and the current verify-key-to-index mapping, and submits only if the collected signatures satisfy the threshold.

`submit` will accept detached signatures through repeated `--signature <FILE>` flags and through a directory input that is scanned for signature files. This keeps the explicit single-file path for scripts while supporting operator workflows that collect many detached signatures in one directory.

This honors the desired online model and avoids treating exchanged files as authoritative snapshots.

**Alternatives considered:**
- Trust file-contained metadata and skip live revalidation. Rejected because the user explicitly wants all steps online and revalidating.
- Revalidate only at submission time. Rejected because stale or unauthorized proposals should be rejected earlier during detached signing as well.

### Stale proposals are rejected by both `sign` and `submit`
Because detached proposals store a fixed header, they are tied to a specific update sequence number and timing choice. If the live next sequence number no longer matches the proposal header sequence number for the relevant queue, `sign` and `submit` will both reject the proposal as stale.

This avoids collecting signatures for a proposal that can no longer be submitted unchanged.

**Alternatives considered:**
- Allow signing stale proposals and defer rejection to `submit`. Rejected because it wastes operator time and encourages signatures over already-invalid coordination artifacts.
- Mutate the proposal automatically during signing or submission. Rejected because changing the header would change the signing hash and invalidate collected detached signatures.

### Detached signing reuses existing signer-selection patterns
Detached local signing will reuse the same governance key selection behavior as the all-in-one flow: explicit key selection stays available, and interactive runs may use the same fuzzy governance-key selection flow. Detached Ledger signing remains opt-in through `--ledger`, which switches the command to a connected Governance Ledger device.

This keeps detached signing familiar for operators and avoids introducing a second local signer-picking UX only for proposal signing.

**Alternatives considered:**
- Require detached local signing to always specify `--key`. Rejected because it would make detached signing less ergonomic than the existing update flow.
- Auto-detect whether to use Ledger without `--ledger`. Rejected because signer backend choice should remain explicit.

### Detached proposal creation uses explicit timing inputs
For detached proposal creation, omitted effective time and timeout values will not silently fall back to prompt defaults. Operators must provide both values explicitly, either by flags or by answering prompts without prefilled defaults.

This makes proposal timing an intentional part of the coordination artifact and avoids collecting detached signatures around accidentally defaulted timing values.

**Alternatives considered:**
- Reuse the all-in-one update defaults for detached proposals. Rejected because detached multi-party signing has a longer coordination loop and is more sensitive to accidental timing choices.

### Detached file output uses canonical pretty JSON
Proposal and detached signature files will be written as canonical pretty JSON derived from the parsed in-memory data model rather than preserving the operator's original input formatting.

This provides stable exchanged artifacts, reduces formatting-only diffs between operators, and keeps file output deterministic for tests and documentation.

**Alternatives considered:**
- Preserve the original input formatting for payload JSON. Rejected because it complicates the implementation without improving signing semantics.

### Existing signer-preparation helpers should be refactored around proposal-shaped data
The current code already has strong internal boundaries:

- `resolve_update_payload`
- `resolve_update_chain_context`
- `prepare_governance_update`
- `sign_prepared_update_with_local_keys`
- `sign_prepared_update_with_ledger`
- `assemble_signed_update_instruction`

The detached flow should reuse these boundaries by introducing proposal/signature JSON wrapper types and conversion helpers around `PreparedGovernanceUpdate` and `GovernanceSignerOutput`. This keeps signing semantics shared across all-in-one and detached paths while isolating file I/O and revalidation logic.

**Alternatives considered:**
- Implement detached signing as a separate parallel codepath. Rejected because it would duplicate payload preparation, Ledger signing, and signature assembly logic.

## Risks / Trade-offs

- **[Live authorization can change between create and sign]** → Reject stale or unauthorized proposals during signing and require operators to recreate the proposal.
- **[Live authorization can change between sign and submit]** → Revalidate every signature at submission time and reject detached signatures whose verify key or index mapping no longer matches the chain.
- **[Proposal JSON re-encoding could diverge from the operator's expectations]** → Treat the encoded SDK payload bytes derived from parsed JSON as the canonical signing material and validate `payloadSize` against the stored header.
- **[Minimal file formats provide limited debugging context]** → Make CLI validation errors explicit about the failing field or revalidation step so operators can diagnose stale sequence numbers, wrong networks, or unauthorized signers.
- **[New command family increases command-surface area]** → Keep all-in-one `governance update` unchanged for simple cases and document the detached flow clearly in `docs/commands.md`.

## Migration Plan

1. Add the new `governance proposal` command family and keep `governance update` behavior unchanged.
2. Update command taxonomy documentation in `docs/commands.md` alongside clap command definitions.
3. Add detached proposal/signature parsing, validation, and signing/submission tests before removing the current threshold>1 Ledger limitation from the documented guidance.
4. Rollback remains straightforward: if detached proposal commands are reverted, the existing all-in-one `governance update` flow still covers current behavior.

## Open Questions

- None currently.
