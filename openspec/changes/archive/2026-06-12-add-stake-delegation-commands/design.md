## Context

The repository already documents staking as a planned `stake` command space with separate validator and delegation branches, but the CLI does not yet implement stake commands. The Concordium SDK already exposes modern `ConfigureDelegation` transaction builders, validator-removal-compatible transaction support through the account transaction model, and node queries for current account staking state, baker lists, pool information, nonces, submission, and finalization. The change therefore fits the existing wallet model of resolving network and signer context locally, validating against live chain state, then signing and submitting an account transaction.

This change crosses multiple areas: clap command taxonomy, command dispatch, account/network resolution helpers, on-chain account inspection, and user-facing documentation. It also introduces a mode-transition case where an account may move from validator staking into delegation through a single chain transaction. The chain already defines the semantics, so the wallet's main responsibility is making the transition understandable and safe in the UI.

## Goals / Non-Goals

**Goals:**
- Implement an explicit `stake` command family for staking inspection and selected staking mutations.
- Support the modern `ConfigureDelegation` model rather than reviving legacy baker or delegation transaction families.
- Allow delegation configuration to target either passive delegation or a specific validator pool.
- Validate validator ids against live chain state before submission.
- Permit validator-to-delegator switching when the chain allows it, while making the transition explicit in prompts and confirmations.
- Improve read-side staking visibility so users can inspect current delegation state before or after mutation.

**Non-Goals:**
- Implement validator configuration commands in the same change beyond validator removal through `stake remove`.
- Introduce new persisted wallet data models for delegation state beyond existing network/account storage.
- Support offline delegation composition or detached signing workflows.
- Rework generic transaction rendering beyond delegation-specific outcome summaries needed for this flow.

## Decisions

### Decision: Use a top-level `stake` surface with generic show and remove actions
The command surface will implement `stake show <account>`, `stake configure delegation <account> ...`, and `stake remove <account>`, while reserving `stake configure validator <account> ...` for future validator-configuration work.

Rationale:
- `ConfigureDelegation` is inherently patch-like, so one configuration-oriented subcommand maps naturally to the chain primitive.
- `stake show` reads better as a staking-focused counterpart to `account show`.
- `stake remove` gives users one clear exit action from the currently configured staking mode, regardless of whether that mode is delegation or validator staking.
- The shape leaves room for a future `configure validator` branch without reshaping the top-level command grammar.

Alternatives considered:
- A nested `stake delegation ...` family only: rejected because it makes staking inspection and removal read less naturally and leaves top-level `stake` underused.
- One giant `configure` command only: rejected because removal becomes too implicit.
- Many intention-specific commands (`add`, `retarget`, `restake`, `set-capital`): rejected for v1 because they multiply surface area without adding new chain capabilities.

### Decision: Reuse existing signer and network resolution helpers
Stake commands will reuse the same account export, unlock, network resolution, nonce lookup, submission, and finalization-wait patterns already used by token and contract mutation commands.

Rationale:
- The wallet already has trusted patterns for resolving a local account into a `WalletAccount` signer.
- The repo already has account-inspection behavior for resolving raw addresses without wallet unlock, which `stake show` can mirror.
- Reuse keeps the command behavior consistent with other mutating flows.
- It minimizes new architecture and avoids parallel abstractions for account transactions.

Alternatives considered:
- Introduce a brand-new generic account-transaction mutation framework: rejected as unnecessary for this change.

### Decision: Validate validator targets through live chain queries before submission
When the user configures delegation to a validator pool, the wallet will validate the supplied validator id against live chain state before building the final transaction.

Rationale:
- This gives earlier, clearer feedback than relying only on chain rejection.
- It aligns with the user's expectation that the wallet should help catch obvious mistakes.
- The SDK already exposes baker and pool queries.

Alternatives considered:
- Submit blindly and rely on chain rejection: rejected because the UX is worse and user intent is clear enough to preflight.

### Decision: Support validator-aware transitions and generic stake removal
If the selected account is currently validating, the wallet will allow delegation configuration that switches the account into delegation when the chain supports it, and `stake remove` will remove validator staking as well as delegation. The wallet will present both cases as staking-mode transitions in the review step.

Rationale:
- The protocol already handles validator-to-delegator transition semantics.
- A generic `stake remove` action is a better user model than teaching separate remove commands per staking mode.
- An explicit confirmation step addresses the main UX risk for destructive or mode-switching operations.

Alternatives considered:
- Block validator-to-delegator switching in v1: rejected because it artificially narrows a supported chain behavior.
- Restrict `stake remove` to delegation only: rejected because it weakens the intended top-level `stake` grammar and forces users to reason about internal staking mode when issuing a remove action.

### Decision: Improve account inspection alongside delegation commands
This change will extend inspection requirements so users can see richer staking details, including whether staking is validator or delegated, the delegated target, restake setting, and pending stake changes.

Rationale:
- Delegation mutation flows are easier to understand when current state is inspectable.
- The node already returns the necessary staking information in `account_info`.
- This reduces the need for users to infer staking state from separate tools.

Alternatives considered:
- Keep inspection shallow and rely only on `stake delegation show`: rejected because `account show --verbose` is already an expected account-centric inspection entrypoint.

## Risks / Trade-offs

- **[Risk] `stake remove` spans more than one staking mode before full validator configuration exists** → Mitigation: make the command inspect current on-chain staking mode first and present mode-specific confirmation text before submission.
- **[Risk] Live validator validation can race with chain changes** → Mitigation: treat validation as a UX preflight only; final chain acceptance remains authoritative.
- **[Risk] Validator-to-delegator transitions may surprise users** → Mitigation: render current staking mode before confirmation and call out the mode switch explicitly in approval text.
- **[Risk] Delegation details shown in account inspection may diverge from stake-specific output if rendered by separate code paths** → Mitigation: centralize delegation/staking rendering helpers where practical.

## Migration Plan

- No database migration is required because delegation state remains chain-derived and signer/account storage already exists.
- Update clap command definitions, command dispatch, and `docs/commands.md` in the same change.
- Update `README.md` examples and command descriptions after the command surface is finalized.
- Rollback is straightforward: remove the new command surface and documentation; no persisted user data needs conversion.

## Open Questions

- Should validator-target validation require only that the validator id exists, or also that pool info is queryable and open to delegators?
- Should mutation confirmations print a chain-style patch summary (changed fields only) or a normalized post-change target state summary?
- How prominently should the CLI steer users toward `stake remove` when `stake configure delegation --capital 0` is also permitted?
