# sqlite-store Specification

## Purpose
TBD - created by archiving change sqlite-wallet-store. Update Purpose after archive.
## Requirements
### Requirement: Database stored in OS application-data directory
The CLI SHALL resolve the wallet database path as `{data_dir}/ccd-wallet/wallet.db`, where `{data_dir}` is the platform application-data directory (`~/Library/Application Support` on macOS, `~/.local/share` on Linux, `%APPDATA%` on Windows).

#### Scenario: Database created on first run
- **WHEN** no `wallet.db` exists at the resolved path
- **THEN** the CLI creates parent directories as needed
- **AND** creates a new SQLite database file at that path
- **AND** initializes it with the current schema version

#### Scenario: Database path is separate from config directory
- **WHEN** the CLI resolves paths for both `wallet.db` and `config.json`
- **THEN** `wallet.db` resolves under the OS data directory
- **AND** `config.json` resolves under the OS config directory
- **AND** the two paths are distinct

#### Scenario: Database path overridden via environment variable
- **WHEN** the environment variable `CCD_WALLET_DB_PATH` is set to an absolute path
- **THEN** the CLI uses that path as the database file location instead of the default

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

### Requirement: SQLite foreign keys enabled for wallet database connections
The wallet database connection SHALL enable SQLite foreign-key enforcement using `PRAGMA foreign_keys = ON` before migrations or normal operations are performed.

#### Scenario: Open wallet database connection
- **WHEN** the CLI opens `wallet.db`
- **THEN** the connection has `PRAGMA foreign_keys` enabled
- **AND** foreign-key cascades are enforced for that connection

