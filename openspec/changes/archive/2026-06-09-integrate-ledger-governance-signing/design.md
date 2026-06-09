## Context

The governance update command currently resolves a network, parses a governance update payload, queries chain parameters and sequence-number context, unlocks the local governance key vault, selects decrypted `UpdateKeyPair` values, signs the update hash locally, submits the resulting update instruction, and optionally waits for finalization.

The completed `ccd-wallet-ledger-governance` crate provides a low-level client for the Concordium Governance Ledger app. It exports typed public-key and update-signing methods and returns raw public keys and signatures, while intentionally avoiding CLI orchestration, signer selection, signed update assembly, and node submission.

The desired user-facing step is small: add a `--ledger` mode for `governance update`. The architectural direction is larger: governance updates eventually need detached multi-signature flows where several Ledger devices on different machines sign the same prepared update and a coordinator combines signatures. This change should therefore avoid burying Ledger signing directly inside local-vault-oriented code paths.

## Goals / Non-Goals

**Goals:**
- Add an exclusive Ledger-backed signing mode for `ccd-wallet governance update`.
- Support typed governance update payloads signed by a connected Ledger running the Concordium Governance app.
- Preserve existing local governance key vault signing behavior when `--ledger` is not used.
- Reject blind/unknown serialized payloads for Ledger signing because the Governance Ledger app exposes typed update flows rather than generic blind signing.
- Keep signer provenance simple: one update invocation uses either local governance keys or Ledger governance keys, not both.
- Introduce internal prepared-update and signature-output concepts that can later be surfaced as detached prepare/sign/submit commands.
- Reuse the existing submission/finalization behavior after Ledger signatures are assembled into the signed update instruction.

**Non-Goals:**
- Supporting mixed local-vault and Ledger signatures in the same governance update.
- Exposing detached multi-machine prepare/sign/submit commands in this change.
- Persisting Ledger governance keys or Ledger-derived governance signing metadata in the governance key vault.
- Changing governance key vault schema, encryption, import, list, or remove behavior.
- Adding blind-sign support for Ledger governance signing.
- Extending or changing the low-level `ccd-wallet-ledger-governance` protocol crate beyond integration needs discovered during wiring.

## Decisions

### 1. Treat `--ledger` as an exclusive signing backend selector

**Decision:** `governance update --ledger` selects the Ledger signing backend for the entire invocation. It SHALL conflict with local-vault signer selection where necessary, including `--key` if that flag continues to mean local stored governance verify keys. The first implementation SHALL support exactly one Ledger signer per invocation.

**Rationale:** Mixed signer flows are not a target use case, and exclusivity keeps user intent and error behavior clear. Limiting the all-in-one CLI to one Ledger signer keeps the first integration small and avoids inventing a partial multi-Ledger UX before detached prepare/sign/submit flows exist.

**Alternatives considered:**
- Allowing local and Ledger signers together. Rejected because it complicates signer selection and threshold diagnostics without serving the desired operating model.
- Reusing `--key` for both local and Ledger public-key selection. Rejected initially because it would make signer source ambiguous unless accompanied by a larger key-source model.

### 2. Introduce an internal prepared update boundary

**Decision:** Refactor governance update assembly so both local and Ledger signing consume an internal prepared update value that contains the resolved update payload, serialized update header data, sequence number, timing, chain authorization context, and any metadata needed to assemble the final update instruction.

**Rationale:** Detached multi-machine signing requires a stable object that can be handed to signers independently of submission. Introducing the boundary now lets `--ledger` use the same shape without exposing new commands yet.

**Alternatives considered:**
- Wiring Ledger calls directly into `build_signed_update_instruction`. Rejected because it would couple device signing to submission-oriented assembly and make future detached signing harder.
- Designing a persisted file format now. Rejected as premature; the first step only needs an internal representation, though it should avoid assumptions that would prevent serialization later.

### 3. Represent signer output independently from signer backend

**Decision:** Add an internal signature-output representation containing the governance key index and raw signature bytes needed to assemble the update signature map. Local signing can continue to derive this from `UpdateKeyPair` signing; Ledger signing derives it from Governance Ledger app responses.

**Rationale:** Final update assembly needs indexed signatures, not the original signing mechanism. Separating signer output from signer backend keeps submission shared and prepares for future detached signature files.

**Alternatives considered:**
- Returning a complete `UpdateInstruction` from each backend. Rejected because multi-signer and detached flows need signature aggregation before final assembly.
- Keeping local signing as `BTreeMap<UpdateKeysIndex, UpdateKeyPair>` only. Rejected because Ledger signatures cannot be represented as local keypairs.

