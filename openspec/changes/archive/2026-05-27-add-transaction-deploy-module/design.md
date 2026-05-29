## Context

The wallet already supports deploy-module transactions through the browser connect flow in `crates/ccd-wallet/src/commands/connect/deploy_module.rs`, including module parsing, derived module reference display, duplicate-module validation, signing, submission, and finalization reporting. The top-level `transaction` command currently supports hash inspection through `transaction show`, while `transaction show` already contains a human-oriented transaction summary renderer for committed and finalized outcomes.

This change introduces a dedicated `contract` command space for first-class smart contract CLI workflows and uses deploy-module as the first command in that space. It also separates deploy-module domain logic from the connect transport wrapper so CLI and connect share deploy semantics without forcing both entrypoints into the same interaction model.

## Goals / Non-Goals

**Goals:**
- Add `ccd-wallet contract deploy-module <file-path>` as a first-class CLI workflow for smart contract module deployment.
- Reuse deploy-module preparation, duplicate validation, submission, and finalization logic between the new CLI command and the existing connect deploy flow.
- Reuse the transaction summary rendering path for CLI finalization output so deploy results and `transaction show` stay visually aligned.
- Keep CLI behavior native to the wallet command line by waiting for finalization inline after submission by default while supporting `--no-wait` for submit-and-return workflows.

**Non-Goals:**
- Redesign the connect deploy-module JSON-RPC contract or make the connect response wait for finalization.
- Introduce a generic transaction capability framework shared by all command families.
- Generalize contract init and contract update in the same change beyond shaping the shared deploy code so future reuse remains possible.
- Add browser-facing features or TypeScript client changes.

## Decisions

### 1. Add a dedicated contract command space

**Decision:** Add a top-level `contract` command family and implement module deployment as `ccd-wallet contract deploy-module <file-path>`. Keep `transaction` focused on transaction inspection, currently `transaction show`.

The Rust command layout should be feature-oriented:
- `commands/contract/` for contract-facing CLI workflows such as deploy-module
- `commands/transaction/` for transaction inspection and reusable transaction summary rendering
- `smart_contracts/deploy_module.rs` for neutral deploy-module mechanics shared by contract CLI and connect

**Rationale:** Deploying a smart contract module is a transaction, but users think of it as a contract lifecycle operation. A dedicated `contract` namespace gives future contract actions a natural home and avoids crowding the generic transaction inspection space.

**Alternatives considered:**
- Keep deploy-module under `transaction`. Rejected because it mixes generic transaction inspection with smart contract lifecycle actions.
- Add a one-off top-level `deploy-module` command. Rejected because it would not scale to future contract init/update-style CLI commands.

### 2. Extract deploy-module mechanics into a neutral Rust module

**Decision:** Move deploy-module mechanics that are independent of the caller into a neutral shared module, proposed as `crates/ccd-wallet/src/smart_contracts/deploy_module.rs` (or equivalent non-connect path).

That shared module should own:
- parsing module bytes into `WasmModule`
- deriving the module reference and module size
- duplicate-module validation against the selected node
- transaction construction and submission
- waiting for finalization and returning structured outcome data

Caller-specific code should stay outside the shared module:
- connect request parsing and JSON-RPC rejection mapping
- CLI file-path reading and command-line argument handling
- entrypoint-specific review prompt wording
- whether finalization is awaited inline or in a spawned background task

**Rationale:** The connect deploy handler already contains the exact mechanics the CLI needs, but its current shape is transport-oriented. A neutral module allows the CLI and connect flows to share the deploy behavior without coupling the CLI to connect request types or prompt text.

**Alternatives considered:**
- Reuse `commands/connect/deploy_module.rs` directly from the CLI. Rejected because it would invert the dependency boundary and make the CLI depend on connect internals.
- Extract a highly generic trait-based transaction framework. Rejected because the current need is specific and the repository guidance prefers simple, concrete reuse over premature abstraction.

### 3. Keep network and signer resolution in the CLI adapter

