# Spec Delta

## ADDED Requirements

### Requirement: Release is the canonical lock command and capability
The CLI and composer SHALL expose `lock release` and the `release` grant capability in parsing, prompts, help, completion, confirmations, and output. Newly entered `lock return` commands and `return` grant capabilities SHALL be rejected rather than submitted. Legacy spelling compatibility SHALL be limited to loading saved version-1 composition plans.

#### Scenario: Release command and grant are accepted
- **WHEN** a user enters a lock release command or creates a grant containing `release`
- **THEN** the CLI or composer accepts the canonical spelling
- **AND** constructs the corresponding protocol release operation or capability

#### Scenario: Legacy interactive spelling is rejected
- **WHEN** a user enters `lock return` or supplies `return` as a grant capability through CLI or composer input
- **THEN** parsing rejects that input before submission or saving
- **AND** help or the error identifies `release` as the supported spelling

## RENAMED Requirements

- FROM: `### Requirement: MetaUpdate transaction events render as human-readable one-line summaries`
- TO: `### Requirement: TokenUpdate transaction events render as human-readable one-line summaries`

- FROM: `### Requirement: Token compose commands submit composed MetaUpdate operations`
- TO: `### Requirement: Token compose commands submit composed TokenUpdate operations`

## MODIFIED Requirements

### Requirement: Token holder and token-admin commands submit account-signed token transactions
The CLI SHALL let a user submit protocol-level token holder and token-admin operations through the `token` command space using an account signer, selected network context, and scoped TokenUpdate transactions for the selected token. All positional and required arguments SHALL have interactive prompt fallback when omitted in interactive mode. In `--non-interactive` mode the CLI SHALL produce actionable errors for missing required values.

#### Scenario: User submits a token transfer with all args
- **WHEN** a user runs `ccd-wallet token transfer` with a token identifier, recipient, amount, and signing account context
- **THEN** the CLI builds and submits a protocol-level token transfer transaction for that token
- **AND** the CLI reports the submitted transaction hash

#### Scenario: User submits a token transfer without args in interactive mode
- **WHEN** a user runs `ccd-wallet token transfer` with no positional or flag arguments in interactive mode
- **THEN** the CLI always presents an account selector showing all finalized accounts for the resolved network
- **AND** the CLI presents a token selector populated from tokens for which the selected account has a non-zero available balance, with the available amount shown as a hint
- **AND** the CLI prompts for the recipient address and amount, showing the available balance as context for the amount prompt

#### Scenario: User submits a token admin-role assignment
- **WHEN** a user runs `ccd-wallet token admin-roles assign` with a token identifier, target account, and one or more admin roles
- **THEN** the CLI builds and submits a protocol-level token admin-role assignment transaction for that token
- **AND** the CLI reports the submitted transaction hash

#### Scenario: User submits a token admin-role assignment interactively
- **WHEN** a user runs `ccd-wallet token admin-roles assign` with no required arguments in interactive mode
- **THEN** the CLI presents an account selector, then prompts for the token identifier, target address, and presents a multi-select of all known admin roles

#### Scenario: User submits a token metadata update
- **WHEN** a user runs `ccd-wallet token metadata update` with a token identifier and metadata URL payload
- **THEN** the CLI builds and submits a protocol-level token metadata update transaction for that token
- **AND** the CLI reports the submitted transaction hash


#### Scenario: Metadata update preserves URL and checksum
- **WHEN** a user submits a token metadata update with a URL and optional SHA-256 checksum
- **THEN** the scoped token update contains that URL and checksum when supplied
- **AND** no unrelated metadata fields are synthesized

### Requirement: Token lock commands manage protocol-level locks
The CLI SHALL let a user manage protocol-level token locks through nested `token lock` commands for lock creation, funding, lock-controlled sends, releases, and cancellation. All token lock mutation commands SHALL always present the account selector to make the signer explicit. For lock creation, the CLI SHALL support either repeated `--recipient` values for limited-recipient locks or `--any-recipient` for any-recipient locks, and those inputs SHALL be mutually exclusive. In interactive mode, when neither limited recipients nor `--any-recipient` is supplied, the CLI SHALL prompt for recipient mode before collecting any recipient-account inputs. For lock creation, the `--grant` option SHALL be optional in interactive mode and omitted grants SHALL be collected through a guided grant composition flow that prompts for a grant account and lets the user select known capabilities. For lock creation, omitted `--keep-alive` SHALL prompt in interactive mode with No selected by default and SHALL default to false in non-interactive mode. For fund, send, release, and cancel, the lock identifier SHALL have interactive prompt fallback when omitted. The token identifier for fund, send, and release SHALL be supplied via `--token` and SHALL have an interactive selector populated from the lock's configured token set when omitted. For lock send, limited-recipient locks SHALL continue to require a configured recipient, while any-recipient locks SHALL accept any resolved recipient account.

#### Scenario: User creates a limited-recipient token lock
- **WHEN** a user runs `ccd-wallet token lock create` with one or more `--recipient` values, the required lock configuration, and signing account context
- **THEN** the CLI submits a protocol-level lock-creation transaction with a limited recipient set
- **AND** the CLI reports the submitted transaction hash

#### Scenario: User creates an any-recipient token lock
- **WHEN** a user runs `ccd-wallet token lock create --any-recipient` with the required lock configuration and signing account context
- **THEN** the CLI submits a protocol-level lock-creation transaction with any-recipient lock configuration
- **AND** the CLI reports the submitted transaction hash

#### Scenario: User creates a token lock without grant arguments interactively
- **WHEN** a user runs `ccd-wallet token lock create` with no `--grant` arguments in interactive mode
- **THEN** the CLI prompts for at least one grant account
- **AND** the CLI lets the user select one or more lock capabilities from the known capability set for each grant
- **AND** the CLI asks whether another grant should be added

