# identity-storage Specification

## Purpose
TBD - created by archiving change add-identity-issuance. Update Purpose after archive.
## Requirements
### Requirement: Identity objects are persisted in SQLite
The system SHALL store issued identities in the local SQLite database as plaintext public metadata plus encrypted private payload data. Public metadata SHALL include the network identity (genesis hash), owning seed id, user-supplied identity label, identity provider index, identity index, issuance status, creation timestamp, and plaintext identity usability metadata required for account creation, including identity expiry. Private payload data SHALL include the `code_uri` and issued identity object and SHALL be encrypted under the owning seed's password domain. The `(network_genesis_hash, label)` pair SHALL be unique; the `(network_genesis_hash, seed_id, ip_identity, identity_index)` tuple SHALL also be unique.

Identity labels SHALL be queryable and updatable by scoped label without changing the row's underlying `(network_genesis_hash, seed_id, ip_identity, identity_index)` identity.

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

#### Scenario: Duplicate (network, seed, IP, identity_index) is rejected
- **WHEN** an identity row with the same `(network_genesis_hash, seed_id, ip_identity, identity_index)` tuple already exists
- **THEN** the store layer returns an error before contacting the identity provider

#### Scenario: Renaming identity preserves underlying tuple
- **WHEN** the user renames an identity label within a network scope
- **THEN** only the plaintext `label` changes
- **AND** the `(network_genesis_hash, seed_id, ip_identity, identity_index)` tuple remains unchanged

### Requirement: Identity index is auto-assigned per (network, seed, IP) tuple
The system SHALL assign the next available identity index (starting at 0) for a given `(network_genesis_hash, seed_id, ip_identity)` tuple when creating a new identity row.

#### Scenario: First identity for a network+seed+IP gets index 0
- **WHEN** no prior identity exists for the given network, seed, and IP
- **THEN** the assigned identity index is 0

#### Scenario: Subsequent identity for same network+seed+IP increments index
- **WHEN** one identity already exists for the given network, seed, and IP
- **THEN** the assigned identity index is 1

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

#### Scenario: Pruning a network partition deletes identities and payloads
- **WHEN** the storage layer deletes all identity rows whose `network_genesis_hash = abc`
- **THEN** all matching identity rows are removed
- **AND** their encrypted private payload rows are deleted automatically by SQLite foreign-key cascade

### Requirement: Identity usability metadata supports account creation prevalidation
The system SHALL persist enough plaintext identity usability metadata to let account creation determine whether an identity is eligible for use without decrypting all stored identity payloads. For this change, the only plaintext usability metadata promoted for account creation is identity expiry. This metadata MUST allow the wallet to reject expired identities during identity selection and again immediately before credential deployment submission.

#### Scenario: Expired identities are discoverable without decrypting private payloads
- **WHEN** the wallet prepares a list of candidate identities for account creation
- **THEN** it can determine from plaintext identity metadata whether each identity is expired
- **AND** it does not need to decrypt every stored identity payload first

#### Scenario: Identity with missing usability metadata cannot be used for account creation
- **WHEN** an identity does not have the required plaintext usability metadata for account creation
- **THEN** the wallet treats that identity as unusable for account creation
- **AND** reports an actionable error instead of attempting transaction submission

### Requirement: Pending identities support deferred completion
The system SHALL support identities remaining in `pending` status after issuance initiation and SHALL use the stored encrypted issuance state to complete them later.

Identity issuance SHALL wait for provider completion by default. Identity issuance SHALL also support an explicit skip-wait option that returns after the browser callback has provided `code_uri`, leaving the identity in `pending` status for later completion. If account creation attempts to use an identity that is still marked `pending`, the system SHALL use the stored encrypted issuance state to perform a lazy confirmation check with the identity provider before deciding whether the identity can be used.

If the provider now reports `done`, the wallet SHALL update the local identity record to `done`, persist any required plaintext expiry metadata, and continue account creation. If the provider still reports `pending`, the wallet SHALL leave the identity pending and stop account creation with an actionable message. If the provider reports `error`, the wallet SHALL surface the provider error and SHALL NOT proceed with account creation.

#### Scenario: Identity issuance waits for completion by default
- **WHEN** the user runs identity issuance without a skip-wait flag
- **THEN** the wallet continues polling the provider after receiving `code_uri`
- **AND** completes the identity flow only when the provider returns `done` or `error`

#### Scenario: Identity issuance can skip waiting after callback
- **WHEN** the user runs identity issuance with a skip-wait flag
- **AND** the browser callback has provided `code_uri`
- **THEN** the wallet stores the pending identity state
- **AND** returns without waiting for provider completion
- **AND** leaves the identity record in `pending` status

#### Scenario: Pending identity becomes done during lazy confirmation
- **WHEN** account creation selects an identity with local status `pending`
- **AND** the provider now reports `done`
- **THEN** the wallet updates the identity record to `done`
- **AND** persists plaintext expiry metadata needed for account creation
- **AND** allows account creation to continue

#### Scenario: Pending identity remains pending during lazy confirmation
- **WHEN** account creation selects an identity with local status `pending`
- **AND** the provider still reports `pending`
- **THEN** the wallet leaves the identity record pending
- **AND** stops account creation with an actionable message

#### Scenario: Pending identity returns provider error during lazy confirmation
- **WHEN** account creation selects an identity with local status `pending`
- **AND** the provider reports `error`
- **THEN** the wallet surfaces the provider error
- **AND** does not proceed with account creation

### Requirement: Identity rows are listable and searchable by plaintext metadata without decrypting private payloads
The system SHALL support listing identities by network scope, seed scope, and relevant plaintext filters such as identity provider id using plaintext metadata only. Listing and interactive search for identities SHALL NOT require decrypting private identity payloads.

#### Scenario: List identities for seed and network scope
- **WHEN** the CLI lists identities for a resolved seed and network scope
- **THEN** it reads the identity rows using plaintext metadata
- **AND** does not decrypt the private payloads just to produce the list

#### Scenario: Filter identities by provider id
- **WHEN** the CLI lists identities with an identity provider filter
- **THEN** it filters identity rows using the stored plaintext `ip_identity` metadata
- **AND** does not decrypt private identity payloads just to apply the filter

#### Scenario: Fuzzy rename search uses plaintext metadata
- **WHEN** the CLI opens a fuzzy selector for identity rename
- **THEN** it can search and display identity labels together with seed and network metadata using plaintext stored data
- **AND** does not decrypt private identity payloads just to populate the selector

### Requirement: Recovered identities can be imported idempotently by tuple
The identity storage layer SHALL support importing completed recovered identities for a resolved `(network_genesis_hash, seed_id, ip_identity, identity_index)` tuple. Recovery import SHALL avoid creating duplicate rows for an already-known tuple and SHALL preserve the existing label when the tuple already exists locally.

#### Scenario: New recovered identity is inserted as completed
- **WHEN** recovery finds an identity whose `(network_genesis_hash, seed_id, ip_identity, identity_index)` tuple is not yet stored
- **THEN** the store inserts a completed identity row for that tuple
- **AND** stores the recovered identity object only inside the encrypted private payload

#### Scenario: Existing recovered identity tuple is reused
- **WHEN** recovery finds an identity whose `(network_genesis_hash, seed_id, ip_identity, identity_index)` tuple already exists locally
- **THEN** the store does not create a duplicate row
- **AND** preserves the existing local label for that identity

