## MODIFIED Requirements

### Requirement: Token command space submits protocol-level token operations
The CLI SHALL expose a top-level `token` command space for protocol-level token operations using the user-facing names `show`, `transfer`, `mint`, `burn`, `allow-list`, `deny-list`, `pause`, `unpause`, `admin-roles`, `metadata`, `lock`, and `compose`.

#### Scenario: Contributor reviews token help
- **WHEN** a contributor inspects the implemented `ccd-wallet` command surface
- **THEN** they can find a top-level `token` command
- **AND** they can find `show`, `transfer`, `mint`, `burn`, `allow-list`, `deny-list`, `pause`, `unpause`, `admin-roles`, `metadata`, `lock`, and `compose` under it

### Requirement: Token lock commands manage protocol-level locks
The CLI SHALL let a user manage protocol-level token locks through nested `token lock` commands for lock creation, funding, lock-controlled sends, returns, and cancellation. All token lock mutation commands SHALL always present the account selector to make the signer explicit. For lock creation, the `--grant` option SHALL be optional in interactive mode and omitted grants SHALL be collected through a guided grant composition flow that prompts for a grant account and lets the user select known capabilities. For fund, send, return, and cancel, the lock identifier SHALL have interactive prompt fallback when omitted. The token identifier for fund, send, and return SHALL be supplied via `--token` and SHALL have an interactive selector populated from the lock's configured token set when omitted.

#### Scenario: User creates a token lock
- **WHEN** a user runs `ccd-wallet token lock create` with the required lock configuration and signing account context
- **THEN** the CLI submits a protocol-level lock-creation transaction
- **AND** the CLI reports the submitted transaction hash

#### Scenario: User creates a token lock without grant arguments interactively
- **WHEN** a user runs `ccd-wallet token lock create` with no `--grant` arguments in interactive mode
- **THEN** the CLI prompts for at least one grant account
- **AND** the CLI lets the user select one or more lock capabilities from the known capability set for each grant
- **AND** the CLI asks whether another grant should be added

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

#### Scenario: User sends from an existing token lock
- **WHEN** a user runs `ccd-wallet token lock send` with a lock identifier, `--token TOKEN_ID`, source, recipient, amount, and signing account context
- **THEN** the CLI submits a protocol-level lock-controlled send transaction for that lock
- **AND** the CLI reports the submitted transaction hash

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

## ADDED Requirements

### Requirement: Token compose commands submit composed MetaUpdate operations
The CLI SHALL expose a `token compose` command family for building, previewing, and submitting token composition plans. Submitted plans SHALL use protocol-level MetaUpdate transaction support and SHALL submit all planned operations in one account transaction.

#### Scenario: Contributor reviews token compose help
- **WHEN** a contributor inspects `ccd-wallet token compose --help`
- **THEN** they can identify the interactive `token compose <PLAN>` form
- **AND** they can identify `token compose preview <PLAN>` and `token compose submit <PLAN>` subcommands

#### Scenario: User submits composed token operations
- **WHEN** a user submits a valid token composition plan
- **THEN** the CLI builds a single MetaUpdate account transaction containing the ordered plan operations
- **AND** the CLI reports one submitted transaction hash for the composed transaction
