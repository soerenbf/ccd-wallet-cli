# Spec Delta

## ADDED Requirements

### Requirement: Version-1 plans preserve legacy release spellings
The composer SHALL read existing version-1 plans containing legacy lock-return operation tags and `return` grant capabilities as release operations and capabilities. New saves SHALL use canonical release spellings without changing the plan version, operation order, amounts, account references, or lock references. Loading or previewing SHALL NOT rewrite the file.

#### Scenario: Legacy plan can be previewed and submitted
- **WHEN** a version-1 plan contains a legacy lock-return operation and a lock grant with the `return` capability
- **THEN** preview describes them using release terminology
- **AND** submission constructs the corresponding release operation and grant
- **AND** loading and previewing leave the file unchanged

#### Scenario: Saving canonicalizes legacy plan spellings
- **WHEN** the composer successfully saves a loaded legacy version-1 plan
- **THEN** the saved operation and grant capability use release spellings
- **AND** the plan remains version 1 with its existing semantic values and operation order preserved

## RENAMED Requirements

- FROM: `### Requirement: Compose records all token and lock MetaUpdate operation families`
- TO: `### Requirement: Compose records all token and lock TokenUpdate operation families`

- FROM: `### Requirement: Compose submit submits a plan as one MetaUpdate transaction`
- TO: `### Requirement: Compose submit submits a plan as one TokenUpdate transaction`

## MODIFIED Requirements

### Requirement: Compose records all token and lock TokenUpdate operation families
The interactive composer SHALL let users add every user-facing token and lock TokenUpdate operation family to the plan through top-level operation commands: token transfer, mint, burn, pause, unpause, allow-list add/remove, deny-list add/remove, admin-roles assign/revoke, metadata update, lock create, lock fund, lock send, lock release, and lock cancel. Operation commands SHALL accept inline arguments where supplied and SHALL prompt for missing required non-secret fields in interactive mode. Before saving an added operation, the composer SHALL bind the plan to a network genesis hash, resolve any local account labels in the operation to raw account addresses, preserve the special `@sender` account reference where supplied, validate referenced tokens against the bound network, and validate token amounts against each token's configured decimals. For lock creation, the composer SHALL support either repeated `--recipient` values for limited-recipient locks or `--any-recipient` for any-recipient locks. When interactive lock creation omits both forms, the composer SHALL prompt for recipient mode before collecting any recipient-account inputs. For lock send, limited-recipient locks SHALL continue to validate recipients against the configured recipient set and SHALL present a selector when the recipient is omitted, while any-recipient locks SHALL accept any resolved recipient and SHALL prompt with free-form account input when the recipient is omitted.

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

#### Scenario: User adds lock send without recipient for limited-recipient lock
- **WHEN** a user adds a lock send operation without a recipient
- **AND** the lock reference resolves to an in-plan lock creation or existing on-chain limited-recipient lock
- **THEN** the composer presents a recipient selector populated from the lock's configured recipients
- **AND** saves the selected recipient as a raw account address or `@sender` where applicable

#### Scenario: User adds lock send without recipient for any-recipient lock
- **WHEN** a user adds a lock send operation without a recipient
- **AND** the lock reference resolves to an in-plan lock creation or existing on-chain any-recipient lock
- **THEN** the composer prompts for a free-form recipient account reference instead of presenting a fixed recipient selector
- **AND** saves the resolved recipient as a raw account address or `@sender` where applicable

#### Scenario: User adds lock send with invalid recipient for limited-recipient lock
- **WHEN** a user adds a lock send operation with an explicit recipient that is not configured for the referenced limited-recipient lock
- **THEN** the composer rejects the operation before saving

#### Scenario: User adds lock send with explicit recipient for any-recipient lock
- **WHEN** a user adds a lock send operation with an explicit recipient
- **AND** the referenced lock resolves to an any-recipient lock
- **THEN** the composer accepts that recipient without checking it against a finite configured recipient list
- **AND** saves the operation if the remaining validation succeeds

