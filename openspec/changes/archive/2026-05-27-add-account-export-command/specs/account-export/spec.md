## ADDED Requirements

### Requirement: Account export writes a minimal SDK-compatible signer file
The CLI SHALL provide an `account export` command that exports a stored wallet account to a JSON file compatible with `concordium_rust_sdk::types::WalletAccount::from_json_file`. In the initial version, the exported JSON SHALL contain the account `address` and `accountKeys` only.

#### Scenario: Export derived account to minimal signer JSON
- **WHEN** the user exports a finalized derived account
- **THEN** the CLI writes a JSON file containing that account's `address` and `accountKeys`
- **AND** the file is accepted by `WalletAccount::from_json_file`

#### Scenario: Export imported account to minimal signer JSON
- **WHEN** the user exports a finalized imported account
- **THEN** the CLI writes a JSON file containing that account's `address` and `accountKeys`
- **AND** the file is accepted by `WalletAccount::from_json_file`

### Requirement: Account export requires explicit destination selection
The account export flow SHALL write plaintext signing material only to a user-selected file destination. In non-interactive mode, the destination path MUST be supplied explicitly.

#### Scenario: Non-interactive export without destination fails
- **WHEN** the user runs `account export` in non-interactive mode without supplying an output file path
- **THEN** the CLI exits with an actionable error
- **AND** no signer JSON is written

#### Scenario: Export writes to supplied file path
- **WHEN** the user supplies a valid output file path for `account export`
- **THEN** the CLI writes the signer JSON to that path
- **AND** reports which account was exported

### Requirement: Account export uses normal account resolution rules
The account export flow SHALL resolve the target account using the wallet's normal account-selection behavior, including network-scoped label uniqueness and existing interactive selection conventions.

#### Scenario: Network-scoped label resolves one account
- **WHEN** the user exports account label `alice` for a network where exactly one account uses that label
- **THEN** the CLI exports that account

#### Scenario: Ambiguous account selection fails in non-interactive mode
- **WHEN** the supplied account label matches multiple stored accounts and the resolved context does not disambiguate them
- **THEN** the CLI exits with an actionable ambiguity error
- **AND** no signer JSON is written
