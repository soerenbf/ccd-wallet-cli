## ADDED Requirements

### Requirement: Set active seed command
The CLI SHALL provide a `seed use <LABEL>` command that sets the named seed as the active seed after validating that the seed exists.

#### Scenario: Select existing seed as active
- **WHEN** the user runs `ccd-wallet seed use main_seed`
- **AND** a seed labeled `main_seed` exists
- **THEN** the CLI writes `active_seed = main_seed` to the SQLite `wallet_state` table
- **AND** exits successfully with a confirmation message

#### Scenario: Reject unknown active seed selection
- **WHEN** the user runs `ccd-wallet seed use unknown_seed`
- **AND** no seed labeled `unknown_seed` exists
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating that the seed is not configured
- **AND** does NOT write an active seed selection to the wallet-state store

### Requirement: Show seed phrase command
The CLI SHALL provide a `seed show [LABEL]` command that prompts for the selected seed's password and temporarily displays the decrypted seed phrase only after successful authentication. The seed phrase SHALL be hidden when the user presses any key or after 30 seconds, whichever happens first.

#### Scenario: Show seed phrase by explicit label
- **WHEN** the user runs `ccd-wallet seed show main_seed`
- **AND** a seed labeled `main_seed` exists
- **AND** the user enters the correct password for `main_seed`
- **THEN** the CLI displays the decrypted seed phrase in a temporary terminal view
- **AND** hides the phrase when the user presses any key or after 30 seconds, whichever happens first
- **AND** exits successfully

#### Scenario: Wrong password does not reveal seed phrase
- **WHEN** the user runs `ccd-wallet seed show main_seed`
- **AND** a seed labeled `main_seed` exists
- **AND** the user enters an incorrect password
- **THEN** the CLI exits with a non-zero status
- **AND** does NOT enter the temporary reveal view
- **AND** does NOT display the seed phrase

#### Scenario: Show active seed phrase when label omitted
- **WHEN** the user runs `ccd-wallet seed show`
- **AND** `wallet_state` contains `active_seed = main_seed`
- **AND** a seed labeled `main_seed` exists
- **AND** the user enters the correct password for `main_seed`
- **THEN** the CLI displays the decrypted seed phrase in a temporary terminal view
- **AND** hides the phrase when the user presses any key or after 30 seconds, whichever happens first

#### Scenario: Missing active seed produces actionable error
- **WHEN** the user runs `ccd-wallet seed show`
- **AND** no active seed is set in the wallet-state store
- **THEN** the CLI exits with a non-zero status
- **AND** prints an error advising the user to run `ccd-wallet seed use <LABEL>` or provide a label explicitly

#### Scenario: Stale active seed produces actionable error
- **WHEN** the user runs `ccd-wallet seed show`
- **AND** `wallet_state` contains an `active_seed` value that no longer matches a stored seed label
- **THEN** the CLI exits with a non-zero status
- **AND** prints an error indicating that the active seed is no longer configured

#### Scenario: Unknown explicit seed produces actionable error
- **WHEN** the user runs `ccd-wallet seed show unknown_seed`
- **AND** no seed labeled `unknown_seed` exists
- **THEN** the CLI exits with a non-zero status
- **AND** prints an error indicating that the seed is not configured
- **AND** does NOT prompt for a password
