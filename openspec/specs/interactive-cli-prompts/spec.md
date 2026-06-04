# interactive-cli-prompts Specification

## Purpose
TBD - created by archiving change improve-identity-issuance-context-ux. Update Purpose after archive.
## Requirements
### Requirement: User-facing command flows support prompt fallback
User-facing command flows that require non-secret input SHALL request missing values through `cliclack` prompts when running in interactive mode.

#### Scenario: Missing value is prompted in interactive mode
- **WHEN** a user runs a supported command without a required non-secret argument
- **AND** `--non-interactive` is not supplied
- **THEN** the CLI requests the missing value through a `cliclack` prompt
- **AND** continues the command using the entered value

#### Scenario: Prompt order is deterministic
- **WHEN** multiple values are missing for a supported command
- **THEN** the CLI prompts for them in a stable order defined by that command's flow

### Requirement: Non-interactive mode disables prompt fallback
Supported prompt-first command flows SHALL accept `--non-interactive` to disable prompt fallback.

#### Scenario: Missing value in non-interactive mode errors
- **WHEN** a user runs a supported command without a required non-secret argument
- **AND** `--non-interactive` is supplied
- **THEN** the CLI does not prompt for the missing value
- **AND** exits with an actionable error explaining what must be provided on the command line

### Requirement: Prompt framework is consistent within supported flows
Supported prompt-first command flows SHALL use `cliclack` for interactive input collection.

#### Scenario: Flow uses cliclack prompts
- **WHEN** a supported command requests user input interactively
- **THEN** the input prompt is implemented with `cliclack`
- **AND** the flow does not mix in a different prompt framework for equivalent user-facing inputs

### Requirement: Selectors minimize redundant interaction
Supported prompt-first command flows SHALL avoid unnecessary selector interaction when the effective choice is already obvious.

#### Scenario: Single-option selector is skipped
- **WHEN** a supported command would present a selector with exactly one valid option
- **THEN** the CLI selects that option automatically
- **AND** does not render a one-item selector

#### Scenario: Existing-entity choice uses selector instead of free-text input
- **WHEN** a supported command asks the user to choose from already configured seeds or networks
- **THEN** the CLI uses a `cliclack` selector instead of a free-text label prompt

#### Scenario: Seed delete chooses an existing seed through a selector
- **WHEN** the user runs `ccd-wallet seed delete` without a label
- **THEN** the CLI renders a `cliclack` selector over configured seeds
- **AND** uses the selected seed label for the destructive flow

#### Scenario: Network reset selector renders partition rows with hashes and aliases
- **WHEN** the user runs `ccd-wallet network reset` without a target
- **AND** the wallet has stored network data for genesis hash `abc`
- **AND** configured aliases `testnet` and `staging-testnet` reference `abc`
- **THEN** the CLI renders a partition-oriented row that shows the genesis hash and matching aliases

#### Scenario: Network reset selector can include orphaned hashes
- **WHEN** the user runs `ccd-wallet network reset` without a target
- **AND** the wallet has stored network data for a genesis hash not present in config
- **THEN** the CLI renders a selectable orphaned-hash target labeled as orphaned in addition to configured partitions

### Requirement: Recovery provider choice uses cliclack multiselect
Supported recovery flows SHALL use a `cliclack` multiselect when the user can narrow recovery to a subset of already-discovered identity providers.

#### Scenario: Recovery provider selection uses multiselect
- **WHEN** interactive `seed sync` offers the user a choice among multiple providers
- **AND** no explicit `--provider` filters were supplied
- **THEN** the CLI renders that choice with a `cliclack` multiselect
- **AND** uses the selected provider subset for recovery

#### Scenario: Explicit provider arguments suppress multiselect
- **WHEN** interactive `seed sync` is run with one or more explicit `--provider` arguments
- **THEN** the CLI does not render the provider multiselect
- **AND** uses the explicitly supplied provider scope for recovery

### Requirement: Account-reference prompts use cliclack autocomplete with raw-address fallback
Supported prompt-first command flows that resolve non-sender account references SHALL use a `cliclack` text input prompt with autocomplete suggestions for finalized local accounts while still accepting pasted raw account addresses.

#### Scenario: Prompted token recipient offers local-account autocomplete suggestions
- **WHEN** an interactive token command prompts for a missing recipient, source, or target account reference
- **THEN** the CLI uses a `cliclack` input prompt with autocomplete suggestions sourced from finalized local accounts on the resolved network
- **AND** the prompt still accepts pasted raw account addresses

#### Scenario: Prompt suggestions show account ownership context
- **WHEN** an interactive account-reference prompt renders suggestions for local accounts
- **THEN** each derived account suggestion shows its seed ownership in bracketed form
- **AND** each imported account suggestion shows `[imported]` before the account label

### Requirement: Destructive flows use cliclack warnings before confirmation
Supported destructive deletion and reset flows SHALL warn the user through `cliclack` before accepting typed confirmation.

#### Scenario: Seed delete warns about owned data removal
- **WHEN** the user runs `ccd-wallet seed delete main_seed`
- **AND** the seed owns stored identities or accounts
- **THEN** the CLI emits a `cliclack` warning explaining that those identities and accounts will also be removed before confirmation is requested

#### Scenario: Network delete warns when data will become orphaned
- **WHEN** the user runs `ccd-wallet network delete testnet`
- **THEN** the CLI emits a `cliclack` warning that explains the action removes config aliases only
- **AND** warns when the deletion will leave identities/accounts for that network hash orphaned before confirmation is requested

### Requirement: Long-running recovery flows show cliclack-based aggregate progress
Supported long-running recovery flows SHALL present progress using cliclack primitives and known outer phases plus live aggregate counters instead of a single synthetic percentage over unknown totals.

#### Scenario: Recovery shows determinate provider progress and aggregate worker state
- **WHEN** interactive `seed sync` is running across multiple selected providers
- **THEN** the CLI shows determinate progress over providers completed versus selected
- **AND** shows aggregate worker-state and discovery counters for the running recovery

#### Scenario: Recovery progress remains truthful when totals are unknown
- **WHEN** the CLI cannot know in advance how many identities or accounts are recoverable
- **THEN** the progress presentation avoids claiming a total identity or account count
- **AND** instead reports aggregate probe position or discovery counts known so far

#### Scenario: Parallel recovery progress stays visually compact
- **WHEN** multiple recovery tasks are running concurrently
- **THEN** the CLI keeps the progress display compact and consistent with other cliclack-based flows
- **AND** does not render an unbounded list of independent progress widgets

