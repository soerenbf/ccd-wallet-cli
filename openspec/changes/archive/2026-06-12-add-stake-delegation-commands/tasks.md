## 1. Command surface and shared stake helpers

- [x] 1.1 Add `stake show`, `stake configure delegation`, and `stake remove` clap definitions and command dispatch, while reserving the `stake configure validator` branch.
- [x] 1.2 Create stake command modules that reuse existing account, signer, and network resolution helpers.
- [x] 1.3 Add shared stake helpers for querying account staking state, rendering delegation and validator details, and validating validator ids against live chain state.

## 2. Stake inspection and mutation flows

- [x] 2.1 Implement `stake show` to resolve either a local account label or raw account address and render staking-mode-specific details, including delegation targets, validator details, restake state, and pending changes from live account state.
- [x] 2.2 Implement `stake configure delegation` to build patch-style `ConfigureDelegation` payloads from explicit or interactive user input, including permitted zero-capital configurations.
- [x] 2.3 Implement validator-target validation before submission for validator-directed delegation changes.
- [x] 2.4 Implement explicit confirmation messaging for validator-to-delegator transitions and other stake mutations.
- [x] 2.5 Implement `stake remove` as a generic removal flow that removes either delegation or validator staking based on current on-chain staking mode.
- [x] 2.6 Reuse existing transaction submission and finalization wait patterns, including `--no-wait` behavior and stake outcome reporting.

## 3. Inspection, documentation, and verification

- [x] 3.1 Extend `account show --verbose` to distinguish validator and delegated staking and render staking details, targets, restake state, and pending changes.
- [x] 3.2 Update `docs/commands.md` and `README.md` to document the implemented `stake` command surface and examples.
- [x] 3.3 Add or update tests covering clap parsing, validator-id validation behavior, stake state rendering, and stake command success and failure paths.
- [x] 3.4 Run project-appropriate verification for the Rust workspace and confirm the new command surface matches the documented taxonomy.
