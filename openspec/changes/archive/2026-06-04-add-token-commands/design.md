## Context

The repository already has a documented planned `token` command space in `docs/commands.md`, but the clap surface and command modules do not yet implement it. The workspace is pinned to a Concordium Rust SDK git branch that exposes both protocol-level token operations through `protocol_level_tokens::token_client::TokenClient` and protocol-level lock workflows through `protocol_level_tokens::lock_client`. That split gives the CLI the required capabilities, but it also means the implementation must present one user-facing `token` space while orchestrating two SDK client families, existing account/network resolution helpers, and the repository's established interactive submission patterns.

## Goals / Non-Goals

**Goals:**
- Add a top-level `token` command space that matches the intended taxonomy and uses protocol-near names.
- Reuse existing CLI account, network, node, review, submission, and finalization patterns instead of inventing a token-specific execution model.
- Map standard token operations through `TokenClient` and lock workflows through `lock_client` while keeping that split internal.
- Support token inspection through the existing `getTokenInfo` query path and lock inspection through the existing `getLockInfo` query path exposed by the pinned SDK branch.
- Keep the command surface and `docs/commands.md` synchronized.

**Non-Goals:**
- Introduce token operation batching or builder-style composition in this change.
- Define a broader generic transaction-authoring abstraction for all future command spaces.
- Add token portfolio or token discovery commands beyond direct token and lock inspection.
- Change the persisted wallet database structure or encryption model.

## Decisions

### 1. Expose a single `token` command space with protocol-near names
The CLI will implement `token show`, `token transfer`, `token mint`, `token burn`, `token allow-list add/remove`, `token deny-list add/remove`, `token pause`, `token unpause`, `token admin-roles assign/revoke`, `token metadata update`, and `token lock ...` subcommands.

This keeps the user-facing taxonomy close to the protocol and SDK naming while resolving the earlier ambiguity between `token send` and `token lock send`. `transfer` is a better top-level holder-operation name because the lock branch already has a distinct `send` term in the SDK.

Alternatives considered:
- `token send` and `token roles grant/revoke`: friendlier wording, but more ambiguous and less aligned with the protocol vocabulary.
- Separate top-level `lock` commands: simpler implementation split, but weaker user-facing grouping and inconsistent with the documented taxonomy.

### 2. Standard token operations will be implemented through `TokenClient`
Holder and token-admin flows will use `TokenClient` convenience methods where the SDK already defines them, including transfer, mint, burn, allow-list updates, deny-list updates, pause, unpause, admin-role assignment/revocation, and metadata updates. The CLI will still own argument parsing, network/account resolution, confirmation output, and finalization handling.

This lets the SDK continue to encapsulate token-operation construction, validation hooks, nonce lookup, and signing payload layout, while the CLI stays focused on user interaction.

Alternatives considered:
- Build all token operations directly with low-level `operations::*` helpers and `send::token_update_operations`: viable, but it would duplicate SDK-level orchestration and validation entry points.
- Build a new repository-local token execution abstraction first: unnecessary indirection for the first implementation.

### 3. Lock workflows will be implemented through `lock_client` and surfaced under `token lock`
Lock creation, funding, sending, returns, cancellation, and inspection will use the SDK's `create_lock`, `create_lock_proposal`, `LockClient`, and lock-info query support. The CLI will present those actions as nested `token lock` commands rather than exposing the SDK split directly.

This preserves a coherent user-facing command family while still taking advantage of the branch-specific lock APIs that do not live on `TokenClient`.

Alternatives considered:
- Treat lock workflows as hidden implementation details of generic token operations: not possible for user-initiated lock lifecycle commands.
- Model lock workflows as contract-like commands: technically inaccurate because these are protocol-level lock operations.

### 4. Share existing account/network resolution and transaction UX patterns
All mutating token commands follow the same interaction model as other account-signed commands in the repository:
- resolve network and node context through existing helpers
- resolve and unlock a signer account through existing account-selection/export helpers
- print a review summary before submission
- submit the transaction and optionally wait for finalization

This keeps token commands consistent with contract and account authoring flows and avoids a second interactive style.

Alternatives considered:
- Add token-specific prompt and context resolution helpers from scratch: more duplication, less consistency.
- Make token commands fully non-interactive first: faster to implement, but inconsistent with the current CLI style.

### 5. Add token and lock inspection as read-only query commands now, and defer broader token portfolio queries
`token show` will query token state through the SDK's token info support and present a human-readable token summary. `token lock show` will query lock state through the SDK's lock info support and present a human-readable lock summary. This change will not add token-account-state, token balance, or broader portfolio query commands.

This keeps the initial surface aligned with the explored scope: implement the execution workflows plus the direct inspection paths that are most naturally paired with the token and lock branches.