**Decision:** The CLI deploy command resolves network context, node endpoint, and signer-capable account through the normal wallet CLI mechanisms, then passes the resolved deploy input into the shared deploy module.

The CLI layer is responsible for:
- reading `<file-path>` into bytes
- resolving the network from explicit flags or existing CLI defaults/prompt flow
- resolving the target account through existing account-selection and signing-source logic
- collecting approval interactively before submission

**Rationale:** File paths, prompt fallback, and account/network selection are command concerns rather than deploy-module domain concerns. Keeping them in the CLI adapter preserves the existing command architecture and allows connect to continue using its session-bound account/network model.

**Alternatives considered:**
- Let the shared deploy module read files or select accounts directly. Rejected because those concerns do not exist for connect and would make the shared layer CLI-specific.

### 4. Reuse the transaction summary renderer for post-submission finalization output

**Decision:** Extract the human-oriented transaction summary rendering logic from `transaction show` into a reusable helper and use that helper when the CLI deploy command finishes waiting for finalization.

The deploy command should still print deploy-specific context before submission, but once the transaction finalizes it should render the finalized outcome through the same summary path used by `transaction show`.

**Rationale:** This gives operators one consistent presentation for finalized transaction outcomes and reduces the risk that deploy reporting and explicit transaction inspection drift in wording or detail.

**Alternatives considered:**
- Keep a deploy-specific finalization printer. Rejected for now because the renderer already exists and the user experience should converge unless that proves too verbose in practice.
- Replace `transaction show` with a deploy-specific summary style. Rejected because `transaction show` is the broader canonical inspection view.

### 5. Preserve different waiting models for CLI and connect

**Decision:** The CLI deploy command waits for finalization inline after submission by default, but it also supports `--no-wait` to stop after submission and print the transaction hash. The connect deploy flow continues to return the JSON-RPC response immediately and report finalization asynchronously.

Both flows may still use the same shared deploy submission and finalization helpers, but the caller decides whether waiting happens inline, is skipped after submission, or runs in the background.

**Rationale:** The entrypoints have different constraints, and the CLI has two valid operator modes. The default path should feel synchronous and complete, while `--no-wait` supports scriptable or fire-and-forget usage without forcing users to wait for finalization.

**Alternatives considered:**
- Make both flows wait inline. Rejected because it would violate the connect capability requirements.
- Make the CLI behave like connect and return after submission only. Rejected because it would be a worse default terminal UX for a direct command.
- Omit `--no-wait`. Rejected because the CLI already uses submit-without-wait patterns elsewhere, and deploy-module should support that operator preference too.

## Risks / Trade-offs

- **Renderer reuse may feel too verbose in an inline flow** → Start with the shared `transaction show` style and adjust later only if real usage shows it is too heavy.
- **Transaction command refactor increases review surface** → Keep the refactor shallow: one file per subcommand plus a small shared rendering helper.
- **Shared deploy logic can drift toward CLI-specific assumptions** → Keep file reading, prompting, and account/network selection out of the shared module boundary.
- **Validation adds another node round-trip** → Run validation by default because duplicate detection is useful, provide `--no-validate` for operators who need to skip the extra node call, and surface duplicate or node-failure findings as warnings rather than pre-submission blockers. Keep the duplicate warning concise: `Validation warning: module already exists on chain for this network.`

## Migration Plan

- Add the new CLI subcommand and neutral deploy-module shared module.
- Extract the transaction summary renderer from `transaction show` into a reusable helper without changing the visible `transaction show` behavior.
- Implement CLI waiting control so the default flow waits for finalization and `--no-wait` exits immediately after submission with the transaction hash.
- Adapt connect deploy-module code to reuse the shared deploy logic where practical while preserving its non-blocking JSON-RPC response semantics.
- Add or update tests for CLI deploy behavior, shared deploy preparation/validation, optional no-wait behavior, `--no-validate` behavior, and shared transaction summary rendering.

Rollback is straightforward: remove the new `contract deploy-module` subcommand and reconnect the deploy logic directly in the connect flow if the shared module shape proves problematic.

## Open Questions

- None at this time.