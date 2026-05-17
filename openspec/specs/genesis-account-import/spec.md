# genesis-account-import Specification

## Purpose
TBD - created by archiving change add-imported-account-vault-and-genesis-import. Update Purpose after archive.
## Requirements
### Requirement: Genesis account JSON files can be imported as accounts
The CLI SHALL provide a genesis-account import flow that imports one genesis account JSON file as a wallet account on a resolved network. The import SHALL parse the account address and secret material needed to later sign transactions with the account.

#### Scenario: Import valid genesis account file
- **WHEN** the user imports a valid genesis account JSON file for a configured network
- **THEN** the CLI stores a finalized imported account for that network
- **AND** stores the imported account secret material encrypted under the network's imported accounts vault

#### Scenario: Import malformed genesis account file
- **WHEN** the user imports a file that is not a valid genesis account JSON bundle
- **THEN** the CLI exits with an actionable parse error
- **AND** no account row or imported secret payload is written

### Requirement: Genesis import requires a network context
The genesis-account import flow SHALL resolve a concrete configured network before writing account data. The imported account SHALL be associated with that network's `network_genesis_hash`.

#### Scenario: Explicit network import
- **WHEN** the user imports a genesis account file with `--network local`
- **THEN** the CLI resolves the configured network named `local`
- **AND** stores the imported account under `local`'s genesis hash

#### Scenario: Missing network in non-interactive mode
- **WHEN** the user imports a genesis account file in non-interactive mode without a network argument or active default
- **THEN** the CLI exits with an actionable error
- **AND** no account row or imported secret payload is written

### Requirement: Genesis import requires an account label
The genesis-account import flow SHALL require a user-facing account label. If the label is omitted in interactive mode, the CLI SHALL prompt for it and suggest the imported JSON filename stem as the default/placeholder value. The final label SHALL pass normal account-label validation and SHALL be unique across all accounts on the resolved network.

#### Scenario: Explicit label is accepted
- **WHEN** the user imports `baker-0.json` with label `local_baker`
- **THEN** the imported account is stored with label `local_baker`

#### Scenario: Missing label prompts with filename stem
- **WHEN** the user imports `baker-0.json` in interactive mode without a label
- **THEN** the CLI prompts for an account label
- **AND** suggests `baker-0` as the default or placeholder value

#### Scenario: Duplicate network label is rejected
- **WHEN** any account on the resolved network already uses label `baker-0`
- **AND** the user imports a genesis account with label `baker-0`
- **THEN** the CLI rejects the import with an actionable duplicate-label error
- **AND** no account row or imported secret payload is written

#### Scenario: Missing label fails in non-interactive mode
- **WHEN** the user imports a genesis account file in non-interactive mode without a label
- **THEN** the CLI exits with an actionable error
- **AND** no account row or imported secret payload is written

### Requirement: Genesis import is single-file in the initial version
The genesis-account import flow SHALL accept one account JSON file per invocation. Directory import and bulk import SHALL NOT be required by the initial version.

#### Scenario: File path import
- **WHEN** the user supplies a path to a single genesis account JSON file
- **THEN** the CLI attempts to import exactly that account file

#### Scenario: Directory path is not treated as bulk import
- **WHEN** the user supplies a directory path to the initial genesis import command
- **THEN** the CLI exits with an actionable error
- **AND** does not scan or import multiple account files

