## MODIFIED Requirements

### Requirement: Identity objects are persisted in SQLite
The system SHALL store issued identities in the local SQLite database as plaintext public metadata plus encrypted private payload data. Public metadata SHALL include the network identity (genesis hash), owning signer owner id, user-supplied identity label, identity provider index, identity index, issuance status, creation timestamp, and plaintext identity usability metadata required for account creation, including identity expiry. Private payload data SHALL include the `code_uri` and issued identity object and SHALL be encrypted under the owning signer owner's password domain. The `(network_genesis_hash, label)` pair SHALL be unique; the `(network_genesis_hash, signer_owner_id, ip_identity, identity_index)` tuple SHALL also be unique.

Identity labels SHALL be queryable and updatable by scoped label without changing the row's underlying `(network_genesis_hash, signer_owner_id, ip_identity, identity_index)` identity.

#### Scenario: New identity row created when issuance starts
- **WHEN** the issuance flow receives a `code_uri`
- **THEN** a new identity metadata row is inserted with status `pending`
- **AND** the plaintext `code_uri` is not stored in the identity metadata row
- **AND** an encrypted private payload row is inserted containing the `code_uri`

#### Scenario: Identity row updated to done on successful poll
- **WHEN** polling returns status `done`
- **THEN** the identity metadata row is updated with status `done`
- **AND** plaintext usability metadata needed for account creation, including identity expiry, is extracted and stored on the identity metadata row
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

#### Scenario: Duplicate (network, signer owner, IP, identity_index) is rejected
- **WHEN** an identity row with the same `(network_genesis_hash, signer_owner_id, ip_identity, identity_index)` tuple already exists
- **THEN** the store layer returns an error before contacting the identity provider

#### Scenario: Renaming identity preserves underlying tuple
- **WHEN** the user renames an identity label within a network scope
- **THEN** only the plaintext `label` changes
- **AND** the `(network_genesis_hash, signer_owner_id, ip_identity, identity_index)` tuple remains unchanged

### Requirement: Identity index is auto-assigned per (network, seed, IP) tuple
The system SHALL assign the next available identity index (starting at 0) for a given `(network_genesis_hash, signer_owner_id, ip_identity)` tuple when creating a new identity row. Seed-backed and Ledger-backed signer owners SHALL each have independent identity index sequences.

#### Scenario: First identity for a network+signer-owner+IP gets index 0
- **WHEN** no prior identity exists for the given network, signer owner, and IP
- **THEN** the assigned identity index is 0

#### Scenario: Subsequent identity for same network+signer-owner+IP increments index
- **WHEN** one identity already exists for the given network, signer owner, and IP
- **THEN** the assigned identity index is 1

### Requirement: Identity private payloads are encrypted under the owning seed password domain
The identity storage layer SHALL encrypt identity private payloads using the owning signer owner's DEK after that DEK has been unlocked by the signer owner's local password. Identity private payload encryption SHALL use ChaCha20-Poly1305 with a unique nonce and AAD binding the ciphertext to the identity row and signer-owner ownership metadata.

#### Scenario: Private payload ciphertext does not reveal personal data
- **WHEN** an identity is issued successfully
- **THEN** the database stores the identity object only as ciphertext and nonce
- **AND** reading the database directly does not reveal the plaintext identity object JSON

#### Scenario: Correct signer owner password decrypts private payload
- **WHEN** the user unlocks the owning signer owner with the correct local password
- **THEN** the system can decrypt the identity private payload for that signer owner's identities

#### Scenario: Wrong signer owner password cannot decrypt private payload
- **WHEN** the user supplies an incorrect password for the owning signer owner
- **THEN** identity private payload decryption fails
- **AND** no plaintext `code_uri` or identity object is exposed

#### Scenario: AAD prevents private payload transplantation
- **WHEN** an encrypted identity private payload is copied to another identity row or signer-owner context
- **THEN** AEAD authentication fails during decryption

### Requirement: Identity private payloads cascade with identities
The `identity_private_payloads.identity_id` foreign key SHALL reference `identities(id) ON DELETE CASCADE`. Identity rows SHALL reference `signer_owners(id) ON DELETE CASCADE`, so deleting a signer owner deletes its identities and their private payload rows.

#### Scenario: Deleting identity deletes private payload
- **WHEN** an identity row is deleted
- **THEN** its encrypted private payload row is deleted automatically by SQLite foreign-key cascade

#### Scenario: Deleting signer owner deletes identity private payloads
- **WHEN** a signer owner row is deleted
- **THEN** identities owned by the signer owner are deleted automatically
- **AND** encrypted private payload rows for those identities are deleted automatically

#### Scenario: Pruning a network partition deletes identities and payloads
- **WHEN** the storage layer deletes all identity rows whose `network_genesis_hash = abc`
- **THEN** all matching identity rows are removed
- **AND** their encrypted private payload rows are deleted automatically by SQLite foreign-key cascade

### Requirement: Identity rows are listable and searchable by plaintext metadata without decrypting private payloads
The system SHALL support listing identities by network scope, signer-owner scope, and relevant plaintext filters such as identity provider id using plaintext metadata only. Listing and interactive search for identities SHALL NOT require decrypting private identity payloads.

#### Scenario: List identities for signer owner and network scope
- **WHEN** the CLI lists identities for a resolved signer owner and network scope
- **THEN** it reads the identity rows using plaintext metadata
- **AND** does not decrypt the private payloads just to produce the list

#### Scenario: Filter identities by provider id
- **WHEN** the CLI lists identities with an identity provider filter
- **THEN** it filters identity rows using the stored plaintext `ip_identity` metadata
- **AND** does not decrypt private identity payloads just to apply the filter

#### Scenario: Fuzzy rename search uses plaintext metadata
- **WHEN** the CLI opens a fuzzy selector for identity rename
- **THEN** it can search and display identity labels together with signer-owner and network metadata using plaintext stored data
- **AND** does not decrypt private identity payloads just to populate the selector

### Requirement: Recovered identities can be imported idempotently by tuple
The identity storage layer SHALL support importing completed recovered identities for a resolved `(network_genesis_hash, signer_owner_id, ip_identity, identity_index)` tuple when recovery is supported by the signer owner kind. Recovery import SHALL avoid creating duplicate rows for an already-known tuple and SHALL preserve the existing label when the tuple already exists locally.

#### Scenario: New recovered identity is inserted as completed
- **WHEN** recovery finds an identity whose `(network_genesis_hash, signer_owner_id, ip_identity, identity_index)` tuple is not yet stored
- **THEN** the store inserts a completed identity row for that tuple
- **AND** stores the recovered identity object only inside the encrypted private payload

#### Scenario: Existing recovered identity tuple is reused
- **WHEN** recovery finds an identity whose `(network_genesis_hash, signer_owner_id, ip_identity, identity_index)` tuple already exists locally
- **THEN** the store does not create a duplicate row
- **AND** preserves the existing local label for that identity
