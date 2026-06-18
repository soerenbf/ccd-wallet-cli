## Context

The CLI currently has no dedicated top-level command space for native CCD transfer authoring. Plain account transactions therefore have no clear user-facing home even though the repository already has separate top-level command spaces for contracts, staking, governance, and protocol-level tokens.

This change introduces a focused `ccd` command space with two initial authoring flows: `ccd transfer` for simple transfers and `ccd schedule` for scheduled transfers. The change crosses multiple areas: clap command definitions, new command modules, signing-source-aware sender resolution, Ledger signing, submission/finalization output, and the canonical command taxonomy in `docs/commands.md`.

The implementation also needs to work with the existing account-source model. Seed-backed and imported accounts can already produce local signing material, while Ledger-backed accounts require transaction-family-specific Ledger request construction and device signing.

## Goals / Non-Goals

**Goals:**
- Introduce a top-level `ccd` command space for native CCD account-transaction authoring.
- Implement `ccd transfer` for simple transfer and transfer-with-memo flows.
- Implement `ccd schedule` for scheduled transfer and scheduled-transfer-with-memo flows.
- Support seed-backed, imported, and Ledger-backed finalized local accounts as signing sources from the start.
- Keep the user-facing CLI surface explicit and consistent with the existing input-resolution and finalization conventions.
- Update `docs/commands.md` and the command-taxonomy spec to make the new command space canonical.

**Non-Goals:**
- Detached transaction proposal, signing, or submission workflows.
- True multisig account-transaction support.
- Additional native account transaction families beyond simple transfer and scheduled transfer.
- Broad refactoring of every existing transaction-submitting command onto a unified generic signer abstraction.

## Decisions

### 1. Add a dedicated `ccd` top-level command space
The CLI will expose `ccd transfer` and `ccd schedule` as user-facing native CCD authoring commands rather than placing them under `transaction`.

Rationale:
- `transaction` is reserved for transaction lifecycle and inspection concepts, not user-facing domain authoring.
- The repository already uses domain-oriented top-level spaces such as `token`, `contract`, `stake`, and `governance`.
- A dedicated `ccd` space gives native CCD transfers a stable home before future detached transaction workflows are added elsewhere.

Alternatives considered:
- Add `transaction transfer`: rejected because it blurs lifecycle/inspection concerns with domain authoring.
- Add `account transfer`: rejected because `account` already means wallet-entity management rather than on-chain mutation.

### 2. Keep `transfer` and `schedule` as separate command paths
`ccd transfer` will cover simple transfer and transfer with memo. `ccd schedule` will cover scheduled transfer and scheduled transfer with memo.

Rationale:
- The payload families have materially different input shapes and review requirements.
- A simple transfer has one amount, while a scheduled transfer has multiple timestamped releases.
- Separate command paths make help output, validation, and interactive prompting clearer.

Alternatives considered:
- One `ccd transfer` command with a `--schedule` mode: rejected because it overloads a simple command with a second input model.

### 3. Model scheduled releases as repeated `--release <RFC3339=CCD>` options
The scheduled-transfer CLI will accept one repeated `--release` option per release entry.

Rationale:
- Each release entry is a single atomic `(timestamp, amount)` pair.
- Repeated paired values are easier to validate and explain than separate repeated `--time` and `--amount` lists.
- RFC3339 timestamps make timezone handling deterministic in scripts and non-interactive mode.

Alternatives considered:
- Separate `--release-time` and `--release-amount` flags: rejected as fragile and position-dependent.
- Inline JSON or TOML schedule input only: rejected as too heavy for the common case.

### 4. Reuse existing sender and account-reference resolution semantics
Sender inputs for `ccd` mutations will resolve through the existing local-account signing rules, while non-sender recipient inputs will continue to accept either raw account addresses or finalized local account labels.

Rationale:
- This keeps `ccd` aligned with the existing input model used by token and stake mutation commands.
- Existing specs already define signer-capable sender resolution and local account-reference behavior.
- Reusing those semantics reduces surprise and limits new validation logic to the CCD-specific payloads.

Alternatives considered:
- Allow raw sender addresses: rejected because they do not provide local signing authority.
- Introduce a new CCD-specific sender-resolution model: rejected as unnecessary divergence.

### 5. Add a CCD-specific signing/submission layer that branches by account source
The implementation will add a focused native-CCD submission path that resolves the sender account record, then signs via:
- local wallet signing material for seed-backed and imported accounts
- Concordium Ledger app signing requests for Ledger-backed accounts

Rationale:
- Local and Ledger-backed accounts require different signing mechanics.
- The Ledger crate already exposes transfer, transfer-with-memo, scheduled-transfer, and scheduled-transfer-with-memo signing commands.
- Limiting the first abstraction to the new `ccd` command space keeps the change scoped while still supporting Ledger-backed accounts from day one.

Alternatives considered:
- Reuse only the existing local-wallet signer path: rejected because Ledger-backed account support is required from the start.
- Generalize every transaction-submitting command to a new global signer abstraction first: rejected as a larger refactor than this change needs.

### 6. Preserve existing confirmation and finalization conventions
Both `ccd` mutation commands will present a final human-readable review before submission, submit only after explicit approval in interactive mode, wait for finalization by default, and support `--no-wait`.

Rationale:
- This matches the current CLI behavior for other chain-mutating commands.
- Native CCD transfer commands should feel operationally consistent with contract, stake, and token mutation flows.

## Risks / Trade-offs

- **[Ledger signing implementation is more complex than local signing]** → Keep the first Ledger branch narrowly scoped to the transfer and scheduled-transfer payload families already supported by `ccd-wallet-ledger`.
- **[A new `ccd` space could drift from existing input conventions]** → Reuse the shared sender-resolution, recipient-resolution, input-mode, and finalization policies already established elsewhere in the CLI.
- **[Scheduled transfer parsing can be error-prone]** → Require deterministic `RFC3339=CCD` release entries and validate every entry before confirmation.
- **[Transfer finalization rendering may be less polished than token rendering initially]** → Reuse the existing transaction summary pipeline first and add CCD-specific rendering improvements only where necessary for clarity.

## Migration Plan

- Add the new `ccd` command space and document it in `docs/commands.md` as part of the same change.
- Leave existing command spaces untouched so there is no user migration burden for current workflows.
- If the change is partially reverted, remove the `ccd` command branch and its documentation together to keep taxonomy and implementation aligned.

## Open Questions

- Whether future native account transaction families such as register-data should eventually live under `ccd` is intentionally deferred.
- Detached transaction proposal and signing flows are intentionally out of scope for this change and can be layered on later without changing the initial `ccd` taxonomy.
