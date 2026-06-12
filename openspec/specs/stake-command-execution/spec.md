# stake-command-execution Specification

## Purpose
TBD - created by archiving change add-stake-delegation-commands. Update Purpose after archive.
## Requirements
### Requirement: Stake inspection command
The CLI SHALL provide a `ccd-wallet stake show <ACCOUNT>` command that resolves either a stored local account label or a raw account address on the selected network or node and renders the account's current staking mode and staking details from live chain state.

#### Scenario: Show delegated local account targeting a validator pool
- **WHEN** the user runs `ccd-wallet stake show alice --network testnet`
- **AND** `alice` resolves to a finalized local account on `testnet`
- **AND** the queried account is currently delegating to validator `42`
- **THEN** the CLI queries live account information from the selected node
- **AND** renders that the account is delegating
- **AND** renders the delegated stake amount
- **AND** renders validator `42` as the current target
- **AND** renders whether earnings are restaked

#### Scenario: Show passive delegation target
- **WHEN** the user runs `ccd-wallet stake show alice --network testnet`
- **AND** the queried account is currently delegating passively
- **THEN** the CLI renders passive delegation as the current target
- **AND** does not require the user to infer that no validator id is present

#### Scenario: Show validator staking details
- **WHEN** the user runs `ccd-wallet stake show validator-1 --network testnet`
- **AND** `validator-1` currently has validator staking configured
- **THEN** the CLI renders that the account is validating
- **AND** renders the validator id
- **AND** renders the staked amount
- **AND** renders whether earnings are restaked
- **AND** renders validator-specific pool details returned by the chain when available

#### Scenario: Show raw account address without wallet unlock
- **WHEN** the user runs `ccd-wallet stake show <ADDRESS> --network testnet`
- **AND** `<ADDRESS>` is a raw Concordium account address
- **THEN** the CLI queries live account information for that address from the selected node
- **AND** does not prompt for a seed password or imported account vault password

#### Scenario: Show pending staking change
- **WHEN** the user runs `ccd-wallet stake show alice --network testnet`
- **AND** the queried account has a pending staking reduction or removal
- **THEN** the CLI renders the pending change kind
- **AND** renders the effective time returned by the chain

### Requirement: Stake delegation configuration command
The CLI SHALL provide a `ccd-wallet stake configure delegation <ACCOUNT>` command that resolves a signing account, builds a modern `ConfigureDelegation` transaction from user-supplied changes, submits it to the selected network, and optionally waits for finalization.

#### Scenario: Configure delegation to a validator pool
- **WHEN** the user runs `ccd-wallet stake configure delegation alice --network testnet --validator 42 --capital 1000 --restake`
- **AND** `alice` resolves to a finalized local account that can sign on `testnet`
- **AND** validator `42` is valid on the selected network
- **THEN** the CLI builds a `ConfigureDelegation` transaction targeting validator `42`
- **AND** sets delegated capital to `1000 CCD`
- **AND** sets restake earnings to enabled
- **AND** submits the transaction from `alice`

#### Scenario: Configure passive delegation
- **WHEN** the user runs `ccd-wallet stake configure delegation alice --network testnet --passive --capital 500 --no-restake`
- **AND** `alice` resolves to a finalized local account that can sign on `testnet`
- **THEN** the CLI builds a `ConfigureDelegation` transaction targeting passive delegation
- **AND** sets delegated capital to `500 CCD`
- **AND** sets restake earnings to disabled

#### Scenario: Configure partial delegation changes
- **WHEN** the user runs `ccd-wallet stake configure delegation alice --network testnet` with only one or more delegation fields supplied
- **THEN** the CLI updates only the user-specified delegation fields in the `ConfigureDelegation` payload
- **AND** does not require unchanged delegation fields to be resupplied

#### Scenario: Zero capital is permitted in delegation configuration
- **WHEN** the user runs `ccd-wallet stake configure delegation alice --network testnet --capital 0`
- **THEN** the CLI accepts the supplied capital value
- **AND** builds the corresponding `ConfigureDelegation` payload without rejecting the command as an invalid removal path

#### Scenario: Wait behavior follows command options
- **WHEN** the user runs `ccd-wallet stake configure delegation ...` without `--no-wait`
- **THEN** the CLI waits for transaction finalization after successful submission
- **AND** renders the final delegation outcome

#### Scenario: Non-wait returns after submission
- **WHEN** the user runs `ccd-wallet stake configure delegation ... --no-wait`
- **THEN** the CLI returns after successful submission
- **AND** renders the submitted transaction hash without waiting for finalization

### Requirement: Validator targets are validated before submission
The CLI SHALL validate validator-targeted delegation changes against live chain state before submitting a `ConfigureDelegation` transaction.

#### Scenario: Known validator id is accepted
- **WHEN** the user runs `ccd-wallet stake configure delegation alice --validator 42 ...`
- **AND** validator `42` exists on the selected network
- **THEN** the CLI accepts the validator id for submission

#### Scenario: Unknown validator id is rejected before submission
- **WHEN** the user runs `ccd-wallet stake configure delegation alice --validator 999999 ...`
- **AND** validator `999999` does not exist on the selected network
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating that the validator id is not valid on the selected network
- **AND** does not submit a transaction

### Requirement: Validator-to-delegator switching is supported explicitly
The CLI SHALL allow a currently validating account to switch into delegation through the delegation configuration flow when the chain supports the requested change.

#### Scenario: Validator account switches to delegation
- **WHEN** the user runs `ccd-wallet stake configure delegation validator-1 --network testnet --validator 42 --capital 1000 --restake`
- **AND** `validator-1` currently has validator staking configured
- **AND** the requested delegation configuration is otherwise valid
- **THEN** the CLI presents the change as a staking-mode transition during confirmation
- **AND** submits a `ConfigureDelegation` transaction when the user approves

### Requirement: Generic stake removal command
The CLI SHALL provide a user-facing `ccd-wallet stake remove <ACCOUNT>` command that removes the account's currently configured staking mode.

#### Scenario: Remove existing delegation
- **WHEN** the user runs `ccd-wallet stake remove alice --network testnet`
- **AND** `alice` currently has delegation configured
- **THEN** the CLI builds the chain-equivalent delegation-removal transaction
- **AND** asks for approval using delegation-removal wording
- **AND** submits the removal transaction after approval

#### Scenario: Remove existing validator staking
- **WHEN** the user runs `ccd-wallet stake remove validator-1 --network testnet`
- **AND** `validator-1` currently has validator staking configured
- **THEN** the CLI builds the chain-equivalent validator-removal transaction
- **AND** asks for approval using validator-removal wording
- **AND** submits the removal transaction after approval

#### Scenario: Reject removal when no staking is configured
- **WHEN** the user runs `ccd-wallet stake remove alice --network testnet`
- **AND** `alice` has neither delegation nor validator staking configured
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating that no staking configuration exists to remove

