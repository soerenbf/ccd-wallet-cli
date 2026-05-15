## ADDED Requirements

### Requirement: Generate random seed phrase
The CLI SHALL support `ccd-wallet seed add <LABEL> --random` to generate a new 24-word English BIP39 seed phrase and store it as a password-protected seed.

#### Scenario: Generate and store random seed
- **WHEN** the user runs `ccd-wallet seed add main_seed --random`
- **AND** enters matching password and confirmation values
- **THEN** the CLI generates a valid 24-word BIP39 seed phrase
- **AND** stores the generated phrase only as encrypted seed payload
- **AND** temporarily reveals the generated phrase using the existing seed reveal flow
- **AND** exits successfully with a confirmation message

#### Scenario: Random seed generation skips phrase prompt
- **WHEN** the user runs `ccd-wallet seed add main_seed --random`
- **THEN** the CLI does NOT prompt the user to enter a seed phrase
- **AND** prompts only for password and password confirmation before storing

#### Scenario: Duplicate label rejected before random generation
- **WHEN** the user runs `ccd-wallet seed add main_seed --random`
- **AND** a seed labeled `main_seed` already exists
- **THEN** the CLI exits with a non-zero status
- **AND** does NOT generate or store a new seed phrase

### Requirement: Remove seed command
The CLI SHALL provide a `seed remove <LABEL>` command that removes a configured seed after explicit confirmation.

#### Scenario: Remove existing seed after confirmation
- **WHEN** the user runs `ccd-wallet seed remove main_seed`
- **AND** a seed labeled `main_seed` exists
- **AND** the user confirms by typing `main_seed`
- **THEN** the CLI deletes the seed row
- **AND** SQLite cascades deletion to the seed's vault row
- **AND** the CLI exits successfully with a confirmation message

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