### 4. Require typed Ledger signing inputs

**Decision:** Ledger signing SHALL require a decoded governance update payload. `--ledger --blind` and serialized payloads that cannot be decoded SHALL fail before any device signing attempt.

**Rationale:** The Governance Ledger app signing surface is update-family-specific and displays typed update data. It does not provide the same generic blind-sign behavior as the local software-key path.

**Alternatives considered:**
- Sending only the update sign hash to the Ledger. Rejected because the Governance Ledger app does not expose that protocol capability.
- Falling back to local vault signing when Ledger cannot sign. Rejected because `--ledger` should not silently change signer source.

### 5. Derive the Ledger governance path from the update authorization family

**Decision:** The CLI SHALL construct the Governance Ledger derivation path from the thing being signed rather than accepting an arbitrary full path. The path shape is `m/purpose'/coin_type'/1'/gov_purpose'/key_index'`, represented in the Governance Ledger crate as `[1, purpose, key_index]`, where `purpose` is selected from the update authorization family (`0` root, `1` level 1, `2` level 2). The first implementation SHALL default `key_index` to `0`, allow overriding it explicitly, and support only one selected Ledger signer.

**Rationale:** Governance signer purpose is implied by the update family and current chain authorization rules, so asking users for a raw path would expose avoidable detail and increase mistakes. A default key index of `0` matches the existing governance path convention while still leaving room for operators who need another key index.

**Alternatives considered:**
- Accepting an arbitrary raw derivation path. Rejected because it is easier to mistype and duplicates knowledge the CLI already has from the update family.
- Selecting Ledger signers only by public key. Rejected for the first integration because the CLI still needs a path to ask the device to sign.
- Persisting Ledger governance key registrations first. Rejected as broader than needed for the first signing integration.

### 6. Verify signatures against chain authorization before submission

**Decision:** Ledger public keys and/or returned signatures SHALL be mapped to current on-chain governance key indices using the same chain authorization context already resolved for local signing. The command SHALL fail if selected Ledger signer public keys are not authorized for the update family or if the resulting Ledger signer set does not meet the required threshold.

**Rationale:** The node expects signatures indexed by governance key index, and operators need actionable errors before submission. Chain-state-based mapping also matches the current local governance key behavior.

**Alternatives considered:**
- Trusting user-supplied key indices without checking public keys. Rejected because it can produce invalid submissions and weak diagnostics.
- Submitting and relying on node rejection. Rejected because device signing is interactive and should fail early when possible.

## Risks / Trade-offs

- **Prepared-update refactor could destabilize existing local governance signing** → Keep local-vault behavior covered by existing tests and make the prepared boundary preserve current payload, timing, sequence-number, and threshold semantics.
- **Ledger command coverage may not map cleanly for every governance update payload variant** → Use the low-level crate's typed request model and fail actionably for unsupported or ambiguous mappings rather than signing incorrectly.
- **Some governance updates require thresholds greater than one, but the first Ledger CLI supports only one signer** → Fail early with a clear threshold-versus-supported-signer-count error and leave multi-Ledger collection to the later detached signing flow.
- **Sequence numbers can become stale before future detached submission** → The future detached flow should make sequence-number pinning explicit; this change only creates the internal boundary and keeps all-in-one submission immediate.
- **Purpose/key-index-derived path selection can still target the wrong Ledger key index** → Fetch the Ledger public key before signing and verify it against chain authorization context before requesting update signatures where practical.
- **New dependency increases CLI build surface** → Depend on the existing low-level Governance Ledger crate narrowly from `ccd-wallet` and avoid leaking protocol details into unrelated modules.

## Migration Plan

1. Add CLI flags for exclusive Ledger governance signing, governance key-index override, and update command documentation.
2. Introduce internal prepared-update and signer-output helpers while preserving current local-vault behavior.
3. Add Ledger signer resolution from update-family-derived governance purpose plus key index, including public-key lookup and authorization/threshold checks.
4. Convert prepared typed governance updates into `ccd-wallet-ledger-governance` request types and collect a raw signature from the connected device.
5. Assemble the signed update instruction from signer outputs and reuse existing node submission/finalization code.
6. Add tests for clap parsing, unsupported combinations, local behavior preservation, Ledger signer mapping, and signed update assembly.

Rollback is straightforward: remove the Ledger CLI flags and Ledger backend wiring. The existing local governance key vault flow remains the default and should continue to work independently.

## Open Questions

- None currently. The change assumes Governance Ledger path construction from the update authorization family with a default key index of `0`, an explicit key-index override, and exactly one Ledger signer in the all-in-one CLI flow.
