## ADDED Requirements

### Requirement: Recovered accounts can be imported idempotently by derivation tuple
The account storage layer SHALL support importing recovered confirmed accounts for a resolved `(network_genesis_hash, seed_id, ip_identity, identity_index, credential_counter)` tuple. Recovery import SHALL avoid creating duplicate rows for an already-known tuple and SHALL preserve the existing label when the tuple already exists locally.

#### Scenario: New recovered account is inserted as confirmed
- **WHEN** recovery finds an account whose `(network_genesis_hash, seed_id, ip_identity, identity_index, credential_counter)` tuple is not yet stored
- **THEN** the store inserts a confirmed account row for that tuple
- **AND** stores the recovered account address only inside the encrypted private payload

#### Scenario: Existing recovered account tuple is reused
- **WHEN** recovery finds an account whose `(network_genesis_hash, seed_id, ip_identity, identity_index, credential_counter)` tuple already exists locally
- **THEN** the store does not create a duplicate row
- **AND** preserves the existing local label for that account