#### Scenario: User creates a token lock without keep-alive flag interactively
- **WHEN** a user runs `ccd-wallet token lock create` without `--keep-alive` in interactive mode
- **THEN** the CLI asks whether to keep the lock alive after funds are released
- **AND** the default selected answer is No

#### Scenario: User creates a token lock without recipient mode interactively
- **WHEN** a user runs `ccd-wallet token lock create` without `--recipient` values and without `--any-recipient` in interactive mode
- **THEN** the CLI prompts the user to choose between any-recipient and limited-recipient lock creation
- **AND** the CLI only prompts for recipient account inputs when the user chooses the limited-recipient option

#### Scenario: Token lock create rejects mixed recipient modes
- **WHEN** a user runs `ccd-wallet token lock create` with both `--any-recipient` and one or more `--recipient` values
- **THEN** the CLI rejects the invocation before submission
- **AND** the error explains that those inputs are mutually exclusive

#### Scenario: Token lock create rejects unknown grant capability
- **WHEN** a user runs `ccd-wallet token lock create --grant alice:fund,nonsense`
- **THEN** the CLI rejects the grant before submission
- **AND** the error identifies `nonsense` as an unknown lock capability

#### Scenario: User funds an existing token lock interactively
- **WHEN** a user runs `ccd-wallet token lock fund` with no required arguments in interactive mode
- **THEN** the CLI always presents an account selector
- **AND** the CLI prompts for the lock identifier
- **AND** the CLI presents a token selector populated from the lock's configured token set, with the signer's available balance shown as a hint per token
- **AND** the CLI prompts for the amount, showing the available balance as context

#### Scenario: User funds an existing token lock with explicit args
- **WHEN** a user runs `ccd-wallet token lock fund` with a lock identifier, `--token TOKEN_ID`, `--amount`, and account context
- **THEN** the CLI submits a protocol-level lock funding transaction for that lock
- **AND** the CLI reports the submitted transaction hash

#### Scenario: User sends from an existing limited-recipient token lock
- **WHEN** a user runs `ccd-wallet token lock send` with a limited-recipient lock identifier, `--token TOKEN_ID`, source, configured recipient, amount, and signing account context
- **THEN** the CLI submits a protocol-level lock-controlled send transaction for that lock
- **AND** the CLI reports the submitted transaction hash

#### Scenario: User sends from an existing any-recipient token lock
- **WHEN** a user runs `ccd-wallet token lock send` with an any-recipient lock identifier, `--token TOKEN_ID`, source, recipient, amount, and signing account context
- **THEN** the CLI accepts the resolved recipient without requiring membership in a configured recipient list
- **AND** submits a protocol-level lock-controlled send transaction for that lock

#### Scenario: User sends interactively and token selector shows locked balances
- **WHEN** a user runs `ccd-wallet token lock send` with a lock identifier and source address but no `--token` in interactive mode
- **THEN** the CLI presents a token selector populated from the lock's configured token set, with the source account's locked balance under that lock shown as a hint per token

#### Scenario: User returns funds from an existing token lock
- **WHEN** a user runs `ccd-wallet token lock release` with a lock identifier, `--token TOKEN_ID`, source, amount, and signing account context
- **THEN** the CLI submits a protocol-level lock release transaction for that lock
- **AND** the CLI reports the submitted transaction hash

#### Scenario: User cancels an existing lock
- **WHEN** a user runs `ccd-wallet token lock cancel` with a lock identifier and signing account context
- **THEN** the CLI submits a protocol-level lock cancellation transaction
- **AND** the CLI reports the submitted transaction hash

### Requirement: TokenUpdate transaction events render as human-readable one-line summaries
The CLI SHALL render TokenUpdate transaction events as concise single-line entries rather than pretty-printed JSON arrays. Token transfer events SHALL render as `Transfer <amount> <token>: <from> -> <to>` with optional `(locked @ <lock-id>)` annotations when lock context is present in the event payload. Lock lifecycle events SHALL render as `Lock created: <lock-id>` or `Lock destroyed: <lock-id>`.

#### Scenario: Finalized lock fund shows transfer event
- **WHEN** a `token lock fund` transaction finalizes
- **THEN** the event section shows a line of the form `- Transfer <amount> <token-id>: <sender> -> <sender> (locked @ <lock-id>)`


#### Scenario: Mixed operation events retain order
- **WHEN** a finalized TokenUpdate contains token transfer and lock lifecycle events
- **THEN** known events render as concise one-line summaries in their original order

#### Scenario: Unknown event does not hide known events
- **WHEN** a TokenUpdate contains an unknown event between known events
- **THEN** the CLI displays an explicit unknown-event entry preserving available raw details
- **AND** continues rendering subsequent known events without failing

### Requirement: Token compose commands submit composed TokenUpdate operations
The CLI SHALL expose a `token compose` command family for building, previewing, and submitting token composition plans. Submitted plans SHALL use unscoped TokenUpdate transaction support and SHALL submit all planned operations in one account transaction.

#### Scenario: Contributor reviews token compose help
- **WHEN** a contributor inspects `ccd-wallet token compose --help`
- **THEN** they can identify the interactive `token compose <PLAN>` form
- **AND** they can identify `token compose preview <PLAN>` and `token compose submit <PLAN>` subcommands

#### Scenario: User submits composed token operations
- **WHEN** a user submits a valid token composition plan
- **THEN** the CLI builds a single unscoped TokenUpdate account transaction containing the ordered plan operations
- **AND** the CLI reports one submitted transaction hash for the composed transaction
