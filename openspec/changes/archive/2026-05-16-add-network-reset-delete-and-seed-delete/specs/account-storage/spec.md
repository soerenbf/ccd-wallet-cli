## MODIFIED Requirements

### Requirement: Account private payloads cascade with account rows and owning seeds
The encrypted account private payload table SHALL reference the account metadata row with `ON DELETE CASCADE`. Account rows owned by a seed SHALL also be removed when the owning seed is deleted so that encrypted account payloads do not outlive their seed domain.

#### Scenario: Deleting account deletes private payload
- **WHEN** an account metadata row is deleted
- **THEN** its encrypted private payload row is deleted automatically by SQLite foreign-key cascade

#### Scenario: Deleting seed deletes owned accounts and payloads
- **WHEN** a seed row is deleted
- **THEN** account rows owned by that seed are deleted automatically
- **AND** encrypted private payload rows for those accounts are deleted automatically

#### Scenario: Pruning a network partition deletes accounts and payloads
- **WHEN** the storage layer deletes all account rows whose `network_genesis_hash = abc`
- **THEN** all matching account rows are removed
- **AND** their encrypted private payload rows are deleted automatically by SQLite foreign-key cascade
