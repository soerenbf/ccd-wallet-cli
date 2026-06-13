## 1. Shared Input Foundation

- [x] 1.1 Add a shared command input module with `InputMode`, prompt/default policies, `Resolved<T>`, `Promptable<T>`, and `Defaultable<T>`.
- [x] 1.2 Add sync and async resolution methods that require callers to provide prompt/default providers at resolution time.
- [x] 1.3 Add shared `FinalizationPolicy` and `ValidationPolicy` helpers for `--no-wait` and validation flags.
- [x] 1.4 Add unit tests for promptable, defaultable, non-interactive, and `--no-defaults` behavior.

## 2. Shared CLI Argument Groups and Domain Inputs

- [x] 2.1 Add shared clap argument groups for input mode, network/node selection, network-only selection, and transaction submission waiting.
- [x] 2.2 Add domain/newtype parsers for account labels, network names, key-source labels, account references, and decimal CCD amounts.
- [x] 2.3 Replace duplicated common flag fields in the first refactored command slice with shared argument groups while preserving public flag names and conflicts.
- [x] 2.4 Add or update parser tests for domain/newtype inputs, including raw-address rejection for signing-account labels and address-or-label parsing for account references.

## 3. Stake Configure Delegation Vertical Slice

- [x] 3.1 Convert `StakeConfigureDelegationArgs` into shared flag groups and domain-oriented clap fields where applicable.
- [x] 3.2 Add `PreparedStakeConfigureDelegation` and `ResolvedStakeConfigureDelegation` types near the stake configure implementation.
- [x] 3.3 Model the delegation account and delegation target/capital/restake inputs as promptable values, with prompts that display current staking state and use current values as interactive defaults when defaults are allowed.
- [x] 3.4 Refactor delegation execution to resolve prepared inputs in explicit dependency order before building and submitting the transaction.
- [x] 3.5 Preserve existing validation, confirmation, submission, finalization, and non-interactive behavior with focused tests.

## 4. Token Mutation Commands

- [x] 4.1 Refactor token shared mutation context resolution to consume prepared signing-account, network/node, input-mode, and finalization inputs.
- [x] 4.2 Convert token transfer, mint, burn, pause, unpause, metadata, list, admin-role, and lock mutation argument handling to prepared input types.
- [x] 4.3 Parse token identifiers, lock identifiers, account references, and amount inputs into domain or unresolved-domain types as early as practical.
- [x] 4.4 Preserve token prompt fallback, account-reference autocomplete, token selectors, amount validation, and non-interactive errors with tests.

## 5. Token Compose REPL and Submit

- [ ] 5.1 Convert `TokenComposeSubmitArgs` and REPL `SubmitCommandArgs` into one shared prepared submit input and resolver.
- [ ] 5.2 Refactor REPL `OperationArgs` helpers to use promptable and optional semantics instead of ad hoc `take_or_prompt` behavior where practical.
- [ ] 5.3 Introduce plan-specific unresolved-domain input types for symbols such as `@sender`, local lock references, token identifiers, account references, and token amounts.
- [ ] 5.4 Preserve plan network inference, plan validation, saved plan format, and existing REPL prompts/completions with tests.

## 6. Contract Submission Commands

- [ ] 6.1 Refactor contract deploy-module, init, and update command args to use shared flag groups and prepared signing/submission inputs.
- [ ] 6.2 Parse contract addresses, module references, CCD amounts, and parameter sources into domain-oriented input types.
- [ ] 6.3 Preserve contract simulation, validation, confirmation, submission, finalization, and non-interactive behavior with tests.

## 7. Local Entity and Governance Cleanup

- [ ] 7.1 Refactor seed, ledger, identity, account, and network flows to distinguish promptable destructive targets from defaultable non-destructive context values.
- [ ] 7.2 Refactor governance key, proposal, and update flows to use shared input mode, network selection, finalization policy, and domain parsing where appropriate.
- [ ] 7.3 Remove obsolete ad hoc prompt/default helper code once each command family has moved to the prepared-input model.

## 8. Verification and Documentation

- [ ] 8.1 Run `cargo fmt` and fix formatting issues.
- [ ] 8.2 Run focused Rust tests for refactored command families.
- [ ] 8.3 Run broader `cargo test` or project-appropriate filtered test commands before marking implementation complete.
- [ ] 8.4 Verify `docs/commands.md` remains accurate because the public command surface is preserved; update it only if implementation changes command structure or taxonomy.
