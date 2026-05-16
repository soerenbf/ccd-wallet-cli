## ADDED Requirements

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

## MODIFIED Requirements

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
