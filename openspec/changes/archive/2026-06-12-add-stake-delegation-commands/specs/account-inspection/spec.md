## MODIFIED Requirements

### Requirement: Account show hides protocol details unless verbose
The default human output for `account show` SHALL hide low-level protocol fields, and `--verbose` SHALL reveal them.

#### Scenario: Default output hides protocol fields
- **WHEN** the user runs `ccd-wallet account show <ACCOUNT>` without `--verbose`
- **THEN** the CLI does not render account nonce
- **AND** does not render account index

#### Scenario: Verbose output includes protocol fields
- **WHEN** the user runs `ccd-wallet account show <ACCOUNT> --verbose`
- **THEN** the CLI renders the account nonce
- **AND** renders the account index
- **AND** renders additional protocol details available from account information such as credential count and account threshold
- **AND** renders staking mode details when staking is configured

## ADDED Requirements

### Requirement: Account show surfaces staking details
When account staking is configured, `ccd-wallet account show --verbose` SHALL distinguish validator staking from delegated staking and SHALL render the staking details returned by the chain.

#### Scenario: Verbose output renders delegated staking details
- **WHEN** the user runs `ccd-wallet account show alice --network testnet --verbose`
- **AND** `alice` is currently delegating to validator `42`
- **THEN** the CLI renders that staking mode is delegated
- **AND** renders the delegated stake amount
- **AND** renders validator `42` as the delegation target
- **AND** renders whether earnings are restaked

#### Scenario: Verbose output renders passive delegation details
- **WHEN** the user runs `ccd-wallet account show alice --network testnet --verbose`
- **AND** `alice` is currently delegating passively
- **THEN** the CLI renders that staking mode is delegated
- **AND** renders passive delegation as the target
- **AND** renders the delegated stake amount

#### Scenario: Verbose output renders validator staking details distinctly
- **WHEN** the user runs `ccd-wallet account show validator-1 --network testnet --verbose`
- **AND** `validator-1` currently has validator staking configured
- **THEN** the CLI renders that staking mode is validator
- **AND** renders the staked amount
- **AND** renders whether earnings are restaked
- **AND** renders validator-specific details available from chain state distinctly from delegation details

#### Scenario: Verbose output renders pending staking change
- **WHEN** the user runs `ccd-wallet account show alice --network testnet --verbose`
- **AND** the account has a pending staking reduction or removal
- **THEN** the CLI renders the pending change kind
- **AND** renders the effective time returned by the chain
