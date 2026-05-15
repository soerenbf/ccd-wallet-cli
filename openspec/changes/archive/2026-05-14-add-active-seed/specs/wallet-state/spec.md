## ADDED Requirements

### Requirement: Active seed stored in wallet_state table
The CLI SHALL persist the active seed label as a key/value row in the `wallet_state` table using key `active_seed`.

#### Scenario: Active seed persisted to wallet_state
- **WHEN** the user runs `ccd-wallet seed use <LABEL>`
- **AND** `<LABEL>` is a configured seed label
- **THEN** the CLI writes a row `("active_seed", "<LABEL>")` to `wallet_state`
- **AND** exits with a confirmation message

#### Scenario: Active seed read from wallet_state
- **WHEN** the user runs `ccd-wallet seed show` without a label
- **THEN** the CLI reads `active_seed` from `wallet_state`
- **AND** uses it to resolve which seed should be unlocked and displayed

#### Scenario: Missing active_seed key produces actionable error
- **WHEN** a command requires an active seed
- **AND** `wallet_state` has no `active_seed` key
- **THEN** the CLI exits with a non-zero status
- **AND** prints an error advising the user to run `ccd-wallet seed use <LABEL>` or provide a label explicitly
