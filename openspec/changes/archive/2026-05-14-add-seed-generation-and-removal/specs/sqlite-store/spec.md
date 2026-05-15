## ADDED Requirements

### Requirement: SQLite foreign keys enabled for wallet database connections
The wallet database connection SHALL enable SQLite foreign-key enforcement using `PRAGMA foreign_keys = ON` before migrations or normal operations are performed.

#### Scenario: Open wallet database connection
- **WHEN** the CLI opens `wallet.db`
- **THEN** the connection has `PRAGMA foreign_keys` enabled
- **AND** foreign-key cascades are enforced for that connection

### Requirement: Schema migration adds seed vault cascade
The database migration system SHALL add a migration from schema version 1 to schema version 2 that recreates `seed_vaults` with `seed_id REFERENCES seeds(id) ON DELETE CASCADE`.

#### Scenario: Migrate version 1 database
- **WHEN** the CLI opens a version 1 database
- **THEN** migration version 2 runs
- **AND** the resulting `seed_vaults` table has cascade-on-delete behavior for its `seed_id` foreign key
- **AND** existing `seed_vaults` rows are preserved
- **AND** `schema_version` is updated to `2`

#### Scenario: Fresh database initializes with cascade migration applied
- **WHEN** a new database is created
- **THEN** all migrations run through version 2
- **AND** deleting a seed cascades to its seed vault
