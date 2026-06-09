## Context

The CLI already has two adjacent but asymmetric key-source experiences:

- `seed add --restore <NETWORK>` and `seed sync` can recover identities and accounts for seed-backed key sources.
- `ledger setup` can enroll a Ledger-backed key source, and Ledger-backed identity issuance can export purpose-based issuance material from a matching device, but there is no Ledger recovery command surface.

Today the recovery implementation in `crates/ccd-wallet/src/commands/seed.rs` is strongly shaped around `ConcordiumHdWallet`, which assumes a locally stored seed phrase. Ledger recovery needs the same downstream network interactions—wallet-proxy provider discovery, identity recovery requests, node-based account discovery, local import, progress reporting—but it cannot derive recovery material from a local mnemonic. Instead, it must obtain recovery-critical material from a connected Ledger device after verifying the enrolled owner and completing an explicit export-approval flow.

This change is cross-cutting because it touches clap command definitions, Ledger command orchestration, the existing recovery engine, security-sensitive Ledger export behavior, and command taxonomy documentation.

## Goals / Non-Goals

**Goals:**
- Add `ledger sync [LABEL]` with the same user-facing recovery intent as `seed sync`.
- Extend `ledger setup [LABEL]` with `--restore <NETWORK>` so enrollment can immediately flow into recovery.
- Reuse as much of the existing recovery pipeline as practical, especially provider selection, network resolution, node queries, import behavior, and progress reporting.
- Preserve Ledger security expectations by requiring explicit approval semantics for secret export and by keeping exported material transient.
- Keep recovered identities and accounts owned by the enrolled Ledger signer owner rather than by a synthetic seed-backed representation.

**Non-Goals:**
- Changing seed recovery semantics or removing the existing `seed` command behavior.
- Supporting Ledger recovery without a connected matching Ledger device.
- Persisting exported Ledger recovery secrets for future offline use.
- Introducing new database tables or changing the signer-owner storage model.
- Expanding Ledger account creation beyond the already-supported or separately specified flows.

## Decisions

### 1. Split recovery orchestration from seed-specific derivation

**Decision:** Refactor the current recovery implementation into a shared recovery pipeline that consumes derived recovery material through a small abstraction, rather than hard-coding `ConcordiumHdWallet` throughout the full workflow.

**Rationale:** The network-facing parts of recovery are already generic: provider selection, identity recovery endpoint calls, account lookups, import, and summary reporting do not fundamentally care whether material comes from a seed or a Ledger. The source-specific part is the derivation of `IDCredSec` and `PRFKey` for a given `(provider, identity)` tuple. Pulling that boundary outward lets seed and Ledger recovery share the same behavioral core without forcing Ledger flows to masquerade as locally stored seeds.

**Alternatives considered:**
- Duplicate the entire seed recovery flow into `commands/ledger.rs`. Rejected because it would fork complex progress, import, and provider-selection behavior.
- Keep the existing `ConcordiumHdWallet` type everywhere and synthesize a fake seed-backed wallet from Ledger exports. Rejected because Ledger does not expose a recoverable mnemonic and because the model would misrepresent the source of truth.

### 2. Model Ledger recovery around per-identity exported material

**Decision:** Ledger recovery will obtain the minimum purpose-specific recovery material needed from the connected Ledger device after owner verification. Identity probing will use the Ledger app's `IdRecovery` export purpose to obtain `IDCredSec` for the current provider/identity tuple. Account probing will only happen for recovered identities, and will use the `AccountCredentialDiscovery` export purpose to obtain the PRF key for that recovered identity.

**Rationale:** Existing recovery behavior only needs identity-recovery and credential-discovery primitives. It does not require a full mnemonic-backed wallet object if those primitives can be provided directly. Using purpose-specific exports avoids prompting for broader identity-issuance material and prevents account-discovery prompts for identities that were not recovered.

**Alternatives considered:**
- Export a wider bundle and treat Ledger recovery as host-side wallet reconstruction. Rejected because it increases security exposure and is unnecessary for the required workflow.
- Add blind host-side derivation rules for Ledger identities without export. Rejected because the current device model does not expose enough host-verifiable material to match seed recovery behavior safely.

### 3. Keep Ledger recovery under explicit export approval semantics

**Decision:** `ledger sync` and `ledger setup --restore` will use the same explicit secret-export security posture as existing Ledger identity issuance. Interactive runs will ask for one up-front approval covering the recovery command session before any export-backed probing begins. Non-interactive runs must require an explicit allow flag before any export-backed recovery probing begins.

**Rationale:** Recovery requires sensitive material that must leave the device transiently. A single up-front CLI approval keeps operator intent explicit without repeatedly interrupting the session with host-side confirmations, while preserving a strict opt-in requirement for automation.

