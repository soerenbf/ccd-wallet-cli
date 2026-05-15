## ADDED Requirements

### Requirement: Add seed command
The CLI SHALL provide a `seed add <LABEL>` command that adds a password-protected seed phrase to the encrypted seed store. Seed labels SHALL be non-empty and contain only ASCII alphanumeric characters, dash (`-`), and underscore (`_`).

#### Scenario: Add a valid seed phrase
- **WHEN** the user runs `ccd-wallet seed add main_seed`
- **AND** enters a valid seed phrase
- **AND** enters matching password and confirmation values
- **THEN** the CLI stores the seed label `main_seed` in plaintext
- **AND** stores the normalized seed phrase only as encrypted seed payload
- **AND** exits successfully with a confirmation message

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
The CLI SHALL NOT accept seed phrases or seed passwords through command-line arguments. The seed phrase, password, and password confirmation SHALL be read through hidden interactive prompts.

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

### Requirement: Seed phrase validation
The CLI SHALL normalize and validate the entered seed phrase before storage. Invalid mnemonic phrases SHALL be rejected before any DB write occurs.

#### Scenario: Valid mnemonic is accepted
- **WHEN** the user enters a valid BIP39 mnemonic phrase for `ccd-wallet seed add main_seed`
- **THEN** the CLI normalizes phrase whitespace
- **AND** encrypts and stores the normalized phrase

#### Scenario: Invalid mnemonic is rejected
- **WHEN** the user enters an invalid seed phrase for `ccd-wallet seed add main_seed`
- **THEN** the CLI exits with a non-zero status
- **AND** prints an error indicating the seed phrase is invalid
- **AND** does NOT write a seed row

#### Scenario: Whitespace is normalized before validation
- **WHEN** the user enters a valid mnemonic with leading, trailing, or repeated internal whitespace
- **THEN** the CLI validates the normalized phrase
- **AND** stores the normalized phrase as the encrypted seed payload
