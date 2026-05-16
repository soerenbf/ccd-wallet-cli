## MODIFIED Requirements

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
