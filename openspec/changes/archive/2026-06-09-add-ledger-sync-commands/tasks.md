## 1. CLI surface and command wiring

- [x] 1.1 Extend the Ledger clap definitions with `ledger setup --restore <NETWORK>`, `ledger sync [LABEL]`, provider filters, network selection, `--non-interactive`, `--no-defaults`, and the explicit Ledger export allow flag.
- [x] 1.2 Update `main.rs` and Ledger command dispatch so Ledger flows can mutate wallet state and invoke async recovery orchestration.
- [x] 1.3 Add CLI parsing tests that cover the new `ledger sync` command shape and `ledger setup --restore` arguments.

## 2. Shared recovery orchestration

- [x] 2.1 Extract the seed-specific recovery pipeline into shared helpers that separate network/provider/import orchestration from source-specific recovery-material derivation.
- [x] 2.2 Keep existing seed recovery behavior unchanged by adapting `seed sync` and `seed add --restore` to the shared recovery helpers.
- [x] 2.3 Add focused tests for the shared recovery helpers so seed-backed behavior remains covered during the refactor.

## 3. Ledger-backed recovery implementation

- [x] 3.1 Implement Ledger recovery-material acquisition that verifies the enrolled owner, gates export behind one up-front interactive approval or an explicit non-interactive allow flag, and derives the transient identity/account recovery inputs needed by the shared pipeline.
- [x] 3.2 Implement `ledger sync` using the shared recovery helpers, including label-explicit resolution with interactive selector preselection for an active Ledger key source, sequential provider/identity/account probing, local password unlock, and recovery summaries.
- [x] 3.3 Extend `ledger setup` so successful enrollment can immediately run recovery when `--restore <NETWORK>` is supplied, while validating the target network before writing enrollment state.
- [x] 3.4 Add Ledger-focused tests for owner mismatch, missing network, missing non-interactive allow flag, declined approval, and successful sync/setup-and-restore flows.

## 4. Documentation and command taxonomy

- [x] 4.1 Update `docs/commands.md` to document `ledger sync` and `ledger setup --restore <NETWORK>` under the implemented Ledger command space.
- [x] 4.2 Review command help text, prompts, and error messages so they consistently describe Ledger-backed recovery as a key-source flow with explicit export approval.
