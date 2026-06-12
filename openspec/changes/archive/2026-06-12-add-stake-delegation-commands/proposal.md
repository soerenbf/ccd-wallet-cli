## Why

The wallet currently lacks a wallet-native stake command space for inspecting and managing on-chain CCD staking state. Users need a consistent way to inspect stake configuration, configure delegation, and remove either delegation or validator staking without falling back to external tooling.

## What Changes

- Add a top-level `stake` command family for on-chain staking workflows.
- Add `stake show <account>` so users can inspect the selected account's current staking mode and staking details, including delegated target, stake amount, restake setting, validator details where relevant, and pending stake changes.
- Add `stake configure delegation <account> ...` commands that can submit modern `ConfigureDelegation` transactions for:
  - creating delegation
  - updating delegated capital
  - updating restake behavior
  - retargeting delegation between passive delegation and a validator pool
  - switching a currently validating account into delegation when supported by chain rules
- Add `stake remove <account>` as an explicit user-facing flow that removes the account's current staking mode, covering both delegation removal and validator removal.
- Validate validator pool identifiers against live chain state before submitting validator-targeted delegation changes.
- Reserve `stake configure validator <account> ...` as the future validator-configuration branch while implementing validator-aware removal now.
- Update command taxonomy and user documentation to describe the implemented `stake` command surface.

## Capabilities

### New Capabilities
- `stake-command-execution`: Execute wallet-native staking inspection and mutation flows that resolve signer context, validate delegation targets against chain state, submit delegation-configuration transactions, remove configured staking modes, and report outcomes.

### Modified Capabilities
- `command-taxonomy`: Change the documented staking taxonomy from planned-only guidance to include an implemented top-level `stake` command surface with `show`, `configure delegation`, and `remove`.
- `account-inspection`: Change account inspection requirements so account-oriented inspection can surface richer staking and delegation details instead of only reporting whether staking is generically configured.

## Impact

- Affected code: `crates/ccd-wallet/src/cli.rs`, new or updated `crates/ccd-wallet/src/commands/stake/*` modules, shared account/network resolution helpers, validator/delegation-aware transaction helpers, and staking-aware rendering in account inspection or stake-specific output.
- Affected docs: `docs/commands.md`, `README.md`, and new or updated OpenSpec capability specs.
- Dependencies/systems: Concordium Rust SDK account transaction builders and node queries for account info, baker list or pool info, next nonce, transaction submission, and finalization wait flows.
