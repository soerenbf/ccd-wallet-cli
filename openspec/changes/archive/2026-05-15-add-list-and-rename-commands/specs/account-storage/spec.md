## MODIFIED Requirements

### Requirement: Accounts are persisted as plaintext metadata plus encrypted private payloads
The system SHALL store wallet-managed accounts in the local SQLite database as plaintext relational metadata plus encrypted private payload data under the owning seed password domain. Plaintext metadata SHALL include the network identity (`network_genesis_hash`), owning seed id, identity provider index, identity index, credential counter, user-supplied account label, lifecycle status, and timestamps. Private payload data SHALL be stored as a structured `AccountPrivatePayload` object rather than as a bare encrypted address string.

In the initial version, the encrypted payload SHALL contain the account address. The payload structure MUST be extensible so future encrypted account details can be added without redesigning the storage model. Account labels SHALL be queryable and updatable by scoped label without changing the account's underlying derivation tuple.

#### Scenario: New account row stores plaintext indexing metadata
- **WHEN** the wallet creates a new pending account record
- **THEN** the plaintext account row stores `network_genesis_hash`, `seed_id`, `ip_identity`, `identity_index`, `credential_counter`, label, status, and timestamps
- **AND** the account address is not stored in plaintext columns

#### Scenario: Encrypted account payload stores structured account data
- **WHEN** the wallet stores private account data
- **THEN** it serializes an `AccountPrivatePayload` structure
- **AND** encrypts that structure under the owning seed password domain
- **AND** does not encrypt the address as a standalone primitive value

#### Scenario: Renaming account preserves derivation tuple
- **WHEN** the user renames an account label within a network scope
- **THEN** only the plaintext `label` changes
- **AND** the `(network_genesis_hash, seed_id, ip_identity, identity_index, credential_counter)` tuple remains unchanged

## ADDED Requirements

### Requirement: Account rows are listable and searchable by plaintext metadata with optional address reveal
The system SHALL support listing accounts by network scope, seed scope, and relevant plaintext filters such as status using plaintext metadata. Account addresses remain encrypted by default and SHALL only be revealed when the CLI explicitly requests address display. Listing and interactive search for accounts SHALL use plaintext metadata unless address reveal has been explicitly requested.

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
- **THEN** it can search and display account labels together with seed and network metadata using plaintext stored data
- **AND** does not decrypt private payloads just to populate the selector

#### Scenario: Address reveal decrypts only on explicit request
- **WHEN** the CLI lists accounts with explicit address reveal enabled
- **THEN** it decrypts the relevant account private payloads under the owning seed domain
- **AND** includes addresses in the displayed output

#### Scenario: Rename address reveal requires single-seed scope
- **WHEN** the CLI opens an account-rename fuzzy selector with address display enabled
- **THEN** it requires a single resolved seed scope before decrypting addresses for selector rows
- **AND** does not attempt to decrypt account addresses across multiple seed domains in one rename search
