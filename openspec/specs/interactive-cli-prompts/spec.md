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
Supported prompt-first command flows SHALL avoid unnecessary selector interaction when the effective choice is already obvious. When a selector is skipped because there is exactly one valid option, because the active network resolves an ambiguous explicit account label, or because an explicit local account label uniquely determines the account and network outside the active network, the CLI SHALL still display any silently selected contextual values that are relevant to understanding the command target.

#### Scenario: Single-option selector is skipped
- **WHEN** a supported command would present a selector with exactly one valid option
- **THEN** the CLI selects that option automatically
- **AND** does not render a one-item selector

#### Scenario: Single configured network is shown after skipped selector
- **WHEN** a supported interactive command automatically selects the only configured network
- **THEN** the CLI does not render a network selector
- **AND** displays the selected network in a resolved context header

#### Scenario: Active-network account match skips account selector
- **WHEN** a supported interactive account-consuming command receives an explicit local account label
- **AND** no explicit network was supplied
- **AND** the active network has an eligible matching account for that label
- **THEN** the CLI selects the active-network account
- **AND** does not render a network selector
- **AND** displays the resolved network and account context before performing command-specific work

#### Scenario: Unique explicit account label outside active network skips network selector
- **WHEN** a supported interactive account-consuming command receives an explicit local account label
- **AND** no explicit network was supplied
- **AND** the active network has no eligible matching account for that label
- **AND** that label uniquely identifies an eligible local account on another configured network
- **THEN** the CLI selects that account and its network automatically
- **AND** does not render a network selector
- **AND** displays the resolved network and account context before performing command-specific work

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

### Requirement: Account disambiguation selectors show network and source metadata
When an interactive command must disambiguate between multiple local accounts with the same label and neither an explicit network nor the active-network soft default resolves the choice, the CLI SHALL present an account selector rather than a network selector. Account selector rows SHALL include the local account label, the configured network name or genesis-hash fallback, and ownership/source metadata sufficient to distinguish derived, Ledger-derived, and imported accounts.

Applicable account-consuming command families SHALL use this shared account-selection behavior where their input semantics fit either account-reference resolution or signing-account/sender resolution, rather than keeping command-specific network-first selectors.

#### Scenario: Ambiguous account label selector shows network and key source
- **WHEN** an interactive command receives local account label `alice`
- **AND** no explicit network was supplied
- **AND** no active-network account match resolves the label
- **AND** matching finalized derived accounts exist on more than one configured network
- **THEN** the CLI opens an account selector over the matching accounts
- **AND** each row shows `alice`, the network, and the owning key-source label

#### Scenario: Ambiguous imported account selector shows imported source
- **WHEN** an interactive command receives local account label `genesis`
- **AND** no explicit network was supplied
- **AND** no active-network account match resolves the label
- **AND** matching finalized imported accounts exist on more than one configured network
- **THEN** the CLI opens an account selector over the matching accounts
- **AND** each imported row is marked as imported rather than displaying a seed key source

### Requirement: Resolved account context headers omit account addresses
When the CLI displays a resolved context header for a silently selected or inferred local account, the header SHALL identify the network, local account label, and source metadata needed to understand the command target. The header SHALL NOT include the account address solely as part of context rendering.

#### Scenario: Derived account header shows source metadata without address
- **WHEN** an interactive command silently resolves local account `alice` owned by key source `main-seed`
- **THEN** the resolved context header shows the selected network
- **AND** shows local account label `alice`
- **AND** shows key source `main-seed`
- **AND** does not include the account address

#### Scenario: Imported account header shows imported source without address
- **WHEN** an interactive command silently resolves imported local account `genesis`
- **THEN** the resolved context header shows the selected network
- **AND** shows local account label `genesis`
- **AND** marks the source as imported
- **AND** does not include the account address

### Requirement: Prompt-first flows use shared prepared input resolution
Supported prompt-first command flows that are refactored under the shared command-input model SHALL represent required missing inputs as promptable prepared values before execution. Resolving a promptable prepared value SHALL require the command flow to provide the prompt, selector, or domain-specific resolver that supplies the value in interactive mode.

#### Scenario: Refactored prompt-first flow resolves through prepared input
- **WHEN** a supported refactored command runs interactively without a required non-secret argument
- **THEN** the command represents the missing argument as a promptable prepared value
- **AND** resolves it through a command-specific `cliclack` prompt, selector, or shared domain resolver before execution continues

#### Scenario: Refactored non-interactive flow reports prepared input error
- **WHEN** a supported refactored command runs with `--non-interactive`
- **AND** a required non-secret argument is missing
- **THEN** resolving the promptable prepared value fails before command execution performs the operation
- **AND** the error explains which command-line value must be supplied

### Requirement: Defaultable flows preserve destructive-command safety
Supported refactored flows SHALL distinguish defaultable values from promptable values so that commands can opt into active defaults only where existing command semantics allow them. Destructive remove, delete, and reset flows SHALL NOT silently use active defaults merely because a defaultable helper exists.

#### Scenario: Non-destructive flow may use active default interactively
- **WHEN** a supported refactored non-destructive command has an omitted context value such as network or key source
- **AND** the command's existing semantics allow an active default
- **AND** the command runs interactively without `--no-defaults`
- **THEN** the command may resolve the value from the active default through the shared defaultable input model

#### Scenario: Destructive flow selects or prompts instead of silent active default
- **WHEN** a supported refactored destructive command such as delete, remove, or reset is missing its target
- **THEN** the command SHALL use an explicit selector or prompt according to its existing semantics
- **AND** SHALL NOT silently choose the active target solely through the shared defaultable input model

