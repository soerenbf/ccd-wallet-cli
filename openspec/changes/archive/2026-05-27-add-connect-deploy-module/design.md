## Context

The current browser connect flow already spans three layers:

- `crates/ccd-wallet-connect` for the WebSocket JSON-RPC protocol and session handling
- `crates/ccd-wallet/src/commands/connect.rs` (to be refactored into `commands/connect/`) for interactive wallet approval, signing, and submission
- `packages/ccd-wallet-connect-client` plus `packages/ccd-wallet-connect-example` for browser-side integration

Smart contract init and update already work end-to-end, and the example app presents them together in the Smart Contracts section. Module deployment is the missing adjacent workflow in that same user journey.

At the same time, the current Rust `commands::connect` implementation is becoming large enough that adding another request type directly to the same file would make the code harder to navigate, review, and evolve. The TypeScript client already trends toward feature-level files under `src/features/`, and the Rust side now needs the same capability-oriented split.

This change therefore combines a new end-to-end capability (`requestDeployModule`) with a small architectural tightening: connect-related code in both TypeScript and Rust should mirror capability boundaries where practical.

## Goals / Non-Goals

**Goals:**
- Add a `requestDeployModule` connect protocol method for wallet-approved smart contract module deployment.
- Support deploy requests from the example app Smart Contracts section alongside Contract Init and Contract Update.
- Make the example app use file upload as the deploy input surface while sending raw module bytes as hex over the protocol.
- Support optional deploy validation that checks whether the module already exists on chain before prompting for approval.
- Refactor the Rust connect command into feature-oriented modules.
- Keep the TypeScript connect client organized by matching feature-oriented capability modules.
- Record the mirrored module-boundary decision in project guidance.

**Non-Goals:**
- Redesign the overall paired-session shell or move deploy outside the Smart Contracts section.
- Replace file-upload input with raw hex paste in the example app.
- Remove the existing node-endpoint application configuration from the example app.
- Introduce a generic trait-heavy connect feature framework.
- Add browser-side finalization polling or richer deploy status APIs.

## Decisions

### 1. Add `requestDeployModule` as a first-class connect capability

**Decision:** Add a new JSON-RPC method named `requestDeployModule` across the connect server and TypeScript client.

The method will accept:
- `sessionToken`
- `moduleHex`
- `validate` (optional, default `false`)

On success it returns:
- `transactionHash`

**Rationale:** The method name matches the existing connect naming convention (`requestAccount`, `requestContractInit`, `requestContractUpdate`) while staying aligned with the Concordium transaction type name `DeployModule`.

**Alternative considered:** `requestModuleDeploy`. Rejected because it breaks the emerging naming pattern and is less directly aligned with the transaction type.

### 2. Use file upload in the example app but hex on the wire

**Decision:** The example app will accept only uploaded module files in the Deploy Module flow. It will convert the file bytes to hex in-browser and send the resulting `moduleHex` through `@ccd-wallet/connect-client`.

**Rationale:** File upload is the natural operator input for module deployment and makes the example app a much better integration reference than forcing users to paste large hex blobs. The protocol remains hex-based for consistency with existing connect request fields such as `parameterHex`.

**Alternative considered:** Expose a raw hex textarea in the example app. Rejected because it would be awkward for normal usage and would make the showcase feel protocol-centric rather than workflow-centric.

### 3. Keep deploy adjacent to init/update in the Smart Contracts section

**Decision:** The example app will keep a single Smart Contracts section with three adjacent flows:
- Deploy Module
- Contract Init
- Contract Update

**Rationale:** These are sibling smart-contract lifecycle actions. Grouping them in the same section preserves the showcase mental model and avoids scattering related capabilities across the app.

**Alternative considered:** Move deploy under Transactions. Rejected because it weakens the Smart Contracts showcase and separates closely related workflows.

### 4. Support deploy-specific validation via on-chain module existence checks

**Decision:** `requestDeployModule` supports `validate: true`. When enabled, the wallet derives the module reference from the submitted module bytes and checks whether that module already exists on chain before prompting for approval.

If the validation check fails due to node issues, the wallet will surface that result as a warning in the approval prompt and still let the user decide whether to proceed. If the module is confirmed to already exist on chain, the wallet will also surface that result as a warning in the approval prompt and still let the user decide whether to proceed.

**Rationale:** For deploy flows, the meaningful preflight is not contract execution simulation but duplicate-module detection. This preserves the user intent behind `validate` — "check something important before I approve" — while fitting deploy semantics. Keeping duplicate detection non-blocking also matches the broader connect philosophy that validation informs operator judgment rather than replacing it.

**Alternatives considered:**
- Omit `validate` for deploy. Rejected because duplicate detection is a useful and coherent preflight.
- Treat "module already exists" as a blocking error. Rejected because the operator may still want to inspect and intentionally submit despite the warning.

### 5. Mirror connect capability boundaries across TypeScript and Rust where practical

