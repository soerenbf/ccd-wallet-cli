## ADDED Requirements

### Requirement: Token command space submits protocol-level token operations
The CLI SHALL expose a top-level `token` command space for protocol-level token operations using the user-facing names `show`, `transfer`, `mint`, `burn`, `allow-list`, `deny-list`, `pause`, `unpause`, `admin-roles`, `metadata`, and `lock`.

#### Scenario: Contributor reviews token help
- **WHEN** a contributor inspects the implemented `ccd-wallet` command surface
- **THEN** they can find a top-level `token` command
- **AND** they can find `show`, `transfer`, `mint`, `burn`, `allow-list`, `deny-list`, `pause`, `unpause`, `admin-roles`, `metadata`, and `lock` under it

### Requirement: Token show command can inspect token state
The CLI SHALL let a user inspect protocol-level token state through `ccd-wallet token show` using the protocol token-info query support.

#### Scenario: User shows token details
- **WHEN** a user runs `ccd-wallet token show` with a token identifier and network or node context
- **THEN** the CLI queries token information for that identifier
- **AND** the CLI prints a human-readable summary of the resolved token state

### Requirement: Token holder and token-admin commands submit account-signed token transactions
The CLI SHALL let a user submit protocol-level token holder and token-admin operations through the `token` command space using an account signer, selected network context, and the pinned SDK branch's token-operation submission support. All positional and required arguments SHALL have interactive prompt fallback when omitted in interactive mode. In `--non-interactive` mode the CLI SHALL produce actionable errors for missing required values.

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

### Requirement: Token lock commands manage protocol-level locks
The CLI SHALL let a user manage protocol-level token locks through nested `token lock` commands for lock creation, funding, lock-controlled sends, returns, and cancellation. All token lock mutation commands SHALL always present the account selector to make the signer explicit. For fund, send, return, and cancel, the lock identifier SHALL have interactive prompt fallback when omitted. The token identifier for fund, send, and return SHALL be supplied via `--token` and SHALL have an interactive selector populated from the lock's configured token set when omitted.

#### Scenario: User creates a token lock
- **WHEN** a user runs `ccd-wallet token lock create` with the required lock configuration and signing account context
- **THEN** the CLI submits a protocol-level lock-creation transaction
- **AND** the CLI reports the submitted transaction hash

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

#### Scenario: User cancels an existing token lock
- **WHEN** a user runs `ccd-wallet token lock cancel` with a lock identifier and signing account context
- **THEN** the CLI submits a protocol-level lock cancellation transaction for that lock
- **AND** the CLI reports the submitted transaction hash

### Requirement: MetaUpdate transaction events render as human-readable one-line summaries
The CLI SHALL render MetaUpdate transaction events as concise single-line entries rather than pretty-printed JSON arrays. Token transfer events SHALL render as `Transfer <amount> <token>: <from> -> <to>` with optional `(locked @ <lock-id>)` annotations when lock context is present in the event payload. Lock lifecycle events SHALL render as `Lock created: <lock-id>` or `Lock destroyed: <lock-id>`.

#### Scenario: Finalized lock fund shows transfer event
- **WHEN** a `token lock fund` transaction finalizes
- **THEN** the event section shows a line of the form `- Transfer <amount> <token-id>: <sender> -> <sender> (locked @ <lock-id>)`

### Requirement: Token lock commands can inspect lock state
The CLI SHALL let a user inspect protocol-level lock state through `ccd-wallet token lock show` using the protocol lock-info query support.

#### Scenario: User shows lock details
- **WHEN** a user runs `ccd-wallet token lock show` with a lock identifier and network or node context
- **THEN** the CLI queries lock information for that identifier
- **AND** the CLI prints a human-readable summary of the resolved lock state
