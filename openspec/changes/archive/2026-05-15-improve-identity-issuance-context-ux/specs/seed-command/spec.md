## MODIFIED Requirements

### Requirement: Add seed command
The CLI SHALL provide a `seed add` command that adds a password-protected seed phrase to the encrypted seed store. Seed labels SHALL be non-empty and contain only ASCII alphanumeric characters, dash (`-`), and underscore (`_`). In interactive mode, the label SHALL be prompted if omitted; in `--non-interactive` mode, omitting the label SHALL be an error.

#### Scenario: Add a valid seed phrase
- **WHEN** the user runs `ccd-wallet seed add main_seed`
- **AND** enters a valid seed phrase
- **AND** enters matching password and confirmation values
- **THEN** the CLI stores the seed label `main_seed` in plaintext
- **AND** stores the normalized seed phrase only as encrypted seed payload
- **AND** exits successfully with a confirmation message

#### Scenario: Missing label is prompted interactively
- **WHEN** the user runs `ccd-wallet seed add`
- **AND** `--non-interactive` is not supplied
- **THEN** the CLI prompts for the seed label using `cliclack`
- **AND** continues with seed setup after the label is provided

#### Scenario: Missing label in non-interactive mode errors
- **WHEN** the user runs `ccd-wallet seed add --non-interactive`
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating that the seed label must be provided

#### Scenario: Duplicate seed label is rejected
- **WHEN** the user runs `ccd-wallet seed add main_seed`
- **AND** a seed labeled `main_seed` already exists
- **THEN** the CLI exits with a non-zero status
- **AND** prints an error indicating the label is already in use
- **AND** does NOT prompt for or store a seed phrase

#### Scenario: Invalid seed label is rejected
- **WHEN** the user runs `ccd-wallet seed add "main seed"`
- **THEN** the CLI exits with a non-zero status
- **AND** prints an error indicating that labels may contain only ASCII letters, digits, dash, and underscore
- **AND** does NOT prompt for or store a seed phrase

### Requirement: Sensitive seed inputs are interactive only
The CLI SHALL NOT accept seed phrases or seed passwords through command-line arguments. The seed phrase, password, and password confirmation SHALL be read through hidden interactive prompts, using `cliclack` for supported input collection.

#### Scenario: Seed phrase is prompted interactively
- **WHEN** the user runs `ccd-wallet seed add main_seed`
- **THEN** the CLI prompts for the seed phrase without echoing it to the terminal
- **AND** the seed phrase is not supplied through a CLI flag or positional argument

#### Scenario: Password confirmation mismatch rejected
- **WHEN** the user runs `ccd-wallet seed add main_seed`
- **AND** enters a valid seed phrase
- **AND** enters a password and a different confirmation value
- **THEN** the CLI exits with a non-zero status
- **AND** prints an error indicating the passwords do not match
- **AND** does NOT write a seed row

### Requirement: Set active seed command
The CLI SHALL provide a `seed use` command that sets the named seed as the active seed after validating that the seed exists. In interactive mode, omitting the label SHALL open a selector over configured seeds; in `--non-interactive` mode, omitting the label SHALL be an error.

#### Scenario: Select existing seed as active
- **WHEN** the user runs `ccd-wallet seed use main_seed`
- **AND** a seed labeled `main_seed` exists
- **THEN** the CLI writes `active_seed = main_seed` to the SQLite `wallet_state` table
- **AND** exits successfully with a confirmation message

#### Scenario: Missing label opens a selector for seed use
- **WHEN** the user runs `ccd-wallet seed use`
- **AND** `--non-interactive` is not supplied
- **THEN** the CLI opens a `cliclack` selector over configured seed labels
- **AND** preselects the active seed when one exists
- **AND** uses the selected label for active-seed selection

#### Scenario: Reject unknown active seed selection
- **WHEN** the user runs `ccd-wallet seed use unknown_seed`
- **AND** no seed labeled `unknown_seed` exists
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating that the seed is not configured
- **AND** does NOT write an active seed selection to the wallet-state store

### Requirement: Remove seed command
The CLI SHALL provide a `seed remove` command that removes a configured seed after explicit confirmation. In interactive mode, the label SHALL be prompted if omitted; in `--non-interactive` mode, omitting the label SHALL be an error.

#### Scenario: Remove existing seed after confirmation
- **WHEN** the user runs `ccd-wallet seed remove main_seed`
- **AND** a seed labeled `main_seed` exists
- **AND** the user confirms by typing `main_seed`
- **THEN** the CLI deletes the seed row
- **AND** SQLite cascades deletion to the seed's vault row
- **AND** the CLI exits successfully with a confirmation message

#### Scenario: Missing label is prompted for seed removal
- **WHEN** the user runs `ccd-wallet seed remove`
- **AND** `--non-interactive` is not supplied
- **THEN** the CLI prompts for the seed label using `cliclack`
- **AND** uses the entered label for confirmation and removal

#### Scenario: Remove rejects unknown seed
- **WHEN** the user runs `ccd-wallet seed remove missing_seed`
- **AND** no seed labeled `missing_seed` exists
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating that the seed is not configured

#### Scenario: Remove rejected when confirmation does not match
- **WHEN** the user runs `ccd-wallet seed remove main_seed`
- **AND** a seed labeled `main_seed` exists
- **AND** the user enters any confirmation other than `main_seed`
- **THEN** the CLI exits with a non-zero status
- **AND** does NOT delete the seed row
- **AND** does NOT delete the seed's vault row

### Requirement: Show seed phrase command
The CLI SHALL provide a `seed show [LABEL]` command that prompts for the selected seed's password and temporarily displays the decrypted seed phrase only after successful authentication. The seed phrase SHALL be hidden when the user presses any key or after 30 seconds, whichever happens first. If the label is omitted, the CLI SHALL use the active seed by default unless `--no-defaults` is supplied.

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

#### Scenario: No-defaults forces explicit seed selection for show
- **WHEN** the user runs `ccd-wallet seed show --no-defaults`
- **AND** does not provide a label explicitly
- **THEN** the CLI prompts the user to choose a seed explicitly instead of silently using the active seed
- **AND** the active seed is preselected in the picker when one exists

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