Alternatives considered:
- Defer `token show` until a wider token query story exists: would leave the top-level token branch without a direct inspection command even though the SDK exposes a dedicated query.
- Omit `token lock show` until a wider query story exists: would leave the lock branch operationally incomplete.

### 6. All token mutation arguments use interactive prompt fallback when omitted
Every positional and required argument across all token mutation commands is declared optional at the clap level. When a value is missing in interactive mode, the CLI prompts for it. In `--non-interactive` mode, missing required values produce actionable errors.

This gives the commands full usability from a bare invocation (`ccd-wallet token transfer`) while still accepting explicit values on the command line for scripting.

Alternatives considered:
- Keep required positional args and require explicit flags for scripting: simpler clap model, but forces every interactive session to pre-compose the full command.

### 7. Lock mutation commands always present the account selector; token mutation commands do the same
All token mutation commands pass `always_prompt_account = true` to the account resolution path. This ensures the account selector is always shown even if only one account is configured for the network, making the signer explicit for every submission.

Alternatives considered:
- Auto-select when there is only one account: fewer keystrokes, but silently commits to an account the user may not have intended.

### 8. `token transfer` token selection uses the account's available balances
When the token identifier is omitted from `token transfer`, the CLI queries the signer account's token balances (`getAccountInfo`) and presents an interactive selector populated with every token that has a non-zero available balance, with the available amount shown as a hint. Explicit `--token` / positional supply skips the query entirely.

For lock fund/send/return, token selection is constrained to the lock's configured token set (from `getLockInfo`) with locked/available balance hints as applicable.

Alternatives considered:
- Text prompt for token ID in all cases: always requires the user to know and type the identifier.

### 9. Lock fund/send/return use `--token` instead of a positional token argument
The `token_id` argument in `token lock fund`, `token lock send`, and `token lock return` is a named `--token` flag rather than a second positional argument. This avoids positional ambiguity when `LOCK_ID` is also optional and prompting is involved.

Alternatives considered:
- Keep `TOKEN_ID` as the second positional: works when both lock and token are supplied, but is fragile when only the lock ID is given on the command line and the token is meant to be prompted.

### 10. MetaUpdate transaction events render as human-readable one-line summaries
The finalization output for MetaUpdate transactions (token transfers, lock fund/send/return/cancel/create) renders each event as a concise single line rather than a pretty-printed JSON array. Known event types (`TokenTransfer`, `LockCreated`, `LockDestroyed`, `TokenMint`, `TokenBurn`, `TokenModuleEvent`) have dedicated formatters. Unknown events fall back to compact inline JSON.

Token transfer lines show lock context (`locked @ <lock-id>`) when `fromLock`/`toLock` fields are present in the event payload.

Alternatives considered:
- Always pretty-print the raw events JSON: consistent with existing contract/governance commands, but verbose and hard to scan for lock workflows.

## Risks / Trade-offs

- **[`TokenClient` and `lock_client` expose different orchestration styles]** → Mitigation: keep the SDK split inside a small `commands::token` module tree and normalize review/submission UX at the CLI layer.
- **[Protocol-near role names are accurate but verbose]** → Mitigation: use kebab-case CLI spellings derived directly from SDK role names and document them clearly in help text and command docs.
- **[Token amount handling depends on token decimals fetched from chain state]** → Mitigation: resolve token metadata before parsing decimal amounts and use the SDK's existing conversion helpers consistently.
- **[Lock creation has both simple and composed SDK entry points]** → Mitigation: keep composition out of scope and implement only the explicit command actions needed by the taxonomy.
- **[Token commands expand the clap surface substantially]** → Mitigation: isolate the implementation in a dedicated `commands::token` module tree with one module per feature branch where practical.

## Migration Plan

- Add the new clap command branch and command dispatcher.
- Implement token execution modules plus token and lock inspection using the pinned SDK branch.
- Update `docs/commands.md` so the `token` branch changes from planned to implemented and reflects the final names.
- Add or update tests for clap parsing and targeted command behavior.
- Rollback is straightforward because the change is additive: removing the new `token` branch from clap and command dispatch returns the CLI to its current state.

## Open Questions

All open questions from the initial proposal have been resolved by the implementation:

- **`token show` output level**: shows token ID, module reference, decimals, total supply, and all decoded module state fields (name, metadata, governance account, feature flags).
- **`token lock show` output level**: shows lock ID, expiry, recipients, controller type and grants, and current locked fund balances per account per token.
- **Role name format**: protocol-near kebab-case values (`update-admin-roles`, `mint`, `burn`, `update-allow-list`, `update-deny-list`, `pause`, `update-metadata`) are used in help text, accepted on the CLI, and presented in the interactive multi-select.
