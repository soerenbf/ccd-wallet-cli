## ADDED Requirements

### Requirement: Governance key inspection uses live chain queries
The CLI SHALL use live Concordium node queries to derive governance authorization state for governance key inspection flows. `governance keys list` SHALL query current chain parameters from the resolved node instead of relying on locally stored governance authorization snapshots.

#### Scenario: Governance key list derives authorization from chain parameters
- **WHEN** the user runs `ccd-wallet governance keys list`
- **THEN** the CLI connects to the resolved node
- **AND** queries current chain parameters
- **AND** derives governance key levels and authorization status from the live response

#### Scenario: Governance key list surfaces node query failure actionably
- **WHEN** the user runs `ccd-wallet governance keys list`
- **AND** the CLI cannot connect to the resolved node or query chain parameters
- **THEN** the command exits with a non-zero status
- **AND** prints an actionable error indicating that live governance state could not be queried