**Alternatives considered:**
- Treat recovery as a routine read-only operation with no extra approval. Rejected because it exports recovery-critical secrets into host memory.
- Prompt for separate host-side approval before each export-backed probe. Rejected because it would make recovery noisy without improving the trust boundary beyond the explicit session approval plus any device-native confirmations.
- Require a new bespoke approval mechanism unrelated to existing Ledger flows. Rejected because it would create inconsistent security semantics for similar export-backed operations.

### 4. Verify the enrolled owner once per command, then gate all recovery work on that verified session

**Decision:** Ledger recovery commands will open the device, verify the connected Ledger against stored canonical public key enrollment data, unlock the local Ledger owner vault, and only then begin recovery.

**Rationale:** Owner verification and vault unlock are prerequisites for both safe export and correct ownership of imported entities. Doing this up front produces clearer failure behavior and prevents partial recovery work against the wrong device.

**Alternatives considered:**
- Delay verification until the first provider probe. Rejected because it makes failures later and less understandable.
- Re-verify before every provider or identity probe. Rejected because it adds overhead without improving the trust model for a single command session.

### 5. Preserve the seed recovery UX shape, but make Ledger probing fully sequential

**Decision:** The command surface and most UX semantics should match seed recovery—same network resolution, provider filters, `--no-defaults`, truthful summaries, and import rules—but Ledger-backed recovery probing itself will run sequentially and prioritize locally known identities. For each selected provider, discover accounts for locally known recovered identities first, then probe the next unused identity index one at a time and stop at the first missing identity unless existing local state already proves later identities should be considered. For each newly recovered identity, export account-discovery material immediately and probe account indexes until the account inactivity bound is hit.

**Rationale:** A physical Ledger is a serialized interaction point, and the recovery path depends on repeated device-backed exports for each identity tuple. A fully sequential probing model is simpler to reason about, avoids interleaving device interactions, and prevents the unusable flow where a Ledger user must approve a long run of missing identity probes.

**Alternatives considered:**
- Preserve some provider/account concurrency after serializing only export-backed steps. Rejected because it still complicates control flow and progress semantics for limited practical gain in a device-bound workflow.
- Preserve the seed recovery concurrency model unchanged. Rejected because concurrent export-backed probing is likely to produce poor device UX and brittle command behavior.

### 6. Do not implicitly default Ledger sync to an active key source

**Decision:** `ledger sync` will not silently derive its target from an active key source. When the label is omitted in interactive mode, the CLI may show a selector and preselect the active key source only if that active key source is Ledger-backed. In non-interactive mode, omission remains an error when no explicit defaulting path is allowed.

**Rationale:** Recovery against the wrong hardware-backed key source is higher cost and harder to undo than choosing the wrong soft default in a purely local flow. Preselecting an active Ledger key source in the UI keeps the command ergonomic without hiding target selection.

**Alternatives considered:**
- Automatically use the active key source when one exists. Rejected because it makes a sensitive recovery command too implicit.
- Never consult active key-source state at all. Rejected because preselection in an interactive selector is a safe convenience when the active key source is already Ledger-backed.

## Risks / Trade-offs

- **Shared recovery refactor could destabilize existing seed recovery** → Mitigation: keep the behavioral surface unchanged for `seed sync`, and cover the shared pipeline with seed and Ledger-focused tests.
- **Ledger export volume may make recovery slow or approval-heavy** → Mitigation: export only the minimum material needed, verify device ownership once, and keep the UX explicit about ongoing device-backed probing.
- **Non-interactive Ledger recovery could become dangerous if approval is implicit** → Mitigation: require an explicit allow flag before any export-backed probing in non-interactive mode.
- **Ledger app version differences could cause partial or confusing failures** → Mitigation: reuse the existing version-gated export error mapping and fail early with actionable messages when purpose-based export support is missing.
- **Refactoring around a new abstraction may introduce too much indirection** → Mitigation: keep the abstraction narrowly focused on recovery-material derivation rather than designing a large generic signer framework.

## Migration Plan

1. Add the new Ledger CLI arguments and document the intended command surface.
2. Extract or introduce shared recovery helpers so seed and Ledger commands can share the same network-facing recovery engine.
3. Implement Ledger-specific recovery-material acquisition, owner verification, and approval handling.
4. Wire `ledger sync` and `ledger setup --restore` into the shared recovery flow.
5. Update `docs/commands.md` and extend tests for CLI parsing, error handling, and recovery orchestration.

Rollback is straightforward: remove the new Ledger command paths and shared recovery adapter layer, leaving the existing seed recovery flow intact.

## Open Questions

No open questions currently. The design assumes one up-front interactive approval per recovery session, fully sequential Ledger probing, account discovery for locally known recovered identities before new identity recovery, first-missing identity termination, account discovery only for recovered identities, and label-explicit target selection with at most interactive preselection of an active Ledger key source.
