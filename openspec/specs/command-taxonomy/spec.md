# command-taxonomy Specification

## Purpose
TBD - created by archiving change document-command-spaces. Update Purpose after archive.
## Requirements
### Requirement: Canonical command taxonomy document
The repository SHALL include a canonical command taxonomy document at `docs/commands.md` that describes the wallet CLI command spaces and distinguishes between currently implemented and planned command branches.

#### Scenario: Contributor reviews command taxonomy
- **WHEN** a contributor opens `docs/commands.md`
- **THEN** they can identify the intended top-level CLI command spaces
- **AND** they can determine which documented command branches are implemented versus planned

### Requirement: Token operations are documented under the `token` command space
The canonical command taxonomy SHALL document protocol-level token inspection, protocol-level token transfers, token policy operations, token admin-role changes, token metadata updates, protocol-level lock operations, and token composition operations under the `token` command space, using nested grouping where needed instead of exposing `metaupdate` as a user-facing command path. The documented user-facing branch SHALL use `show` for token inspection, `transfer` for holder transfers, `admin-roles` for token admin-role operations, `lock show` for lock inspection, and `compose` for token MetaUpdate composition. For lock fund, send, and return, the token identifier SHALL be documented as `--token` rather than a positional argument.

#### Scenario: Contributor reviews token command grouping
- **WHEN** a contributor reads the token section of `docs/commands.md`
- **THEN** they can find token show, token transfer, metadata, admin-role, lock operations, and compose operations grouped under `token`
- **AND** they can find `token lock show` documented alongside the lock mutation commands
- **AND** they can see that `token lock fund`, `token lock send`, and `token lock return` accept `--token` for the token identifier
- **AND** they can find `token compose <PLAN>`, `token compose preview <PLAN>`, and `token compose submit <PLAN>` documented as token composition commands
- **AND** they do not see `metaupdate` documented as a required user-facing command namespace

### Requirement: Staking taxonomy groups validator and delegation flows
The canonical command taxonomy SHALL document staking as a grouped command space that separates validator-oriented flows from delegation-oriented flows. The implemented taxonomy SHALL expose a top-level `stake` command surface with generic `show` and `remove` actions, plus a nested `configure` area that includes a delegation branch and reserves a validator branch for validator-oriented staking flows.

#### Scenario: Contributor reviews staking command grouping
- **WHEN** a contributor reads the staking section of `docs/commands.md`
- **THEN** they can identify a validator branch and a delegation branch within the staking taxonomy
- **AND** they can understand that the staking area is not a flat list of unrelated stake actions
- **AND** they can identify `stake show`, `stake configure delegation`, and `stake remove` as part of the implemented command surface rather than only planned guidance

### Requirement: Validator taxonomy excludes deprecated legacy baker transactions
The canonical command taxonomy SHALL exclude deprecated legacy baker transaction families and SHALL describe validator-oriented staking flows in terms of modern `ConfigureBaker`-compatible behavior.

#### Scenario: Contributor reviews validator command scope
- **WHEN** a contributor reads the validator-oriented staking section of `docs/commands.md`
- **THEN** they do not see deprecated legacy baker transaction families documented as part of the intended command space
- **AND** they can see that the validator branch is scoped to modern validator configuration behavior used on recent protocol versions

### Requirement: Command-surface changes require documentation synchronization
The repository contribution guidance SHALL require changes to command-surface code or command taxonomy to update `docs/commands.md` in the same change.

#### Scenario: Contributor changes command structure
- **WHEN** a contributor updates clap command definitions or command modules that affect the CLI structure
- **THEN** `AGENTS.md` instructs them to update `docs/commands.md` in the same change
- **AND** the command taxonomy document remains the expected reference for command-surface review

### Requirement: Account show is documented under the account command space
The canonical command taxonomy SHALL document `account show` as an implemented account command for on-chain account inspection.

#### Scenario: Contributor reviews account command grouping
- **WHEN** a contributor reads the account section of `docs/commands.md`
- **THEN** they can find `account show` documented alongside account export, import, list, new, and rename
- **AND** they can identify it as an implemented account command

### Requirement: Ledger command space includes app inspection
The canonical command taxonomy SHALL document a Ledger app inspection command under the `ledger` command space. The command SHALL let users inspect the connected Concordium Ledger app name and, when the app supports it, the app version used for version-gated Ledger flows such as identity issuance.

#### Scenario: Contributor reviews Ledger command grouping
- **WHEN** a contributor reads the Ledger section of `docs/commands.md`
- **THEN** they can find a Ledger app inspection command such as `ledger show`
- **AND** they can see that it reports connected app name and app version when supported
- **AND** they can understand that Ledger identity issuance requires a sufficiently recent Concordium Ledger app version

