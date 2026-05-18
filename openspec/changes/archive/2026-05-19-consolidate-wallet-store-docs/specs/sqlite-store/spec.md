## MODIFIED Requirements

### Requirement: Versioned schema migrations
The database SHALL contain a `schema_version` table with a single row indicating the current schema version. During this development reset, the migration set SHALL be consolidated into a single baseline schema that represents the full current wallet store. Existing development databases from earlier schema versions are not required to be migrated in-place and SHALL be rejected with an actionable error instructing the user to recreate the local database.

#### Scenario: Fresh database initialised to current version
- **WHEN** a new database is created
- **THEN** the consolidated baseline schema runs
- **AND** `schema_version` contains the current version number after initialization

#### Scenario: Existing database at current version requires no migration
- **WHEN** the CLI opens a database already at the current schema version
- **THEN** no migrations are applied
- **AND** the CLI proceeds without error

#### Scenario: Existing older development database is rejected actionably
- **WHEN** the CLI opens a database from an older pre-reset schema version
- **THEN** the CLI fails with an error indicating that the development database must be recreated
- **AND** the CLI does not attempt to apply the legacy migration chain in place

### Requirement: Schema includes seeds, seed_vaults, and wallet_state tables
The consolidated baseline schema SHALL define the current wallet store tables: `schema_version`, `wallet_state`, `seeds`, `seed_vaults`, `identities`, `identity_private_payloads`, `accounts`, `account_private_payloads`, `imported_account_vaults`, `imported_account_payloads`, `governance_key_vaults`, `governance_keys`, and `governance_key_payloads`.

#### Scenario: Schema tables are present after initialization
- **WHEN** a new database is initialized
- **THEN** querying `sqlite_master` returns table entries for `schema_version`, `wallet_state`, `seeds`, `seed_vaults`, `identities`, `identity_private_payloads`, `accounts`, `account_private_payloads`, `imported_account_vaults`, `imported_account_payloads`, `governance_key_vaults`, `governance_keys`, and `governance_key_payloads`
