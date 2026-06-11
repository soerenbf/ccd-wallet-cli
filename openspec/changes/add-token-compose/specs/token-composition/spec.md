## ADDED Requirements

### Requirement: Token compose command opens a persistent interactive composer
The CLI SHALL provide `ccd-wallet token compose <PLAN>` to create or continue a token composition plan at the supplied TOML file path. When creating a new plan or opening a plan without a stored network genesis hash, the composer SHALL prompt for explicit network selection before accepting composer commands and SHALL save the selected network genesis hash into the plan. The composer SHALL use a Reedline-backed command loop for command entry, in-session help, history, completions, and Ctrl-C handling. Before executing each non-empty composer command, the composer SHALL reload the plan from disk so manual edits are reflected. Missing non-secret operation fields and confirmations SHALL use `cliclack` prompts.

#### Scenario: User starts a new composition plan
- **WHEN** a user runs `ccd-wallet token compose plan.toml`
- **AND** `plan.toml` does not exist
- **THEN** the CLI prompts the user to select the target network
- **AND** the CLI saves the selected network genesis hash in `plan.toml`
- **AND** the CLI enters the interactive token composer with an empty versioned plan associated with `plan.toml`
- **AND** the composer accepts operation commands such as `mint`, `lock create`, and `lock fund`, plus `preview`, `submit`, `help`, `?`, and `exit` commands

#### Scenario: User continues an existing composition plan
- **WHEN** a user runs `ccd-wallet token compose plan.toml`
- **AND** `plan.toml` contains a valid token composition plan
- **THEN** the CLI loads the plan from `plan.toml`
- **AND** the interactive composer continues editing that plan

#### Scenario: User manually edits plan during composer session
- **WHEN** a user changes the plan file on disk while the interactive composer is open
- **AND** the user enters another non-empty composer command
- **THEN** the composer reloads the plan from disk before executing that command

#### Scenario: User requests help in the composer
- **WHEN** a user enters `help` or `?` in the interactive composer
- **THEN** the composer displays available commands and examples for adding token and lock operations

### Requirement: Compose records all token and lock MetaUpdate operation families
The interactive composer SHALL let users add every user-facing token and lock MetaUpdate operation family to the plan through top-level operation commands: token transfer, mint, burn, pause, unpause, allow-list add/remove, deny-list add/remove, admin-roles assign/revoke, metadata update, lock create, lock fund, lock send, lock return, and lock cancel. Operation commands SHALL accept inline arguments where supplied and SHALL prompt for missing required non-secret fields in interactive mode. Before saving an added operation, the composer SHALL bind the plan to a network genesis hash, resolve any local account labels in the operation to raw account addresses, preserve the special `@sender` account reference where supplied, validate referenced tokens against the bound network, and validate token amounts against each token's configured decimals.

#### Scenario: User adds an operation with inline arguments
- **WHEN** a user enters `mint --token CCD --amount 100` in the interactive composer
- **THEN** the composer appends a mint operation for token `CCD` and amount `100` to the plan
- **AND** the plan is saved to the associated TOML path

#### Scenario: User adds operation with local account labels
- **WHEN** a user adds an operation whose account inputs use finalized local account labels
- **THEN** the composer resolves those labels using the plan's network genesis hash before saving
- **AND** the saved plan contains raw account addresses for those inputs rather than local labels

#### Scenario: User adds operation with sender reference
- **WHEN** a user adds an operation whose account input is `@sender`
- **THEN** the composer preserves `@sender` in the saved plan
- **AND** submit resolves `@sender` to the selected sender account before constructing the transaction

#### Scenario: User adds lock fund with unknown token
- **WHEN** a user adds a lock funding operation with a token identifier that does not exist on the plan network
- **THEN** the composer rejects the operation before saving
- **AND** the saved plan remains unchanged

#### Scenario: User adds lock fund with invalid amount decimals
- **WHEN** a user adds a lock funding operation with an amount that has more fractional digits than the token supports
- **THEN** the composer rejects the operation before saving
- **AND** the saved plan remains unchanged

#### Scenario: User adds lock fund for local lock reference
- **WHEN** a user adds a lock funding operation that references a lock created earlier in the plan
- **AND** the token identifier is omitted
- **THEN** the composer presents a token selector populated from the referenced lock creation operation's configured tokens
- **AND** validates that the selected token is configured by the referenced lock creation operation before saving

#### Scenario: User adds lock fund for existing lock id
- **WHEN** a user adds a lock funding operation that references an existing on-chain lock id
- **AND** the token identifier is omitted
- **THEN** the composer queries the lock on the plan network
- **AND** presents a token selector populated from the lock's configured tokens
- **AND** validates that the selected token is configured by that lock before saving

