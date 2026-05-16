## MODIFIED Requirements

### Requirement: Seed removal by label
The seed storage layer SHALL continue to support remove-by-label deletion semantics for a configured seed, and the user-facing CLI contract for this operation SHALL now be exposed as `seed delete`.

#### Scenario: Delete configured seed by label
- **WHEN** the storage layer deletes seed label `main_seed`
- **AND** `main_seed` exists
- **THEN** the seed row is deleted
- **AND** the operation succeeds

#### Scenario: Delete unknown seed by label
- **WHEN** the storage layer deletes seed label `missing_seed`
- **AND** no such seed exists
- **THEN** the operation returns an error indicating that the seed is not configured

### Requirement: Future seed-owned rows cascade on delete
Future DB tables that store objects owned by a seed SHALL reference `seeds(id)` with `ON DELETE CASCADE` unless explicitly justified otherwise. Identity rows and account rows SHALL be treated as seed-owned rows and cascade when the owning seed is deleted.

#### Scenario: Deleting seed deletes owned identities and accounts
- **WHEN** a seed row is deleted
- **THEN** identity rows owned by that seed are deleted automatically by SQLite foreign-key cascade
- **AND** account rows owned by that seed are deleted automatically by SQLite foreign-key cascade
- **AND** encrypted private payload rows for those identities and accounts are also deleted automatically
