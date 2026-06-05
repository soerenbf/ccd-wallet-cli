## MODIFIED Requirements

### Requirement: Accounts are persisted as plaintext metadata plus encrypted private payloads
The system SHALL store wallet-managed accounts in the local SQLite database as plaintext relational metadata plus source-specific encrypted private payload data. Derived account metadata SHALL include the network identity (`network_genesis_hash`), owning signer owner id, identity provider index, identity index, credential counter, user-supplied account label, lifecycle status, and timestamps. Imported account metadata SHALL reference an imported account vault instead of a signer owner. Derived account private payload data SHALL be encrypted under the owning signer owner's password domain and stored as a structured payload object rather than as a bare encrypted address string.

In the initial version, the derived encrypted payload SHALL contain the account address. The payload structure MUST be extensible so future encrypted derived account details can be added without redesigning the storage model. Account labels SHALL be queryable and updatable by scoped label without changing the account's underlying source tuple.

#### Scenario: New derived account row stores plaintext indexing metadata
- **WHEN** the wallet creates a new pending derived account record
- **THEN** the plaintext account row stores `network_genesis_hash`, `signer_owner_id`, `ip_identity`, `identity_index`, `credential_counter`, label, status, and timestamps
- **AND** the account address is not stored in plaintext columns

#### Scenario: Encrypted derived account payload stores structured account data
- **WHEN** the wallet stores private data for a derived account
- **THEN** it serializes a structured derived account private payload
- **AND** encrypts that structure under the owning signer owner's password domain
- **AND** does not encrypt the address as a standalone primitive value

#### Scenario: Renaming account preserves source tuple
- **WHEN** the user renames an account label within a network scope
- **THEN** only the plaintext `label` changes
- **AND** the derived tuple `(network_genesis_hash, signer_owner_id, ip_identity, identity_index, credential_counter)` or imported vault source remains unchanged

### Requirement: Account uniqueness follows the credential derivation tuple
The system SHALL enforce derived account uniqueness within the tuple `(network_genesis_hash, signer_owner_id, ip_identity, identity_index, credential_counter)`. The `(network_genesis_hash, label)` pair SHALL also be unique for wallet-local naming across derived and imported accounts.

#### Scenario: Duplicate credential counter for same signer-owner identity tuple is rejected
- **WHEN** a derived account row already exists for a given `(network_genesis_hash, signer_owner_id, ip_identity, identity_index, credential_counter)` tuple
- **THEN** inserting another derived account row for the same tuple fails

#### Scenario: Same label on different networks is allowed
- **WHEN** two account rows use the same label but different `network_genesis_hash` values
- **THEN** both account rows can be stored

### Requirement: Account private payloads are encrypted under the owning seed password domain
The account storage layer SHALL encrypt derived account private payloads using the owning signer owner's DEK after that DEK has been unlocked by the signer owner's local password. Derived account private payload encryption SHALL use the same signer-owner encryption model as identity private payloads, including unique nonces and AAD binding the ciphertext to the account row and ownership metadata.

#### Scenario: Correct signer owner password decrypts account payload
- **WHEN** the user unlocks the owning signer owner with the correct local password
- **THEN** the system can decrypt that signer owner's derived account private payloads
- **AND** recover the structured derived account private payload

#### Scenario: Wrong signer owner password cannot decrypt account payload
- **WHEN** the user supplies an incorrect password for the owning signer owner
- **THEN** derived account private payload decryption fails
- **AND** no plaintext account address is exposed

#### Scenario: AAD prevents account payload transplantation
- **WHEN** an encrypted derived account private payload is copied to another account row or signer-owner context
- **THEN** AEAD authentication fails during decryption

### Requirement: Account private payloads cascade with account rows and owning seeds
The encrypted derived account private payload table SHALL reference the account metadata row with `ON DELETE CASCADE`. Derived account rows owned by a signer owner SHALL also be removed when the owning signer owner is deleted so that encrypted derived account payloads do not outlive their signer-owner domain.

#### Scenario: Deleting account deletes private payload
- **WHEN** an account metadata row is deleted
- **THEN** its encrypted source-specific private payload row is deleted automatically by SQLite foreign-key cascade

#### Scenario: Deleting signer owner deletes owned derived accounts and payloads
- **WHEN** a signer owner row is deleted
- **THEN** derived account rows owned by that signer owner are deleted automatically
- **AND** encrypted derived account private payload rows for those accounts are deleted automatically

#### Scenario: Pruning a network partition deletes accounts and payloads
- **WHEN** the storage layer deletes all account rows whose `network_genesis_hash = abc`
- **THEN** all matching account rows are removed
- **AND** their source-specific private payload rows are deleted automatically by SQLite foreign-key cascade

