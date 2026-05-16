## ADDED Requirements

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
