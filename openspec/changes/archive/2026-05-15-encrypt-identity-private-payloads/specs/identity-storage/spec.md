## MODIFIED Requirements

### Requirement: Identity objects are persisted in SQLite
The system SHALL store issued identities in the local SQLite database as plaintext public metadata plus encrypted private payload data. Public metadata SHALL include the network identity (genesis hash), owning seed id, user-supplied identity label, identity provider index, identity index, issuance status, and creation timestamp. Private payload data SHALL include the `code_uri` and issued identity object and SHALL be encrypted under the owning seed's password domain. The `(network_genesis_hash, label)` pair SHALL be unique; the `(network_genesis_hash, seed_id, ip_identity, identity_index)` tuple SHALL also be unique.

#### Scenario: New identity row created when issuance starts
- **WHEN** the issuance flow receives a `code_uri`
- **THEN** a new identity metadata row is inserted with status `pending`
- **AND** the plaintext `code_uri` is not stored in the identity metadata row
- **AND** an encrypted private payload row is inserted containing the `code_uri`

#### Scenario: Identity row updated to done on successful poll
- **WHEN** polling returns status `done`
- **THEN** the identity metadata row is updated with status `done`
- **AND** the issued identity object JSON is stored only inside the encrypted private payload
- **AND** no plaintext identity object JSON is stored in SQLite

#### Scenario: Identity row deleted on failed poll
- **WHEN** polling returns status `error`
- **THEN** the pending identity row is deleted
- **AND** its encrypted private payload row is deleted by cascade

#### Scenario: Duplicate label within same network is rejected
- **WHEN** an identity with the same `(network_genesis_hash, label)` already exists
- **THEN** the store layer returns an error

#### Scenario: Same label on different networks is allowed
- **WHEN** two identities use the same `label` but different `network_genesis_hash` values
- **THEN** both identities can be stored

#### Scenario: Duplicate (network, seed, IP, identity_index) is rejected
- **WHEN** an identity row with the same `(network_genesis_hash, seed_id, ip_identity, identity_index)` tuple already exists
- **THEN** the store layer returns an error before contacting the identity provider

### Requirement: Identity index is auto-assigned per (network, seed, IP) tuple
The system SHALL assign the next available identity index (starting at 0) for a given `(network_genesis_hash, seed_id, ip_identity)` tuple when creating a new identity row.

#### Scenario: First identity for a network+seed+IP gets index 0
- **WHEN** no prior identity exists for the given network, seed, and IP
- **THEN** the assigned identity index is 0

#### Scenario: Subsequent identity for same network+seed+IP increments index
- **WHEN** one identity already exists for the given network, seed, and IP
- **THEN** the assigned identity index is 1

## ADDED Requirements

### Requirement: Identity private payloads are encrypted under the owning seed password domain
The identity storage layer SHALL encrypt identity private payloads using the owning seed's DEK after that DEK has been unlocked by the seed password. Identity private payload encryption SHALL use ChaCha20-Poly1305 with a unique nonce and AAD binding the ciphertext to the identity row and seed ownership metadata.

#### Scenario: Private payload ciphertext does not reveal personal data
- **WHEN** an identity is issued successfully
- **THEN** the database stores the identity object only as ciphertext and nonce
- **AND** reading the database directly does not reveal the plaintext identity object JSON

#### Scenario: Correct seed password decrypts private payload
- **WHEN** the user unlocks the owning seed with the correct password
- **THEN** the system can decrypt the identity private payload for that seed's identities

#### Scenario: Wrong seed password cannot decrypt private payload
- **WHEN** the user supplies an incorrect password for the owning seed
- **THEN** identity private payload decryption fails
- **AND** no plaintext `code_uri` or identity object is exposed

#### Scenario: AAD prevents private payload transplantation
- **WHEN** an encrypted identity private payload is copied to another identity row or seed context
- **THEN** AEAD authentication fails during decryption

### Requirement: Identity private payloads cascade with identities
The `identity_private_payloads.identity_id` foreign key SHALL reference `identities(id) ON DELETE CASCADE`.

#### Scenario: Deleting identity deletes private payload
- **WHEN** an identity row is deleted
- **THEN** its encrypted private payload row is deleted automatically by SQLite foreign-key cascade

#### Scenario: Deleting seed deletes identity private payloads
- **WHEN** a seed row is deleted
- **THEN** identities owned by the seed are deleted automatically
- **AND** encrypted private payload rows for those identities are deleted automatically
