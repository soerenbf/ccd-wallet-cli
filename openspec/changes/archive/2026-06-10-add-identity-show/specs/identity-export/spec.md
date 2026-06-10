## ADDED Requirements

### Requirement: Identity export writes decrypted identity details to a JSON file
The CLI SHALL provide `ccd-wallet identity export <LABEL> [--out <FILE>]` to export a stored identity as a wallet-owned JSON file.

The export flow SHALL authenticate the user against the owning key source, decrypt the identity private payload, and write the resulting JSON only to the selected file destination. The CLI SHALL NOT print the full decrypted identity JSON to normal command output.

#### Scenario: Export completed identity to JSON file
- **WHEN** the user runs `ccd-wallet identity export my-id --out my-id.json`
- **AND** the selected identity is completed
- **AND** the user enters the correct key-source password
- **THEN** the CLI writes a JSON file for that identity to `my-id.json`
- **AND** the file includes plaintext metadata and the issued identity object
- **AND** `privatePayload.codeUri` is `null`
- **AND** the CLI does not print the full decrypted identity JSON to stdout

#### Scenario: Export pending identity to JSON file
- **WHEN** the user runs `ccd-wallet identity export my-id --out my-id.json`
- **AND** the selected identity is pending
- **AND** the user enters the correct key-source password
- **THEN** the CLI writes a JSON file for that identity to `my-id.json`
- **AND** the file includes plaintext metadata and decrypted `code_uri`
- **AND** the issued identity object is represented as absent or null

### Requirement: Identity export requires explicit destination selection
The identity export flow SHALL write sensitive JSON only to an explicit file destination. If `--out <FILE>` is supplied, the CLI SHALL use that path directly. If `--out` is omitted and prompting is available, the CLI SHALL prompt for an output destination instead of defaulting to stdout. If `--out` is omitted and prompting is unavailable, the CLI SHALL fail with an actionable error.

#### Scenario: Export prompts for destination when path is omitted interactively
- **WHEN** the user runs `ccd-wallet identity export my-id`
- **AND** `--out` is not supplied
- **AND** the command can prompt for an output destination
- **THEN** the CLI prompts for a destination path
- **AND** may suggest a default filename derived from the identity label such as `my-id.json`

#### Scenario: Export without explicit destination fails when prompting is unavailable
- **WHEN** the user runs `ccd-wallet identity export my-id`
- **AND** `--out` is not supplied
- **AND** the command cannot prompt for an output destination
- **THEN** the CLI exits with an actionable error
- **AND** no identity JSON is written

#### Scenario: Export writes to supplied file path
- **WHEN** the user supplies a valid output file path for `identity export`
- **THEN** the CLI writes the identity JSON to that path
- **AND** reports which identity was exported

### Requirement: Identity export uses a stable wallet-owned JSON schema
The identity export flow SHALL emit a stable wallet-owned JSON schema.

The exported JSON SHALL contain:
- a top-level `version` field;
- an `identity` object with `label`, `status`, `provider`, `identityIndex`, `createdAt`, and `expiresAt`;
- a `network` object with `label` and `genesisHash`;
- a `keySource` object with `kind` and `label`;
- a `privatePayload` object with `codeUri` and `identityObject`.

`privatePayload.codeUri` SHALL contain the stored pending `code_uri` when one is retained locally and SHALL be `null` for completed identities whose `code_uri` has been discarded. Timestamps in the exported JSON SHALL use RFC 3339 UTC strings. The exported JSON SHALL NOT include internal database identifiers such as SQLite row ids or `signer_owner_id`.

#### Scenario: Pending identity exports null identity object
- **WHEN** the user exports a pending identity
- **THEN** `privatePayload.identityObject` is `null`
- **AND** `privatePayload.codeUri` contains the stored pending code URI
- **AND** `identity.expiresAt` is `null` when no expiry is known

#### Scenario: Completed identity exports null code URI
- **WHEN** the user exports a completed identity
- **THEN** `privatePayload.codeUri` is `null`

### Requirement: Identity export uses normal identity resolution rules
The identity export flow SHALL resolve the target identity using the wallet's normal identity-selection behavior, including rename-style interactive ambiguity resolution.

#### Scenario: Label resolves one identity for export
- **WHEN** the user exports identity label `my-id`
- **AND** exactly one stored identity matches that label
- **THEN** the CLI exports that identity

#### Scenario: Ambiguous identity export fails when prompting is unavailable
- **WHEN** the supplied identity label matches multiple stored identities
- **AND** the command cannot prompt to disambiguate them
- **THEN** the CLI exits with an actionable ambiguity error
- **AND** no identity JSON is written

#### Scenario: Wrong key-source password does not write export file
- **WHEN** the user runs `ccd-wallet identity export my-id --out my-id.json`
- **AND** enters an incorrect password for the identity's owning key source
- **THEN** the CLI does not write decrypted identity JSON to `my-id.json`
- **AND** it does not reveal decrypted identity data in normal command output
