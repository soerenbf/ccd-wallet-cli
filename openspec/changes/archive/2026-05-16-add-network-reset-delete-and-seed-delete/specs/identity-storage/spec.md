## MODIFIED Requirements

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
