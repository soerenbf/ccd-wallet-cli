## 1. Document the command taxonomy

- [x] 1.1 Audit the current top-level and nested CLI command structure in `crates/ccd-wallet/src/cli.rs` and related command modules.
- [x] 1.2 Draft `docs/commands.md` as the canonical command taxonomy document, labeling implemented versus planned command branches.
- [x] 1.3 Document the intended `token` grouping, including send, policy, roles, metadata, and lock branches without introducing `metaupdate` as a user-facing namespace.
- [x] 1.4 Document the intended staking grouping, including separate validator and delegation branches under the staking area.
- [x] 1.5 Ensure the validator-oriented staking section excludes deprecated legacy baker transaction families and reflects modern `ConfigureBaker`-based behavior only.

## 2. Add synchronization guidance

- [x] 2.1 Update `AGENTS.md` with a rule that command-surface code changes and `docs/commands.md` must be kept in sync.
- [x] 2.2 Review the new guidance and `docs/commands.md` together to ensure they describe the same source-of-truth workflow.

## 3. Validate the proposal outcome

- [x] 3.1 Check the documented taxonomy against the current codebase to ensure implemented sections are accurate.
- [x] 3.2 Verify that planned sections clearly avoid overcommitting on future composition syntax while reserving room for token-operation composition.
