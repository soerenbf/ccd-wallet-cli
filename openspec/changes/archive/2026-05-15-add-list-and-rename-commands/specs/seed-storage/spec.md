## MODIFIED Requirements

### Requirement: Seed phrase has a plaintext label and an encrypted secret payload
The CLI SHALL store each seed phrase as a row in the `seeds` table (plaintext label, timestamps) and a corresponding row in the `seed_vaults` table (KDF parameters, salt, encrypted DEK, encrypted payload). The label SHALL be unique across all seeds.

Renaming a seed SHALL update only the plaintext label. The seed's stable `id` and encrypted payload rows SHALL remain unchanged.

#### Scenario: Seed phrase stored with a unique label
- **WHEN** the user adds a seed phrase with label `"main_seed"`
- **THEN** a row is inserted into `seeds` with the given label
- **AND** a corresponding row is inserted into `seed_vaults` with the encrypted payload
- **AND** the plaintext mnemonic or seed entropy is not stored anywhere in the clear

#### Scenario: Duplicate seed label rejected
- **WHEN** the user attempts to add a seed phrase with a label already present in `seeds`
- **THEN** the CLI rejects the operation with an error
- **AND** no new rows are inserted

#### Scenario: Renaming seed preserves stable id
- **WHEN** the user renames seed label `main_seed` to `daily`
- **THEN** the `seeds.label` value changes to `daily`
- **AND** the seed's `id` remains unchanged
- **AND** the encrypted seed payload remains linked to the same seed row

## ADDED Requirements

### Requirement: Seed labels are queryable and listable without a password
The CLI SHALL be able to list configured seed labels and their plaintext metadata without prompting for a password, and it SHALL be able to resolve a seed by label for rename operations without decrypting the seed payload.

#### Scenario: List seeds without password
- **WHEN** the user runs `seed list`
- **THEN** the CLI displays the configured seed labels and available plaintext metadata
- **AND** no password is requested

#### Scenario: Rename seed resolves by plaintext label
- **WHEN** the user runs `seed rename old new`
- **THEN** the storage layer resolves the source seed by plaintext label
- **AND** no seed payload decryption is required just to perform the rename
