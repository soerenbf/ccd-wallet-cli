# ccd-command-execution Specification

## Purpose
TBD - created by archiving change add-ccd-command-space. Update Purpose after archive.
## Requirements
### Requirement: CCD command space exposes native transfer authoring commands
The CLI SHALL expose a top-level `ccd` command space for native CCD account-transaction authoring. The initial user-facing commands under that space SHALL be `transfer` for simple CCD transfers and `schedule` for scheduled CCD transfers.

#### Scenario: Contributor reviews CCD command help
- **WHEN** a contributor inspects the implemented `ccd-wallet` command surface
- **THEN** they can find a top-level `ccd` command
- **AND** they can find `transfer` and `schedule` under it

### Requirement: CCD transfer submits simple transfer transactions with optional memos
The CLI SHALL let a user submit a simple CCD transfer through `ccd-wallet ccd transfer` using a finalized local sender account, a recipient account, a decimal CCD amount, and an optional memo. When no memo is supplied, the CLI SHALL submit the simple-transfer transaction family. When a memo is supplied, the CLI SHALL submit the simple-transfer-with-memo transaction family.

#### Scenario: User submits simple transfer without memo
- **WHEN** a user runs `ccd-wallet ccd transfer alice --recipient bob --amount 12.5 --network testnet`
- **AND** `alice` resolves to a finalized local signing-capable account on `testnet`
- **THEN** the CLI builds and submits a simple CCD transfer transaction
- **AND** reports the submitted transaction hash

#### Scenario: User submits simple transfer with memo
- **WHEN** a user runs `ccd-wallet ccd transfer alice --recipient bob --amount 12.5 --memo "invoice 7" --network testnet`
- **AND** `alice` resolves to a finalized local signing-capable account on `testnet`
- **THEN** the CLI builds and submits a simple CCD transfer with memo
- **AND** reports the submitted transaction hash

#### Scenario: Transfer prompts for missing values in interactive mode
- **WHEN** a user runs `ccd-wallet ccd transfer` in interactive mode without required sender, recipient, or amount inputs
- **THEN** the CLI resolves network and sender context through the shared sender-resolution rules
- **AND** prompts for any remaining required non-secret values before confirmation

### Requirement: CCD schedule submits scheduled transfer transactions with optional memos
The CLI SHALL let a user submit a scheduled CCD transfer through `ccd-wallet ccd schedule` using a finalized local sender account, a recipient account, one or more release entries, and an optional memo. Each release entry SHALL be supplied as a repeated `--release <RFC3339=CCD>` option where the timestamp is an RFC3339 instant and the amount is a decimal CCD value. When no memo is supplied, the CLI SHALL submit the scheduled-transfer transaction family. When a memo is supplied, the CLI SHALL submit the scheduled-transfer-with-memo transaction family.

#### Scenario: User submits scheduled transfer without memo
- **WHEN** a user runs `ccd-wallet ccd schedule alice --recipient bob --release 2026-07-01T00:00:00Z=10 --release 2026-10-01T00:00:00Z=15.5 --network testnet`
- **AND** `alice` resolves to a finalized local signing-capable account on `testnet`
- **THEN** the CLI builds and submits a scheduled CCD transfer using those two release entries
- **AND** reports the submitted transaction hash

#### Scenario: User submits scheduled transfer with memo
- **WHEN** a user runs `ccd-wallet ccd schedule alice --recipient bob --release 2026-07-01T00:00:00Z=10 --memo "vesting tranche" --network testnet`
- **AND** `alice` resolves to a finalized local signing-capable account on `testnet`
- **THEN** the CLI builds and submits a scheduled CCD transfer with memo
- **AND** reports the submitted transaction hash

#### Scenario: Scheduled transfer rejects invalid release entry format
- **WHEN** a user runs `ccd-wallet ccd schedule alice --recipient bob --release tomorrow=10 --network testnet`
- **THEN** the CLI rejects the release entry before submission
- **AND** explains that `--release` must use `RFC3339=CCD` format

### Requirement: CCD command signing supports all local account source kinds
`ccd transfer` and `ccd schedule` SHALL resolve sender accounts through the shared local signing-account rules and SHALL support seed-backed, imported, and Ledger-backed finalized local accounts. Ledger-backed sender accounts SHALL be signed through the connected Concordium Ledger app using the transaction-family-specific Ledger signing flow for the selected CCD payload.

#### Scenario: Seed-backed sender signs CCD transfer
- **WHEN** a user runs `ccd-wallet ccd transfer alice --recipient bob --amount 1 --network testnet`
- **AND** `alice` is a seed-backed finalized local account on `testnet`
- **THEN** the CLI resolves signing material from the owning seed key source
- **AND** submits the transfer after confirmation

#### Scenario: Imported sender signs CCD transfer
- **WHEN** a user runs `ccd-wallet ccd transfer imported-main --recipient bob --amount 1 --network testnet`
- **AND** `imported-main` is an imported finalized local account on `testnet`
- **THEN** the CLI resolves signing material from the imported account vault
- **AND** submits the transfer after confirmation

#### Scenario: Ledger-backed sender signs scheduled transfer
- **WHEN** a user runs `ccd-wallet ccd schedule ledger-main --recipient bob --release 2026-07-01T00:00:00Z=10 --network testnet`
- **AND** `ledger-main` is a Ledger-backed finalized local account on `testnet`
- **THEN** the CLI resolves the account through its enrolled Ledger signer owner
- **AND** requests the required signature from the connected Concordium Ledger app for the scheduled-transfer payload family
- **AND** submits the transaction only if Ledger signing succeeds

### Requirement: CCD commands accept local account labels for recipients
`ccd transfer` and `ccd schedule` SHALL accept either raw account addresses or finalized local account labels for recipient inputs. Recipient local account labels SHALL resolve through the shared account-reference behavior within the already resolved network context.

#### Scenario: Transfer recipient resolves from local account label
- **WHEN** a user runs `ccd-wallet ccd transfer alice --recipient bob --amount 2 --network testnet`
- **AND** `bob` matches a finalized local account on `testnet`
- **THEN** the CLI resolves `bob` to its account address before submission
- **AND** submits the transfer to the resolved address

