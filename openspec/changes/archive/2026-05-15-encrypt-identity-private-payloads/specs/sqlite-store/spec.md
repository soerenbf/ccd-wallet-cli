## MODIFIED Requirements

### Requirement: Versioned schema migrations
The database SHALL contain a `schema_version` table with a single row indicating the current schema version. During this development reset, the migration set SHALL be consolidated into a new initial schema version that represents the full current wallet schema. Existing development databases from earlier schema versions are not required to be migrated in-place.

#### Scenario: Fresh database initialised to current version
- **WHEN** a new database is created
- **THEN** the consolidated initial schema runs
- **AND** `schema_version` contains the current version number after initialization

#### Scenario: Existing database at current version requires no migration
- **WHEN** the CLI opens a database already at the current schema version
- **THEN** no migrations are applied
- **AND** the CLI proceeds without error

#### Scenario: Existing older development database is unsupported
- **WHEN** the CLI opens a database from an older pre-reset schema version
- **THEN** the CLI may fail with an actionable error indicating that the development database must be recreated

### Requirement: Schema includes seeds, seed_vaults, and wallet_state tables
The initial schema SHALL define the following tables: `schema_version`, `wallet_state`, `seeds`, `seed_vaults`, `identities`, and `identity_private_payloads`.

#### Scenario: Schema tables are present after initialization
- **WHEN** a new database is initialized
- **THEN** querying `sqlite_master` returns table entries for `schema_version`, `wallet_state`, `seeds`, `seed_vaults`, `identities`, and `identity_private_payloads`

## REMOVED Requirements

### Requirement: Schema migration adds seed vault cascade
**Reason**: The development migration set is being consolidated into a new initial schema. Seed vault cascade behavior is part of the new initial schema rather than a separate version 2 migration.

**Migration**: Delete/recreate development `wallet.db` files instead of applying old incremental migrations.
