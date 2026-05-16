## ADDED Requirements

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
