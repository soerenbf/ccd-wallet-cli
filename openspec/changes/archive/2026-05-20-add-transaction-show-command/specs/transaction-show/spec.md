## ADDED Requirements

### Requirement: Show transaction lifecycle by hash
The CLI SHALL provide a top-level `transaction show <HASH>` command that queries a Concordium node for the current status of the supplied transaction hash.

#### Scenario: Show a received transaction
- **WHEN** the user runs `ccd-wallet transaction show <HASH>`
- **AND** the resolved node reports the transaction status as received
- **THEN** the CLI exits successfully
- **AND** prints the transaction hash and queried network/node context
- **AND** prints `Status: received`

#### Scenario: Show a finalized transaction
- **WHEN** the user runs `ccd-wallet transaction show <HASH>`
- **AND** the resolved node reports the transaction status as finalized
- **THEN** the CLI exits successfully
- **AND** prints the transaction hash and queried network/node context
- **AND** prints `Status: finalized`
- **AND** renders the finalized block hash together with transaction outcome details

#### Scenario: Unknown hash is shown as absent
- **WHEN** the user runs `ccd-wallet transaction show <HASH>`
- **AND** the resolved node does not know the transaction hash
- **THEN** the CLI exits successfully
- **AND** prints `Status: absent`
- **AND** includes guidance that the selected network or node may be wrong

### Requirement: Show committed and finalized transaction details in a stable human-oriented layout
For committed and finalized transactions, the CLI SHALL render stable transaction properties in a human-oriented layout before printing transaction-specific outcome details.

#### Scenario: Finalized output includes stable summary fields
- **WHEN** the user runs `ccd-wallet transaction show <HASH>`
- **AND** the resolved node reports a finalized transaction summary
- **THEN** the CLI prints the finalized block hash
- **AND** prints the block time as an RFC3339 UTC timestamp
- **AND** prints a one-line outcome summary such as success or rejected
- **AND** prints the transaction type and energy cost when those values are available from the summary

#### Scenario: Committed output renders one section per block
- **WHEN** the user runs `ccd-wallet transaction show <HASH>`
- **AND** the resolved node reports the transaction as committed in one or more blocks
- **THEN** the CLI prints `Status: committed`
- **AND** prints the number of committed blocks
- **AND** renders one committed-block section per returned block hash

### Requirement: Render variant-specific transaction details using the concrete summary type
The CLI SHALL match on the concrete `BlockItemSummaryDetails` variant for committed and finalized results and render stable fields specific to that variant before printing JSON only for the nested non-static payloads.

#### Scenario: Account transaction shows static fields and nested outcome details
- **WHEN** the user runs `ccd-wallet transaction show <HASH>`
- **AND** the resolved node reports an account-transaction summary
- **THEN** the CLI prints static account-transaction fields such as sender and cost
- **AND** prints a `Reject reason:` section for rejected account transactions
- **AND** prints an `Events: <N>` section for successful account transactions

#### Scenario: Credential deployment shows static fields without nested JSON payloads
- **WHEN** the user runs `ccd-wallet transaction show <HASH>`
- **AND** the resolved node reports a credential-deployment summary
- **THEN** the CLI prints static credential-deployment fields such as credential type, address, and registration id
- **AND** does not print a nested JSON payload section

#### Scenario: Chain update shows payload JSON
- **WHEN** the user runs `ccd-wallet transaction show <HASH>`
- **AND** the resolved node reports a chain-update summary
- **THEN** the CLI prints stable update fields such as effective time and update type
- **AND** prints a `Payload:` section with pretty-printed JSON for the update payload

#### Scenario: Token creation shows static fields and events
- **WHEN** the user runs `ccd-wallet transaction show <HASH>`
- **AND** the resolved node reports a token-creation summary
- **THEN** the CLI prints static token-creation fields such as token id, token module, and decimals
- **AND** prints an `Events: <N>` section with pretty-printed token event JSON

#### Scenario: Received transaction does not show nested variant details
- **WHEN** the user runs `ccd-wallet transaction show <HASH>`
- **AND** the resolved node reports the transaction status as received
- **THEN** the CLI does not print nested variant detail sections such as `Events:`, `Reject reason:`, or `Payload:`
