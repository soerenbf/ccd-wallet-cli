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
The canonical command taxonomy SHALL document protocol-level token inspection, protocol-level token transfers, token policy operations, token admin-role changes, token metadata updates, and protocol-level lock operations under the `token` command space, using nested grouping where needed instead of exposing `metaupdate` as a user-facing command path. The documented user-facing branch SHALL use `show` for token inspection, `transfer` for holder transfers, `admin-roles` for token admin-role operations, and `lock show` for lock inspection. For lock fund, send, and return, the token identifier SHALL be documented as `--token` rather than a positional argument.

#### Scenario: Contributor reviews token command grouping
- **WHEN** a contributor reads the token section of `docs/commands.md`
- **THEN** they can find token show, token transfer, metadata, admin-role, and lock operations grouped under `token`
- **AND** they can find `token lock show` documented alongside the lock mutation commands
- **AND** they can see that `token lock fund`, `token lock send`, and `token lock return` accept `--token` for the token identifier
- **AND** they do not see `metaupdate` documented as a required user-facing command namespace

### Requirement: Staking taxonomy groups validator and delegation flows
The canonical command taxonomy SHALL document staking as a grouped command space that separates validator-oriented flows from delegation-oriented flows.

#### Scenario: Contributor reviews staking command grouping
- **WHEN** a contributor reads the staking section of `docs/commands.md`
- **THEN** they can identify a validator branch and a delegation branch within the staking taxonomy
- **AND** they can understand that the staking area is not a flat list of unrelated stake actions

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