### Requirement: Account rows are listable and searchable by plaintext metadata with optional address reveal
The system SHALL support listing accounts by network scope, signer-owner scope, and relevant plaintext filters such as status using plaintext metadata. Account addresses remain encrypted by default and SHALL only be revealed when the CLI explicitly requests address display. Listing and interactive search for accounts SHALL use plaintext metadata unless address reveal has been explicitly requested.

#### Scenario: Default account listing uses plaintext metadata only
- **WHEN** the CLI lists accounts without requesting addresses
- **THEN** it reads account rows using plaintext metadata
- **AND** does not decrypt private payloads just to produce the list

#### Scenario: Filter accounts by status
- **WHEN** the CLI lists accounts with a status filter
- **THEN** it filters account rows using the stored plaintext status metadata
- **AND** does not decrypt private payloads just to apply the filter

#### Scenario: Fuzzy rename search uses plaintext metadata
- **WHEN** the CLI opens a fuzzy selector for account rename
- **THEN** it can search and display account labels together with signer-owner and network metadata using plaintext stored data
- **AND** does not decrypt private payloads just to populate the selector

#### Scenario: Address reveal decrypts only on explicit request
- **WHEN** the CLI lists accounts with explicit address reveal enabled
- **THEN** it decrypts the relevant source-specific account private payloads under the owning signer owner or imported vault domain
- **AND** includes addresses in the displayed output

#### Scenario: Rename address reveal requires bounded owner scope
- **WHEN** the CLI opens an account-rename fuzzy selector with address display enabled
- **THEN** it requires enough signer-owner or imported-vault scope to avoid prompting across unbounded password domains
- **AND** does not attempt to decrypt account addresses across unrelated owner domains without explicit user selection

### Requirement: Recovered accounts can be imported idempotently by derivation tuple
The account storage layer SHALL support importing recovered confirmed accounts for a resolved `(network_genesis_hash, signer_owner_id, ip_identity, identity_index, credential_counter)` tuple when recovery is supported by the signer owner kind. Recovery import SHALL avoid creating duplicate rows for an already-known tuple and SHALL preserve the existing label when the tuple already exists locally.

#### Scenario: New recovered account is inserted as confirmed
- **WHEN** recovery finds an account whose `(network_genesis_hash, signer_owner_id, ip_identity, identity_index, credential_counter)` tuple is not yet stored
- **THEN** the store inserts a confirmed account row for that tuple
- **AND** stores the recovered account address only inside the encrypted derived account private payload

#### Scenario: Existing recovered account tuple is reused
- **WHEN** recovery finds an account whose `(network_genesis_hash, signer_owner_id, ip_identity, identity_index, credential_counter)` tuple already exists locally
- **THEN** the store does not create a duplicate row
- **AND** preserves the existing local label for that account

### Requirement: Account records distinguish derived and imported sources
The account storage layer SHALL represent whether each account is backed by signer-owner derivation or imported secret material. Derived accounts SHALL retain their network, signer owner, identity provider, identity index, and credential counter metadata. Imported accounts SHALL reference imported account vault-backed secret material instead of requiring signer-owner derivation metadata.

#### Scenario: Existing derived account metadata is represented by signer owner tuple
- **WHEN** the wallet stores a derived account
- **THEN** the account is associated with its `network_genesis_hash`, `signer_owner_id`, `ip_identity`, `identity_index`, and `credential_counter`
- **AND** the account source is recorded as derived

#### Scenario: Imported account does not require derivation tuple
- **WHEN** the wallet stores an imported account
- **THEN** the account source is recorded as imported
- **AND** the account does not require a signer owner id, identity provider id, identity index, or credential counter to be present

### Requirement: Account deletion cascades source-specific private data
The account storage layer SHALL remove source-specific private payload rows when an account row is deleted. Deleting a signer owner SHALL cascade derived accounts owned by that signer owner, but SHALL NOT delete imported accounts. Pruning a network partition SHALL delete all accounts on that network, including imported accounts and imported account payloads.

#### Scenario: Deleting imported account deletes imported payload
- **WHEN** an imported account metadata row is deleted
- **THEN** its encrypted imported account payload row is deleted automatically

#### Scenario: Deleting signer owner leaves imported accounts intact
- **WHEN** a signer owner is deleted
- **THEN** derived accounts owned by that signer owner are deleted
- **AND** imported accounts on the same networks remain stored

#### Scenario: Network prune deletes imported accounts and payloads
- **WHEN** the storage layer deletes all account rows for a network genesis hash
- **THEN** imported accounts for that network are removed
- **AND** their encrypted imported account payloads are removed
