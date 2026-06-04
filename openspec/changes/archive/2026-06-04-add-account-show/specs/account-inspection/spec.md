## ADDED Requirements

### Requirement: Account show queries account state by address
The CLI SHALL provide `ccd-wallet account show <ACCOUNT>` to query on-chain account state for a raw Concordium account address on the selected network or node.

#### Scenario: Show raw address account state
- **WHEN** the user runs `ccd-wallet account show <ADDRESS>` with an account address and a resolvable network or node context
- **THEN** the CLI queries account information for `<ADDRESS>` from the selected node
- **AND** the CLI renders account balances for the queried account

#### Scenario: Raw address target does not require wallet unlock
- **WHEN** the user runs `ccd-wallet account show <ADDRESS>` with a raw account address
- **THEN** the CLI MUST NOT prompt for a seed password or imported account vault password to resolve the target

### Requirement: Account show resolves local account labels
The CLI SHALL allow `<ACCOUNT>` to identify a stored local account label, resolved within the selected network context.

#### Scenario: Show local derived account state
- **WHEN** the user runs `ccd-wallet account show alice --network testnet` and `alice` is a finalized derived account stored for `testnet`
- **THEN** the CLI unlocks the relevant seed context to read the stored account address
- **AND** queries account information for that address from the selected network node
- **AND** annotates the account header with the seed label and local account label

#### Scenario: Show local imported account state
- **WHEN** the user runs `ccd-wallet account show genesis --network local` and `genesis` is a finalized imported account stored for `local`
- **THEN** the CLI unlocks the relevant imported account vault to read the stored account address
- **AND** queries account information for that address from the selected network node
- **AND** annotates the account header with imported-account local metadata

#### Scenario: Local label is constrained by selected network
- **WHEN** the same local account label exists on multiple configured networks
- **AND** the user supplies or resolves a concrete network context
- **THEN** `account show` selects the account record for that network only

### Requirement: Account show reports pending local accounts
The CLI SHALL handle pending local account records without requiring a finalized account address.

#### Scenario: Pending local account has transaction hash
- **WHEN** the user runs `ccd-wallet account show alice --network testnet` and `alice` is pending with a stored transaction hash
- **THEN** the CLI renders pending status
- **AND** the CLI renders the transaction hash
- **AND** the CLI does not query account information for a missing finalized account address

#### Scenario: Pending local account has no transaction hash
- **WHEN** the user runs `ccd-wallet account show alice --network testnet` and `alice` is pending without a stored transaction hash
- **THEN** the CLI renders pending status
- **AND** the CLI explains that finalized on-chain account state is not available yet

### Requirement: Account show renders balance-oriented human output
The default human output for `account show` SHALL focus on account balances, locks, and release schedules rather than low-level protocol details.

#### Scenario: Render CCD balances
- **WHEN** account information is returned by the node
- **THEN** the CLI renders the CCD total balance
- **AND** renders the CCD available balance
- **AND** renders the CCD locked balance when the total balance exceeds the available balance

#### Scenario: Render CCD release schedule
- **WHEN** account information includes one or more release schedule entries
- **THEN** the CLI renders each release amount with its release timestamp in the CCD section

#### Scenario: Render protocol-level token balances
- **WHEN** account information includes protocol-level token balances
- **THEN** the CLI renders one section per token with the token identifier and total balance
- **AND** renders token available balance and locked balance when availability information indicates that some of the token balance is locked

#### Scenario: Omit empty token section
- **WHEN** account information contains no protocol-level token balances
- **THEN** the CLI does not render an empty token balances section

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

### Requirement: Account show supports JSON output
The CLI SHALL support `--json` for `account show` using a stable wallet-owned JSON schema.

#### Scenario: JSON output for account state
- **WHEN** the user runs `ccd-wallet account show <ACCOUNT> --json` and account information is returned by the node
- **THEN** the CLI emits valid JSON
- **AND** the JSON includes the queried account address, network context, CCD balance information, and token balance information
- **AND** the JSON does not use raw Rust debug formatting

#### Scenario: JSON output includes local metadata when applicable
- **WHEN** the user runs `ccd-wallet account show <LOCAL_LABEL> --json` for a stored account
- **THEN** the JSON includes local account metadata sufficient to identify the local label and source
