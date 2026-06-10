## ADDED Requirements

### Requirement: Token compose command opens a persistent interactive composer
The CLI SHALL provide `ccd-wallet token compose <PLAN>` to create or continue a token composition plan at the supplied TOML file path. The composer SHALL use a Reedline-backed command loop for command entry, in-session help, history, completions, and Ctrl-C handling. Missing non-secret operation fields and confirmations SHALL use `cliclack` prompts.

#### Scenario: User starts a new composition plan
- **WHEN** a user runs `ccd-wallet token compose plan.toml`
- **AND** `plan.toml` does not exist
- **THEN** the CLI enters the interactive token composer with an empty versioned plan associated with `plan.toml`
- **AND** the composer accepts `add`, `preview`, `submit`, `help`, `?`, and `exit` commands

#### Scenario: User continues an existing composition plan
- **WHEN** a user runs `ccd-wallet token compose plan.toml`
- **AND** `plan.toml` contains a valid token composition plan
- **THEN** the CLI loads the plan from `plan.toml`
- **AND** the interactive composer continues editing that plan

#### Scenario: User requests help in the composer
- **WHEN** a user enters `help` or `?` in the interactive composer
- **THEN** the composer displays available commands and examples for adding token and lock operations

### Requirement: Compose add records all token and lock MetaUpdate operation families
The interactive composer SHALL let users add every user-facing token and lock MetaUpdate operation family to the plan: token transfer, mint, burn, pause, unpause, allow-list add/remove, deny-list add/remove, admin-roles assign/revoke, metadata update, lock create, lock fund, lock send, lock return, and lock cancel. Add commands SHALL accept inline arguments where supplied and SHALL prompt for missing required non-secret fields in interactive mode.

#### Scenario: User adds an operation with inline arguments
- **WHEN** a user enters `add mint --token CCD --amount 100` in the interactive composer
- **THEN** the composer appends a mint operation for token `CCD` and amount `100` to the plan
- **AND** the plan is saved to the associated TOML path

#### Scenario: User adds an operation through prompts
- **WHEN** a user enters `add lock create` in the interactive composer without all required lock configuration fields
- **THEN** the composer prompts for the missing lock creation fields using `cliclack`
- **AND** appends the completed lock creation operation only after all required fields are collected

#### Scenario: Cancelled add does not mutate the plan
- **WHEN** a user cancels a prompt while adding an operation
- **THEN** the composer does not append a partial operation
- **AND** the saved plan remains unchanged

### Requirement: Composer autosaves plans after successful additions
The composer SHALL persist the full canonical plan to the supplied TOML path after every successful `add` command. The write SHALL be atomic so a failed write does not leave a partially-written plan file.

#### Scenario: Successful add is immediately persisted
- **WHEN** a user successfully adds an operation in the interactive composer
- **THEN** the TOML plan file contains the newly added operation before the composer accepts the next command

#### Scenario: Failed add preserves previous plan
- **WHEN** an add command fails validation or cannot be parsed
- **THEN** the TOML plan file remains byte-for-byte equivalent to the plan before that command where practical
- **AND** no partial operation is saved

### Requirement: Same-plan lock references use explicit at-references
Composition plans SHALL represent references to locks created earlier in the same plan with explicit `@N` lock references, where `N` is the one-based order of lock creation operations in the plan. Interactive input MAY use `@` as shorthand for the most recent preceding lock creation, but the saved plan SHALL canonicalize it to the corresponding explicit `@N` reference.

#### Scenario: Shorthand lock reference is canonicalized
- **WHEN** a plan already contains one lock creation operation
- **AND** the user adds `add lock fund @ --token CCD --amount 100`
- **THEN** the saved plan records the lock reference as `@1`

#### Scenario: Numbered lock reference targets specific created lock
- **WHEN** a plan contains two lock creation operations
- **AND** the user adds a lock operation referencing `@2`
- **THEN** the operation targets the second lock creation operation in the plan

#### Scenario: Invalid same-plan lock reference is rejected
- **WHEN** a user adds a lock operation referencing `@3`
- **AND** the plan has fewer than three lock creation operations
- **THEN** the composer rejects the operation with an actionable error
- **AND** the saved plan remains unchanged

### Requirement: Compose preview lists planned operations
The CLI SHALL provide `ccd-wallet token compose preview <PLAN>` to render the ordered operation list in a token composition plan. Preview SHALL not require sender, network, node, or signing context and SHALL not display a signed transaction payload.

#### Scenario: User previews a composition plan
- **WHEN** a user runs `ccd-wallet token compose preview plan.toml`
- **AND** `plan.toml` contains a valid token composition plan
- **THEN** the CLI prints each planned operation in order with human-readable token, account-reference, amount, lock, and metadata details available from the plan

#### Scenario: Interactive preview shows current saved plan
- **WHEN** a user enters `preview` in the interactive composer
- **THEN** the composer prints the same operation-list preview for the current plan file

### Requirement: Compose submit submits a plan as one MetaUpdate transaction
The CLI SHALL provide `ccd-wallet token compose submit <PLAN> --sender <LABEL> ...` to resolve and submit a token composition plan as one protocol-level token MetaUpdate account transaction. Submit SHALL resolve network and node context, signer account, account references, token amounts, existing lock IDs, and same-plan `@N` lock references before presenting a final confirmation and submitting the transaction.

#### Scenario: User submits a saved plan
- **WHEN** a user runs `ccd-wallet token compose submit plan.toml --sender alice --network testnet`
- **AND** `plan.toml` contains a valid token composition plan
- **THEN** the CLI resolves the plan using Alice as the signing account on testnet
- **AND** submits all planned operations as a single MetaUpdate transaction after confirmation
- **AND** reports the submitted transaction hash

#### Scenario: Submit prompts for missing details in interactive mode
- **WHEN** a user submits a plan with required submission context omitted
- **AND** `--non-interactive` is not supplied
- **THEN** the CLI prompts for the missing non-secret submission details using `cliclack`

#### Scenario: Submit rejects missing details in non-interactive mode
- **WHEN** a user submits a plan with required submission context omitted
- **AND** `--non-interactive` is supplied
- **THEN** the CLI exits with an actionable error instead of prompting

#### Scenario: Interactive submit uses the current plan file
- **WHEN** a user enters `submit --sender alice` in the interactive composer
- **THEN** the composer submits the current saved plan file using the same behavior as `ccd-wallet token compose submit <PLAN> --sender alice`