**Decision:** Organize connect-related code around feature modules in both ecosystems.

TypeScript should continue using:
- `core/` for transport/protocol primitives
- `features/` with one file per connect capability

Rust should move `commands::connect` to a directory with:
- `mod.rs` for orchestration and server wiring
- one module per connect capability
- a small `shared.rs` for reused helpers

Expected Rust shape:
- `pairing.rs`
- `account.rs`
- `contract_init.rs`
- `contract_update.rs`
- `deploy_module.rs`
- `shared.rs`

Expected TypeScript shape:
- `features/pairing.ts`
- `features/account.ts`
- `features/contract-init.ts`
- `features/contract-update.ts`
- `features/deploy-module.ts`

**Rationale:** Mirrored capability boundaries make request flows easier to trace end-to-end, reduce cognitive load when adding new features, and prevent large monolithic connect modules.

**Alternative considered:** Keep Rust in one large `connect.rs` file while TypeScript stays split by feature. Rejected because the two layers would drift structurally and the Rust command would become progressively harder to maintain.

### 6. Keep deploy flow lightweight in the example app

**Decision:** The Deploy Module flow will not introduce an artificial explicit "prepare" step analogous to schema derivation for init/update. File selection and validation state will be reflected through passive status text, and submit will perform the actual request.

**Rationale:** Deploy does not require schema lookup or parameter preparation. A fake preparation button would imply symmetry that does not exist and would add unnecessary UX friction.

### 7. Show duplicate-module findings as actionable wallet warnings

**Decision:** When deploy validation confirms that the derived module reference already exists on chain, the wallet will show an actionable warning in the approval prompt instead of rejecting the request before submission.

A suitable warning is:

`Validation warning: module already exists on chain for this network; submitting again is expected to reuse the same module reference.`

The wallet may include the derived module reference in the prompt or adjacent logs for operator clarity.

**Rationale:** This keeps deploy validation aligned with the connect UX used elsewhere: validation is advisory and helps the operator decide, but it does not replace operator intent. Duplicate detection is still highly useful, but it should not force the browser into an immediate RPC error if the user wants to inspect and proceed anyway. Because redeploying an already-known module is accepted by the chain, the warning should describe the behavior accurately instead of implying likely failure.

**Alternative considered:** Reject duplicate findings with a dedicated JSON-RPC error code. Rejected because it blocks an intentional user override and makes deploy validation stricter than the broader connect validation model.

### 8. Keep finalization reporting feature-specific and render readable summaries

**Decision:** Finalization printing for connect transaction features will remain feature-specific rather than being forced into a shared helper at this stage. Each feature should print a concise, legible summary instead of a debug dump, and may assert feature-specific events when those events should exist for successful outcomes.

For deploy-module finalization specifically, the wallet should print a readable summary such as the finalized block, the deployed module reference, and whether the deploy-module transaction succeeded or was rejected.

**Rationale:** Init, update, and deploy have different success indicators and different useful operator-facing details. Keeping them separate preserves clarity and avoids premature abstraction. Replacing debug-style output with readable summaries makes the wallet terminal a better review surface without changing the JSON-RPC contract.

**Alternative considered:** Introduce a generic shared finalization printer now. Rejected because the transaction-specific event expectations are different enough that a shared abstraction would either leak complexity or flatten useful detail.

## Risks / Trade-offs

- **Large module payloads over WebSocket** → Module bytes expand when converted to hex and then encoded in JSON. Mitigation: keep the protocol simple for now, test realistic module sizes, and defer chunking/compression unless real limits appear.
- **Refactor and feature addition in one change** → Combining architecture cleanup with new behavior increases review surface. Mitigation: keep the refactor feature-oriented and shallow, with minimal abstraction beyond module boundaries.
- **Validation requires extra node interaction** → Deploy validation adds latency and a new failure mode. Mitigation: make validation optional and treat node connectivity failures as warnings rather than automatic blockers.
- **Mirrored structure is a guideline, not a perfect isomorphism** → TypeScript and Rust will still differ because one side is a browser client and the other is an interactive wallet command. Mitigation: mirror capability boundaries, not every helper or implementation detail.
- **File upload only limits protocol debugging from the example UI** → Operators cannot paste hex directly in the demo. Mitigation: keep the protocol typed and hex-based in the client so other test harnesses can still call it directly.

## Migration Plan

- Add the new `requestDeployModule` protocol support and TypeScript client API.
- Refactor `crates/ccd-wallet/src/commands/connect.rs` into feature modules while preserving existing pairing, account, init, and update behavior.
- Add the Deploy Module example app flow on top of the refactored client/server support.
- Update project guidance and package documentation to describe the new capability and mirrored module-boundary convention.

Rollback is straightforward: if deploy support proves problematic, remove the `requestDeployModule` method and example-app deploy mode while keeping the non-breaking connect refactor if desired.

## Open Questions

- None at this time.
