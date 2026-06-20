## MODIFIED Requirements

### Requirement: Token lock commands manage protocol-level locks
The CLI SHALL let a user manage protocol-level token locks through nested `token lock` commands for lock creation, funding, lock-controlled sends, returns, and cancellation. All token lock mutation commands SHALL always present the account selector to make the signer explicit. For lock creation, the CLI SHALL support either repeated `--recipient` values for limited-recipient locks or `--any-recipient` for any-recipient locks, and those inputs SHALL be mutually exclusive. In interactive mode, when neither limited recipients nor `--any-recipient` is supplied, the CLI SHALL prompt for recipient mode before collecting any recipient-account inputs. For lock creation, the `--grant` option SHALL be optional in interactive mode and omitted grants SHALL be collected through a guided grant composition flow that prompts for a grant account and lets the user select known capabilities. For lock creation, omitted `--keep-alive` SHALL prompt in interactive mode with No selected by default and SHALL default to false in non-interactive mode. For fund, send, return, and cancel, the lock identifier SHALL have interactive prompt fallback when omitted. The token identifier for fund, send, and return SHALL be supplied via `--token` and SHALL have an interactive selector populated from the lock's configured token set when omitted. For lock send, limited-recipient locks SHALL continue to require a configured recipient, while any-recipient locks SHALL accept any resolved recipient account.

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
- **THEN** the CLI asks whether to keep the lock alive after funds are returned
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
- **WHEN** a user runs `ccd-wallet token lock return` with a lock identifier, `--token TOKEN_ID`, source, amount, and signing account context
- **THEN** the CLI submits a protocol-level lock return transaction for that lock
- **AND** the CLI reports the submitted transaction hash

#### Scenario: User cancels an existing lock
- **WHEN** a user runs `ccd-wallet token lock cancel` with a lock identifier and signing account context
- **THEN** the CLI submits a protocol-level lock cancellation transaction
- **AND** the CLI reports the submitted transaction hash

### Requirement: Token lock commands can inspect lock state
The CLI SHALL let a user inspect protocol-level lock state through `ccd-wallet token lock show` using the protocol lock-info query support. Human-readable lock output SHALL distinguish between limited-recipient locks and any-recipient locks, rendering the any-recipient variant as `any eligible account`.

#### Scenario: User shows limited-recipient lock details
- **WHEN** a user runs `ccd-wallet token lock show` for a lock configured with explicit recipient accounts
- **THEN** the CLI queries lock information for that identifier
- **AND** the CLI prints a human-readable summary listing those recipient accounts

#### Scenario: User shows any-recipient lock details
- **WHEN** a user runs `ccd-wallet token lock show` for a lock configured with the any-recipient variant
- **THEN** the CLI queries lock information for that identifier
- **AND** the CLI prints a human-readable summary describing the recipients as `any eligible account`