#### Scenario: User adds lock send without recipient
- **WHEN** a user adds a lock send operation without a recipient
- **AND** the lock reference resolves to an in-plan lock creation or existing on-chain lock
- **THEN** the composer presents a recipient selector populated from the lock's configured recipients
- **AND** saves the selected recipient as a raw account address or `@sender` where applicable

#### Scenario: User adds lock send with invalid recipient
- **WHEN** a user adds a lock send operation with an explicit recipient that is not configured for the referenced lock
- **THEN** the composer rejects the operation before saving

#### Scenario: User adds an operation through prompts
- **WHEN** a user enters `lock create` in the interactive composer without all required lock configuration fields
- **THEN** the composer prompts for the missing lock creation fields using `cliclack`
- **AND** lock grants are composed by prompting for a grant account and selecting one or more known lock capabilities
- **AND** the composer prompts whether to keep the lock alive, defaulting to No when `--keep-alive` is omitted
- **AND** appends the completed lock creation operation only after all required fields are collected

#### Scenario: Cancelled add does not mutate the plan
- **WHEN** a user cancels a prompt while adding an operation
- **THEN** the composer does not append a partial operation
- **AND** the saved plan remains unchanged

### Requirement: Composer autosaves plans after successful additions
The composer SHALL persist the full canonical plan to the supplied TOML path after every successful `add` command. The write SHALL be atomic so a failed write does not leave a partially-written plan file. Saved plans SHALL include the target network genesis hash. Saved lock grants SHALL use structured account and capability fields rather than opaque comma-delimited grant strings.

#### Scenario: Successful add is immediately persisted
- **WHEN** a user successfully adds an operation in the interactive composer
- **THEN** the TOML plan file contains the newly added operation before the composer accepts the next command

#### Scenario: Lock grant is saved structurally
- **WHEN** a user successfully adds a lock creation operation with a grant for local account label `alice` and capabilities `fund` and `send`
- **THEN** the saved TOML records the resolved account address and capabilities `fund` and `send` as structured fields
- **AND** the saved TOML does not split grant capability commas into separate grants
- **AND** the saved TOML does not persist the local label `alice`

#### Scenario: Failed add preserves previous plan
- **WHEN** an add command fails validation or cannot be parsed
- **THEN** the TOML plan file remains byte-for-byte equivalent to the plan before that command where practical
- **AND** no partial operation is saved

### Requirement: Same-plan lock references use explicit at-references
Composition plans SHALL represent references to locks created earlier in the same plan with explicit `@N` lock references, where `N` is the one-based order of lock creation operations in the plan. Interactive input MAY use `@` as shorthand for the most recent preceding lock creation, but the saved plan SHALL canonicalize it to the corresponding explicit `@N` reference.

#### Scenario: Shorthand lock reference is canonicalized
- **WHEN** a plan already contains one lock creation operation
- **AND** the user adds `lock fund @ --token CCD --amount 100`
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

### Requirement: Composer validates lock grant capabilities before saving
The composer SHALL validate lock grant capability names before saving a lock creation operation. Unknown capabilities SHALL be rejected with an actionable error that lists the supported capability names.

#### Scenario: Inline grant with unknown capability is rejected
- **WHEN** a user enters `lock create --grant alice:fund,nonsense` in the interactive composer
- **THEN** the composer rejects the operation before saving it
- **AND** the error identifies `nonsense` as an unknown lock capability

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
The CLI SHALL provide `ccd-wallet token compose submit <PLAN> --sender <LABEL> ...` to resolve and submit a token composition plan as one protocol-level token MetaUpdate account transaction. Submit SHALL resolve network and node context, signer account, token amounts, existing lock IDs, and same-plan `@N` lock references before presenting a final confirmation and submitting the transaction. In interactive mode, submit SHALL always require explicit sender selection when no sender is supplied, even if only one account is available. Submit SHALL reject plans whose stored network genesis hash does not match the selected network.

#### Scenario: User submits a saved plan
- **WHEN** a user runs `ccd-wallet token compose submit plan.toml --sender alice --network testnet`
- **AND** `plan.toml` contains a valid token composition plan for testnet's genesis hash
- **THEN** the CLI resolves the plan using Alice as the signing account on testnet
- **AND** submits all planned operations as a single MetaUpdate transaction after confirmation
- **AND** reports the submitted transaction hash

#### Scenario: Submit rejects wrong network
- **WHEN** a user submits a composition plan with a stored network genesis hash
- **AND** the selected network has a different genesis hash
- **THEN** the CLI rejects the submission before signing

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
- **AND** the composer exits after the submit command completes successfully
