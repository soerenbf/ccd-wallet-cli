## ADDED Requirements

### Requirement: Accounts are persisted as plaintext metadata plus encrypted private payloads
The system SHALL store wallet-managed accounts in the local SQLite database as plaintext relational metadata plus encrypted private payload data under the owning seed password domain. Plaintext metadata SHALL include the network identity (`network_genesis_hash`), owning seed id, identity provider index, identity index, credential counter, user-supplied account label, lifecycle status, and timestamps. Private payload data SHALL be stored as a structured `AccountPrivatePayload` object rather than as a bare encrypted address string.

In the initial version, the encrypted payload SHALL contain the account address. The payload structure MUST be extensible so future encrypted account details can be added without redesigning the storage model.

#### Scenario: New account row stores plaintext indexing metadata
- **WHEN** the wallet creates a new pending account record
- **THEN** the plaintext account row stores `network_genesis_hash`, `seed_id`, `ip_identity`, `identity_index`, `credential_counter`, label, status, and timestamps
- **AND** the account address is not stored in plaintext columns

#### Scenario: Encrypted account payload stores structured account data
- **WHEN** the wallet stores private account data
- **THEN** it serializes an `AccountPrivatePayload` structure
- **AND** encrypts that structure under the owning seed password domain
- **AND** does not encrypt the address as a standalone primitive value

### Requirement: Account uniqueness follows the credential derivation tuple
The system SHALL enforce account uniqueness within the tuple `(network_genesis_hash, seed_id, ip_identity, identity_index, credential_counter)`. The `(network_genesis_hash, label)` pair SHALL also be unique for wallet-local naming.

#### Scenario: Duplicate credential counter for same identity tuple is rejected
- **WHEN** an account row already exists for a given `(network_genesis_hash, seed_id, ip_identity, identity_index, credential_counter)` tuple
- **THEN** inserting another account row for the same tuple fails

#### Scenario: Same label on different networks is allowed
- **WHEN** two account rows use the same label but different `network_genesis_hash` values
- **THEN** both account rows can be stored

### Requirement: Account private payloads are encrypted under the owning seed password domain
The account storage layer SHALL encrypt account private payloads using the owning seed's DEK after that DEK has been unlocked by the seed password. Account private payload encryption SHALL use the same seed-domain encryption model as identity private payloads, including unique nonces and AAD binding the ciphertext to the account row and ownership metadata.

#### Scenario: Correct seed password decrypts account payload
- **WHEN** the user unlocks the owning seed with the correct password
- **THEN** the system can decrypt that seed's account private payloads
- **AND** recover the structured `AccountPrivatePayload`

#### Scenario: Wrong seed password cannot decrypt account payload
- **WHEN** the user supplies an incorrect password for the owning seed
- **THEN** account private payload decryption fails
- **AND** no plaintext account address is exposed

#### Scenario: AAD prevents account payload transplantation
- **WHEN** an encrypted account private payload is copied to another account row or seed context
- **THEN** AEAD authentication fails during decryption

### Requirement: Account private payloads cascade with account rows and owning seeds
The encrypted account private payload table SHALL reference the account metadata row with `ON DELETE CASCADE`. Account rows owned by a seed SHALL also be removed when the owning seed is deleted so that encrypted account payloads do not outlive their seed domain.

#### Scenario: Deleting account deletes private payload
- **WHEN** an account metadata row is deleted
- **THEN** its encrypted private payload row is deleted automatically by SQLite foreign-key cascade

#### Scenario: Deleting seed deletes owned accounts and payloads
- **WHEN** a seed row is deleted
- **THEN** account rows owned by that seed are deleted automatically
- **AND** encrypted private payload rows for those accounts are deleted automatically
