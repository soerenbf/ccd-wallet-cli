## ADDED Requirements

### Requirement: Seed deletion cascades to seed vault
The seed storage layer SHALL delete seed-owned vault rows when a seed is deleted. The `seed_vaults.seed_id` foreign key SHALL reference `seeds(id) ON DELETE CASCADE`.

#### Scenario: Deleting seed deletes vault
- **WHEN** a seed row is deleted from `seeds`
- **THEN** the corresponding row in `seed_vaults` is deleted automatically by SQLite foreign-key cascade

### Requirement: Seed removal by label
The seed storage layer SHALL provide a remove-by-label operation that deletes the seed row for a configured seed and reports an error for an unknown seed label.

#### Scenario: Remove configured seed by label
- **WHEN** the storage layer removes seed label `main_seed`
- **AND** `main_seed` exists
- **THEN** the seed row is deleted
- **AND** the operation succeeds

#### Scenario: Remove unknown seed by label
- **WHEN** the storage layer removes seed label `missing_seed`
- **AND** no such seed exists
- **THEN** the operation returns an error indicating that the seed is not configured

### Requirement: Future seed-owned rows cascade on delete
Future DB tables that store objects owned by a seed SHALL reference `seeds(id)` with `ON DELETE CASCADE` unless explicitly justified otherwise.

#### Scenario: Add future seed-owned table
- **WHEN** a future migration adds a seed-owned table such as accounts, identities, or credentials
- **THEN** its seed foreign key uses `REFERENCES seeds(id) ON DELETE CASCADE`