#### Scenario: User adds an any-recipient lock create through inline arguments
- **WHEN** a user enters `lock create --any-recipient --expiry 1d --grant alice:fund --token CCD` in the interactive composer
- **THEN** the composer appends an any-recipient lock-creation operation to the plan
- **AND** the plan is saved to the associated TOML path

#### Scenario: User adds an any-recipient lock create through prompts
- **WHEN** a user enters `lock create` in the interactive composer without recipient arguments
- **THEN** the composer prompts for recipient mode before collecting recipient details
- **AND** choosing any-recipient skips recipient-account collection
- **AND** the composer prompts for the remaining missing lock creation fields using `cliclack`

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

### Requirement: Composer supports constrained lock creation metadata
The token compose REPL SHALL support optional `--name` and `--description` inputs for `lock create`, including context-appropriate completion and help. The composer SHALL prompt with empty-allowed input for either omitted field, persist supplied values in the lock-creation plan operation, display them in plan previews, and encode them as the corresponding protocol lock metadata fields during submission. The composer SHALL NOT expose arbitrary additional lock metadata fields.

#### Scenario: User adds lock metadata inline
- **WHEN** a user enters `lock create --name "Treasury vesting" --description "Quarterly contributor allocation"` with the remaining required lock arguments
- **THEN** the composer saves both metadata values on the lock-creation operation
- **AND** completion and help identify `--name` and `--description` as supported lock-create options

#### Scenario: User supplies one lock metadata field inline
- **WHEN** a user supplies only `--name` or only `--description` for a compose lock creation
- **THEN** the composer preserves the supplied value
- **AND** prompts with empty-allowed input for the omitted metadata field

#### Scenario: User supplies lock metadata through prompts
- **WHEN** a user adds a lock creation without inline metadata options
- **THEN** the composer prompts for an optional name and description
- **AND** non-empty responses are saved on the lock-creation operation
- **AND** empty responses are recorded as absent

#### Scenario: Compose preview shows lock metadata
- **WHEN** a composition plan contains a lock creation with a name or description
- **THEN** CLI and interactive plan previews display the supplied metadata alongside the lock-creation summary

#### Scenario: Compose submission encodes lock metadata
- **WHEN** a user submits a composition plan containing a lock name or description
- **THEN** the corresponding lock-create TokenUpdate operation contains metadata with those recognized fields
- **AND** arbitrary additional metadata fields are not encoded

#### Scenario: Metadata-free version-1 plan remains compatible
- **WHEN** the composer opens, previews, or submits an existing version-1 plan whose lock creation has no name or description
- **THEN** the plan remains valid without migration
- **AND** its lock-create operation is submitted without a metadata payload

### Requirement: Compose submit submits a plan as one TokenUpdate transaction
The CLI SHALL provide `ccd-wallet token compose submit <PLAN> --sender <LABEL> ...` to resolve and submit a token composition plan as one unscoped TokenUpdate account transaction. Submit SHALL resolve network and node context, signer account, token amounts, existing lock IDs, and same-plan `@N` lock references before presenting a final confirmation and submitting the transaction. When neither `--network` nor `--node` is supplied, submit SHALL infer the network from the plan's stored genesis hash instead of prompting for network selection. In interactive mode, submit SHALL always require explicit sender selection when no sender is supplied, even if only one account is available. Submit SHALL reject plans whose stored network genesis hash does not match the selected network.

#### Scenario: User submits a saved plan
- **WHEN** a user runs `ccd-wallet token compose submit plan.toml --sender alice --network testnet`
- **AND** `plan.toml` contains a valid token composition plan for testnet's genesis hash
- **THEN** the CLI resolves the plan using Alice as the signing account on testnet
- **AND** submits all planned operations as a single unscoped TokenUpdate transaction after confirmation
- **AND** reports the submitted transaction hash

#### Scenario: User submits without network flag
- **WHEN** a user runs `ccd-wallet token compose submit plan.toml`
- **AND** `plan.toml` contains a valid token composition plan with a stored network genesis hash
- **THEN** the CLI infers a configured network matching that genesis hash
- **AND** does not prompt for network selection

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