### Requirement: Ledger command taxonomy documents recovery flows
The canonical command taxonomy SHALL document Ledger enrollment, Ledger recovery, Ledger app inspection, and local Ledger key-source removal under the `ledger` command space. The documented implemented Ledger branch SHALL include `setup`, `sync`, `show`, and `remove`; Ledger setup documentation SHALL indicate that enrollment supports immediate recovery through `--restore <NETWORK>`, and Ledger removal documentation SHALL indicate that removal affects local wallet state rather than the physical Ledger device.

#### Scenario: Contributor reviews Ledger recovery command grouping
- **WHEN** a contributor reads the Ledger section of `docs/commands.md`
- **THEN** they can find `ledger setup`, `ledger sync`, `ledger show`, and `ledger remove` documented as implemented commands
- **AND** they can see that `ledger setup` supports immediate recovery through `--restore <NETWORK>`
- **AND** they can understand that Ledger recovery belongs to the `ledger` command space rather than the `seed` command space
- **AND** they can understand that `ledger remove` removes local Ledger key-source state and does not modify the physical Ledger device

### Requirement: Governance Ledger signing is documented under governance update
The canonical command taxonomy SHALL document Ledger-backed on-chain governance signing as part of the implemented `governance update` command surface.

#### Scenario: Contributor reviews governance Ledger signing command
- **WHEN** a contributor reads `docs/commands.md`
- **THEN** they can find `governance update --ledger` documented under the governance command space
- **AND** they can see that Ledger-backed governance signing uses the Concordium Governance Ledger app rather than the local governance key vault

#### Scenario: Contributor reviews unsupported mixed signing
- **WHEN** a contributor reads the governance update documentation in `docs/commands.md`
- **THEN** they can see that Ledger governance signing is exclusive for a command invocation
- **AND** they can see that local governance key vault signatures are not mixed with Ledger signatures

### Requirement: Governance proposal workflow is documented under the governance command space
The canonical command taxonomy SHALL document a detached governance proposal workflow under the implemented `governance` command space alongside the existing all-in-one `governance update` command.

#### Scenario: Contributor reviews detached governance proposal command grouping
- **WHEN** a contributor reads `docs/commands.md`
- **THEN** they can find a documented `governance proposal` command family
- **AND** they can identify `create`, `sign`, and `submit` as its implemented detached-signing subcommands

#### Scenario: Contributor reviews relationship between proposal and update flows
- **WHEN** a contributor reads the governance section of `docs/commands.md`
- **THEN** they can see that `governance update` remains the all-in-one signing and submission flow
- **AND** they can see that `governance proposal` is the detached multi-party signing flow

### Requirement: Identity inspection and export are documented under the identity command space
The canonical command taxonomy SHALL document `identity show` and `identity export` as implemented identity commands for local identity inspection and explicit JSON export.

#### Scenario: Contributor reviews identity command grouping
- **WHEN** a contributor reads `docs/commands.md`
- **THEN** they can find `identity show` and `identity export` documented alongside identity list, issue, and rename commands
- **AND** they can identify both as implemented commands

### Requirement: Implemented stake commands are documented under the stake command space
The canonical command taxonomy SHALL document `stake show`, `stake configure delegation`, and `stake remove` as implemented staking commands. It SHALL also reserve `stake configure validator` as the validator-oriented configuration branch for future work.

#### Scenario: Contributor reviews implemented stake command grouping
- **WHEN** a contributor reads the staking section of `docs/commands.md`
- **THEN** they can find `stake show`, `stake configure delegation`, and `stake remove` documented as implemented commands
- **AND** they can find `stake configure validator` documented as the validator-oriented configuration branch
- **AND** they can identify that validator configuration itself is not yet implemented

### Requirement: Canonical command taxonomy documents the CCD command space
The canonical command taxonomy SHALL document a top-level `ccd` command space for native CCD account-transaction authoring. The documented `ccd` branch SHALL include `transfer` for simple CCD transfer flows and `schedule` for scheduled CCD transfer flows.

#### Scenario: Contributor reviews CCD command grouping
- **WHEN** a contributor reads `docs/commands.md`
- **THEN** they can find a documented top-level `ccd` command space
- **AND** they can identify `ccd transfer` and `ccd schedule` as the initial CCD authoring commands

### Requirement: Canonical command taxonomy scopes CCD separately from transaction inspection
The canonical command taxonomy SHALL describe `ccd` as the user-facing home for native CCD transfer authoring while keeping transaction inspection under the `transaction` command space.

#### Scenario: Contributor reviews CCD and transaction roles
- **WHEN** a contributor reads the command taxonomy document
- **THEN** they can see that `transaction show` remains under `transaction`
- **AND** they can see that native CCD transfer authoring belongs under `ccd`

