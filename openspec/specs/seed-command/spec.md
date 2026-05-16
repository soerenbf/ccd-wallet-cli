# seed-command Specification

## Purpose
TBD - created by archiving change add-seed-command. Update Purpose after archive.
## Requirements
### Requirement: Add seed command
The CLI SHALL provide a `seed add` command that adds a password-protected seed phrase to the encrypted seed store. Seed labels SHALL be non-empty and contain only ASCII alphanumeric characters, dash (`-`), and underscore (`_`). In interactive mode, the label SHALL be prompted if omitted; in `--non-interactive` mode, omitting the label SHALL be an error. The command SHALL also support `--restore <NETWORK>` to run seed recovery immediately after successful seed storage.

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

#### Scenario: Add and immediately restore on chosen network
- **WHEN** the user runs `ccd-wallet seed add main_seed --restore testnet`
- **AND** enters a valid seed phrase
- **AND** enters matching password and confirmation values
- **AND** the network `testnet` exists
- **THEN** the CLI stores the seed successfully
- **AND** immediately starts recovery for seed `main_seed` on network `testnet`
- **AND** exits with a recovery summary after the restore flow completes

#### Scenario: Restore network must exist before add-and-restore proceeds
- **WHEN** the user runs `ccd-wallet seed add main_seed --restore missingnet`
- **AND** no configured network matches `missingnet`
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating that the network is not configured
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

### Requirement: Delete seed command
The CLI SHALL provide a `seed delete` command that removes a configured seed after explicit confirmation. The command SHALL accept an explicit label or interactive selector resolution when the label is omitted. In `--non-interactive` mode, omitting the label SHALL be an error. The command SHALL NOT infer its target from the active seed.

#### Scenario: Delete existing seed after confirmation
- **WHEN** the user runs `ccd-wallet seed delete main_seed`
- **AND** a seed labeled `main_seed` exists
- **AND** the user confirms by typing `main_seed`
- **THEN** the CLI deletes the seed row
- **AND** deletes seed-owned identities and accounts by existing SQLite cascade semantics
- **AND** exits successfully with a confirmation message

#### Scenario: Missing label opens a selector for seed delete
- **WHEN** the user runs `ccd-wallet seed delete`
- **AND** `--non-interactive` is not supplied
- **THEN** the CLI opens a selector over configured seeds
- **AND** uses the selected label for confirmation and deletion

#### Scenario: Delete rejects missing label in non-interactive mode
- **WHEN** the user runs `ccd-wallet seed delete --non-interactive`
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating that the seed label must be provided

#### Scenario: Delete does not use active seed implicitly
- **WHEN** the user runs `ccd-wallet seed delete`
- **AND** an active seed is configured
- **AND** `--non-interactive` is supplied
- **THEN** the CLI does not use the active seed as the deletion target
- **AND** exits with an actionable missing-target error

#### Scenario: Delete rejected when confirmation does not match
- **WHEN** the user runs `ccd-wallet seed delete main_seed`
- **AND** a seed labeled `main_seed` exists
- **AND** the user enters any confirmation other than `main_seed`
- **THEN** the CLI exits with a non-zero status
- **AND** does NOT delete the seed row

### Requirement: Sync seed recovery state
The CLI SHALL provide a `seed sync` command that discovers recoverable identities and wallet-managed accounts for a resolved seed and network scope. The command SHALL resolve seed scope from an explicit label first and otherwise from the active seed when defaults are allowed. The command SHALL resolve network scope from an explicit `--network <LABEL>` first and otherwise from the active network when defaults are allowed. In interactive mode, any still-missing seed or network scope SHALL be prompted before recovery starts. In non-interactive mode, unresolved required scope SHALL be an error.

#### Scenario: Sync explicit seed and network
- **WHEN** the user runs `ccd-wallet seed sync main_seed --network testnet`
- **AND** the seed and network exist
- **THEN** the CLI runs recovery for that seed on that network
- **AND** exits with a recovery summary

#### Scenario: Sync uses active defaults when allowed
- **WHEN** the user runs `ccd-wallet seed sync`
- **AND** an active seed and active network are configured
- **THEN** the CLI uses those active values as the recovery scope
- **AND** shows the resolved context before recovery begins

#### Scenario: Interactive sync prompts for missing scope
- **WHEN** the user runs `ccd-wallet seed sync --no-defaults`
- **AND** `--non-interactive` is not supplied
- **THEN** the CLI prompts for the seed and network scope before recovery begins

#### Scenario: Non-interactive sync errors on unresolved scope
- **WHEN** the user runs `ccd-wallet seed sync --non-interactive`
- **AND** the effective seed or network scope cannot be resolved from explicit arguments or active defaults
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error explaining which scope must be provided

### Requirement: Sync can narrow or default provider selection
The `seed sync` command SHALL recover across all available identity providers for the chosen network by default. The command SHALL also support repeated `--provider <VALUE>` arguments to select providers explicitly. `--provider all` SHALL mean all recovery-capable providers for the resolved network. Specific provider values supplied through repeated `--provider` arguments SHALL narrow recovery to that subset. `all` SHALL be mutually exclusive with specific provider values. In interactive mode, when no explicit provider filters are supplied and more than one provider is available, the CLI SHALL allow the user to narrow the scan through a provider multiselect prompt. If exactly one provider is available, the CLI SHALL skip the selector and use that provider automatically.

#### Scenario: Interactive sync selects subset of providers
- **WHEN** the user runs `ccd-wallet seed sync`
- **AND** multiple providers are available for the resolved network
- **THEN** the CLI offers a multiselect over those providers before recovery starts
- **AND** recovery uses only the selected providers

#### Scenario: Single provider skips selector
- **WHEN** the resolved network exposes exactly one recovery-capable provider
- **THEN** the CLI selects that provider automatically
- **AND** does not render a one-item provider selector

#### Scenario: Non-interactive sync uses all providers in scope
- **WHEN** the user runs `ccd-wallet seed sync --non-interactive`
- **AND** seed and network scope are resolved
- **AND** no explicit provider filters are supplied
- **THEN** the CLI does not prompt for provider selection
- **AND** scans all recovery-capable providers for that network

#### Scenario: Explicit provider subset mirrors multiselect behavior
- **WHEN** the user runs `ccd-wallet seed sync --provider 2 --provider 7`
- **AND** both providers exist for the resolved network
- **THEN** the CLI skips the interactive provider selector
- **AND** recovers using only providers `2` and `7`

#### Scenario: Explicit all-provider selection is accepted
- **WHEN** the user runs `ccd-wallet seed sync --provider all`
- **AND** seed and network scope are resolved
- **THEN** the CLI skips the interactive provider selector
- **AND** scans all recovery-capable providers for that network

#### Scenario: All-provider selection cannot be mixed with specific providers
- **WHEN** the user runs `ccd-wallet seed sync --provider all --provider 2`
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error explaining that `all` cannot be combined with specific providers

#### Scenario: Unknown explicit provider is rejected
- **WHEN** the user runs `ccd-wallet seed sync --provider 999`
- **AND** no recovery-capable provider `999` exists for the resolved network
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating that the provider is unavailable in the chosen network

