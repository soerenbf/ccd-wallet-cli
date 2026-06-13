## MODIFIED Requirements

### Requirement: Stake inspection command
The CLI SHALL provide a `ccd-wallet stake show <ACCOUNT>` command that resolves either a stored local account label or a raw account address on the selected network or node and renders the account's current staking mode and staking details from live chain state.

When `<ACCOUNT>` is a local account label and no explicit network was supplied, interactive `stake show` SHALL use account-assisted network resolution. The active network SHALL prefer a matching active-network account when one exists. If no active-network match exists and the label uniquely identifies a finalized local account on another configured network, the CLI SHALL infer that account's network. If multiple eligible matches remain, the CLI SHALL open an account selector that includes network and ownership metadata. Non-interactive `stake show` SHALL NOT infer the network from account-label uniqueness or let an account label override the active network.

#### Scenario: Show delegated local account targeting a validator pool
- **WHEN** the user runs `ccd-wallet stake show alice --network testnet`
- **AND** `alice` resolves to a finalized local account on `testnet`
- **AND** the queried account is currently delegating to validator `42`
- **THEN** the CLI queries live account information from the selected node
- **AND** renders that the account is delegating
- **AND** renders the delegated stake amount
- **AND** renders validator `42` as the current target
- **AND** renders whether earnings are restaked

#### Scenario: Show local account constrained by explicit network
- **WHEN** the user runs `ccd-wallet stake show alice --network testnet`
- **AND** `alice` exists as a finalized local account on another network but not on `testnet`
- **THEN** the CLI exits with an actionable error for `testnet`
- **AND** does not infer or switch to the other network

#### Scenario: Show active-network account for ambiguous label
- **WHEN** the user runs `ccd-wallet stake show alice` interactively
- **AND** no `--network` argument was supplied
- **AND** the active network is `testnet`
- **AND** finalized local accounts named `alice` exist on both `testnet` and another configured network
- **THEN** the CLI selects the `testnet` account
- **AND** does not prompt for network selection
- **AND** displays the resolved network and account context
- **AND** queries live staking details for that account

#### Scenario: Show unique local account label outside active network interactively
- **WHEN** the user runs `ccd-wallet stake show alice` interactively
- **AND** no `--network` argument was supplied
- **AND** the active network has no finalized local account named `alice`
- **AND** exactly one finalized local account named `alice` exists on another configured network
- **THEN** the CLI infers the network from that account
- **AND** does not prompt for network selection
- **AND** displays the resolved network and account context
- **AND** queries live staking details for that account

#### Scenario: Show ambiguous local account label without active-network match interactively
- **WHEN** the user runs `ccd-wallet stake show alice` interactively
- **AND** no `--network` argument was supplied
- **AND** no active-network account match resolves the label
- **AND** finalized local accounts named `alice` exist on multiple configured networks
- **THEN** the CLI opens an account selector over the matching accounts
- **AND** the selector rows show network and ownership metadata
- **AND** the selected account determines which network is queried

#### Scenario: Show unique local account label without network non-interactively fails
- **WHEN** the user runs `ccd-wallet stake show alice --non-interactive`
- **AND** no `--network` argument was supplied
- **AND** existing non-interactive network rules do not provide a concrete network
- **AND** exactly one finalized local account named `alice` exists across configured networks
- **THEN** the CLI exits with an actionable network-resolution error
- **AND** does not infer the network from `alice`

#### Scenario: Show local account label does not override active network non-interactively
- **WHEN** the user runs `ccd-wallet stake show alice --non-interactive`
- **AND** no `--network` argument was supplied
- **AND** the active network is `testnet`
- **AND** `alice` exists only on another configured network
- **THEN** the CLI exits with an actionable error for `testnet`
- **AND** does not infer or switch to the other network

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

When `<ACCOUNT>` is supplied interactively without an explicit network, the command SHALL follow shared signing-account/sender resolution for local account labels, including active-network soft-default behavior. Raw account addresses SHALL be rejected for stake configuration because the command must sign a transaction with the selected account. If `<ACCOUNT>` is omitted interactively, the active network SHALL remain useful as the account-selector scope before falling back to other network selection rules.

#### Scenario: Configure delegation with active-network sender match
- **WHEN** the user runs `ccd-wallet stake configure delegation alice` interactively
- **AND** no `--network` argument was supplied
- **AND** the active network is `testnet`
- **AND** finalized local accounts named `alice` exist on both `testnet` and another configured network
- **THEN** the CLI selects the `testnet` account
- **AND** does not prompt for network selection before preparing the transaction

#### Scenario: Configure delegation with unique local account label outside active network interactively
- **WHEN** the user runs `ccd-wallet stake configure delegation alice` interactively
- **AND** no `--network` argument was supplied
- **AND** the active network has no finalized local account named `alice`
- **AND** exactly one finalized local account named `alice` exists on another configured network
- **THEN** the CLI infers the network from that account
- **AND** does not prompt for network selection before preparing the transaction

#### Scenario: Configure delegation rejects raw account address
- **WHEN** the user runs `ccd-wallet stake configure delegation <ADDRESS>`
- **AND** `<ADDRESS>` is a raw Concordium account address
- **THEN** the CLI exits with an actionable error explaining that a local account label is required for signing
- **AND** does not submit a transaction

### Requirement: Generic stake removal command
The CLI SHALL provide a user-facing `ccd-wallet stake remove <ACCOUNT>` command that removes the account's currently configured staking mode.

When `<ACCOUNT>` is supplied interactively without an explicit network, the command SHALL follow shared signing-account/sender resolution for local account labels, including active-network soft-default behavior. Raw account addresses SHALL be rejected for stake removal because the command must sign a transaction with the selected account. If `<ACCOUNT>` is omitted interactively, the active network SHALL remain useful as the account-selector scope before falling back to other network selection rules.

#### Scenario: Remove staking with active-network sender match
- **WHEN** the user runs `ccd-wallet stake remove alice` interactively
- **AND** no `--network` argument was supplied
- **AND** the active network is `testnet`
- **AND** finalized local accounts named `alice` exist on both `testnet` and another configured network
- **THEN** the CLI selects the `testnet` account
- **AND** does not prompt for network selection before preparing the removal transaction

#### Scenario: Remove staking with unique local account label outside active network interactively
- **WHEN** the user runs `ccd-wallet stake remove alice` interactively
- **AND** no `--network` argument was supplied
- **AND** the active network has no finalized local account named `alice`
- **AND** exactly one finalized local account named `alice` exists on another configured network
- **THEN** the CLI infers the network from that account
- **AND** does not prompt for network selection before preparing the removal transaction

#### Scenario: Remove staking rejects raw account address
- **WHEN** the user runs `ccd-wallet stake remove <ADDRESS>`
- **AND** `<ADDRESS>` is a raw Concordium account address
- **THEN** the CLI exits with an actionable error explaining that a local account label is required for signing
- **AND** does not submit a transaction
