## Why

Several transaction-detail fields in `ccd-wallet` still require raw account addresses even when the wallet already knows the relevant local account by label. That creates unnecessary copy/paste friction, makes interactive flows less discoverable, and forces users to manually bridge from local wallet metadata to on-chain addresses right before submission.

## What Changes

- Add a shared account-reference resolution capability that accepts either a raw Concordium account address or a finalized local account label within the resolved network context.
- Extend transaction-detail inputs that currently accept non-sender account addresses so they can also resolve local account labels, including interactive prompt fallback when those values are omitted.
- Add interactive account-reference prompts with autocomplete suggestions for local account labels while still allowing pasted raw addresses.
- Show seed ownership in local-account suggestions using the existing bracketed style, for example `[main-seed] alice` and `[imported] baker-0`.
- Reuse already-unlocked local account ownership domains within a single command so resolving another local account from the same seed or imported-account vault does not prompt redundantly.

## Capabilities

### New Capabilities
- `account-reference-resolution`: resolve raw account addresses or finalized local account labels for non-sender command inputs, including interactive prompt behavior and unlock reuse.

### Modified Capabilities
- `token-command-execution`: token and lock commands will accept local account labels anywhere they currently accept non-sender account addresses.
- `contract-instance-execution`: `contract invoke --invoker` will accept either a raw account address or a finalized local account label.
- `interactive-cli-prompts`: supported account-reference prompts will use `cliclack` autocomplete over local accounts while still accepting pasted raw addresses.

## Impact

- Affected code: shared account resolution helpers in `crates/ccd-wallet/src/commands/account.rs`, token command helpers in `crates/ccd-wallet/src/commands/token/shared.rs`, contract invoke handling in `crates/ccd-wallet/src/commands/contract/invoke.rs`, and related prompt utilities.
- Affected specs/docs: new `account-reference-resolution` spec and deltas for `token-command-execution`, `contract-instance-execution`, and `interactive-cli-prompts`.
- Affected UX: transaction-detail prompts and explicit CLI flags for recipient/target/source/invoker-style fields become label-aware and seed-aware.
