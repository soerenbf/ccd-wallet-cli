## MODIFIED Requirements

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

## ADDED Requirements

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
