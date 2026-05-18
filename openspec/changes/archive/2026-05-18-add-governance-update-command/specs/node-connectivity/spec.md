## ADDED Requirements

### Requirement: Governance update submission uses live chain queries when context is not explicit
The CLI SHALL use live Concordium node queries during governance update submission when authorization, key-index, or sequence-number context is not fully specified by the user.

#### Scenario: Known governance update derives signing context from chain state
- **WHEN** the user runs `ccd-wallet governance update` with a payload the wallet can deserialize
- **THEN** the CLI queries live chain state as needed to determine authorization structures, governance key indices, and next update sequence numbers

#### Scenario: Blind signing resolves selected verify keys to current key indices
- **WHEN** the user runs `ccd-wallet governance update --serialized <HEX> --blind --key <VERIFY_KEY>`
- **AND** the wallet can query the resolved node
- **THEN** the CLI uses live chain parameters to map the selected verify keys to their current governance key indices before constructing the update instruction

#### Scenario: Explicit sequence number skips default lookup
- **WHEN** the user runs `ccd-wallet governance update ... --sequence-number <N>`
- **THEN** the CLI uses the explicit sequence number instead of requiring default next-sequence lookup from the node

#### Scenario: Governance update submission surfaces node-query failure actionably
- **WHEN** the CLI needs live governance query data during governance update submission
- **AND** the resolved node cannot be reached or queried successfully
- **THEN** the command exits with a non-zero status
- **AND** prints an actionable error describing which live governance query failed
