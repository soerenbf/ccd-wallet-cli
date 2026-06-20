## MODIFIED Requirements

### Requirement: Compose records all token and lock MetaUpdate operation families
The interactive composer SHALL let users add every user-facing token and lock MetaUpdate operation family to the plan through top-level operation commands: token transfer, mint, burn, pause, unpause, allow-list add/remove, deny-list add/remove, admin-roles assign/revoke, metadata update, lock create, lock fund, lock send, lock return, and lock cancel. Operation commands SHALL accept inline arguments where supplied and SHALL prompt for missing required non-secret fields in interactive mode. Before saving an added operation, the composer SHALL bind the plan to a network genesis hash, resolve any local account labels in the operation to raw account addresses, preserve the special `@sender` account reference where supplied, validate referenced tokens against the bound network, and validate token amounts against each token's configured decimals. For lock creation, the composer SHALL support either repeated `--recipient` values for limited-recipient locks or `--any-recipient` for any-recipient locks. When interactive lock creation omits both forms, the composer SHALL prompt for recipient mode before collecting any recipient-account inputs. For lock send, limited-recipient locks SHALL continue to validate recipients against the configured recipient set and SHALL present a selector when the recipient is omitted, while any-recipient locks SHALL accept any resolved recipient and SHALL prompt with free-form account input when the recipient is omitted.

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

### Requirement: Composer autosaves plans after successful additions
The composer SHALL persist the full canonical plan to the supplied TOML path after every successful `add` command. The write SHALL be atomic so a failed write does not leave a partially-written plan file. Saved plans SHALL include the target network genesis hash. Saved lock grants SHALL use structured account and capability fields rather than opaque comma-delimited grant strings. Saved lock-creation operations SHALL serialize limited-recipient locks with array-valued `recipients` and any-recipient locks with `recipients = "any"`.

#### Scenario: Successful add is immediately persisted
- **WHEN** a user successfully adds an operation in the interactive composer
- **THEN** the TOML plan file contains the newly added operation before the composer accepts the next command

#### Scenario: Lock grant is saved structurally
- **WHEN** a user successfully adds a lock creation operation with a grant for local account label `alice` and capabilities `fund` and `send`
- **THEN** the saved TOML records the resolved account address and capabilities `fund` and `send` as structured fields
- **AND** the saved TOML does not split grant capability commas into separate grants
- **AND** the saved TOML does not persist the local label `alice`

#### Scenario: Limited-recipient lock create is saved with recipient array
- **WHEN** a user successfully adds a lock creation operation with explicit recipient accounts
- **THEN** the saved TOML records those recipients as an array under `recipients`
- **AND** the saved TOML does not serialize that lock as `recipients = "any"`

#### Scenario: Any-recipient lock create is saved with recipient sentinel
- **WHEN** a user successfully adds an any-recipient lock creation operation
- **THEN** the saved TOML records that operation as `recipients = "any"`
- **AND** the saved TOML does not require a separate recipient-mode field

#### Scenario: Failed add preserves previous plan
- **WHEN** an add command fails validation or cannot be parsed
- **THEN** the TOML plan file remains byte-for-byte equivalent to the plan before that command where practical
- **AND** no partial operation is saved

### Requirement: Compose preview lists planned operations
The CLI SHALL provide `ccd-wallet token compose preview <PLAN>` to render the ordered operation list in a token composition plan. Preview SHALL not require sender, network, node, or signing context and SHALL not display a signed transaction payload. Preview output for lock-creation operations SHALL distinguish between limited-recipient locks and any-recipient locks.

#### Scenario: User previews a composition plan
- **WHEN** a user runs `ccd-wallet token compose preview plan.toml`
- **AND** `plan.toml` contains a valid token composition plan
- **THEN** the CLI prints each planned operation in order with human-readable token, account-reference, amount, lock, and metadata details available from the plan

#### Scenario: Interactive preview shows current saved plan
- **WHEN** a user enters `preview` in the interactive composer
- **THEN** the composer prints the same operation-list preview for the current plan file

#### Scenario: Preview shows any-recipient lock create
- **WHEN** a user previews a composition plan containing an any-recipient lock creation operation
- **THEN** the preview describes that lock as targeting `any eligible account`
- **AND** it does not render a synthetic recipient list for that operation
