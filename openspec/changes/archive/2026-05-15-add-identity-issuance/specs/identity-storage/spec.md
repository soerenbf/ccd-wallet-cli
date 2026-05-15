## ADDED Requirements

### Requirement: Identity objects are persisted in SQLite
The system SHALL store issued identity objects in the local SQLite database, associated with the network identity (genesis hash), seed label, user-supplied identity label, identity provider index, identity index, and issuance status. The `(network_genesis_hash, label)` pair SHALL be unique; the `(network_genesis_hash, seed_label, ip_identity, identity_index)` tuple SHALL also be unique.

#### Scenario: New identity row created when issuance starts
- **WHEN** the issuance flow begins and a `code_uri` is received
- **THEN** a new identity row is inserted with status `pending` and the `code_uri` stored

#### Scenario: Identity row updated to done on successful poll
- **WHEN** polling returns status `done`
- **THEN** the identity row is updated: status set to `done`, `identity_object` JSON stored

#### Scenario: Identity row updated to error on failed poll
- **WHEN** polling returns status `error`
- **THEN** the identity row is updated: status set to `error`

#### Scenario: Duplicate label within same network is rejected
- **WHEN** an identity with the same `(network_genesis_hash, label)` already exists
- **THEN** the store layer returns an error

#### Scenario: Same label on different networks is allowed
- **WHEN** two identities use the same `label` but different `network_genesis_hash` values
- **THEN** both identities can be stored

#### Scenario: Duplicate (network, seed, IP, identity_index) is rejected
- **WHEN** an identity row with the same `(network_genesis_hash, seed_label, ip_identity, identity_index)` tuple already exists
- **THEN** the store layer returns an error before contacting the identity provider

### Requirement: Identity index is auto-assigned per (network, seed, IP) tuple
The system SHALL assign the next available identity index (starting at 0) for a given `(network_genesis_hash, seed_label, ip_identity)` tuple when creating a new identity row.

#### Scenario: First identity for a network+seed+IP gets index 0
- **WHEN** no prior identity exists for the given network, seed, and IP
- **THEN** the assigned identity index is 0

#### Scenario: Subsequent identity for same network+seed+IP increments index
- **WHEN** one identity already exists for the given network, seed, and IP
- **THEN** the assigned identity index is 1
