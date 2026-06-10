## MODIFIED Requirements

### Requirement: Token command space submits protocol-level token operations
The CLI SHALL expose a top-level `token` command space for protocol-level token operations using the user-facing names `show`, `transfer`, `mint`, `burn`, `allow-list`, `deny-list`, `pause`, `unpause`, `admin-roles`, `metadata`, `lock`, and `compose`.

#### Scenario: Contributor reviews token help
- **WHEN** a contributor inspects the implemented `ccd-wallet` command surface
- **THEN** they can find a top-level `token` command
- **AND** they can find `show`, `transfer`, `mint`, `burn`, `allow-list`, `deny-list`, `pause`, `unpause`, `admin-roles`, `metadata`, `lock`, and `compose` under it

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
