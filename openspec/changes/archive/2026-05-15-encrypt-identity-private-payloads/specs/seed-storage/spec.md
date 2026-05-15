## MODIFIED Requirements

### Requirement: Each seed phrase is its own independently password-protected encryption domain
The CLI SHALL generate a unique Data Encryption Key (DEK) per seed and protect it with a Key Encryption Key (KEK) derived from the user-supplied password using Argon2id. Unlocking one seed SHALL NOT require the password of any other seed. The seed DEK SHALL also serve as the encryption key for private objects owned by that seed, including identity private payloads, with object-specific AEAD AAD separation.

#### Scenario: Seed unlocked with correct password
- **WHEN** the user provides the correct password for seed `"main_seed"`
- **THEN** the CLI derives the KEK from the password and stored KDF parameters
- **AND** decrypts the DEK from `seed_vaults`
- **AND** uses the DEK to decrypt the seed payload
- **AND** returns the plaintext seed secret to the operation that requested it

#### Scenario: Seed unlock context supports seed-owned private payload encryption
- **WHEN** an operation unlocks seed `"main_seed"` with the correct password
- **THEN** the operation can use the seed's DEK to encrypt and decrypt private payloads owned by that seed
- **AND** each payload encryption uses object-specific AAD distinct from seed phrase encryption AAD

#### Scenario: Wrong password rejected
- **WHEN** the user provides an incorrect password for a seed
- **THEN** the CLI fails to authenticate and returns an error
- **AND** no plaintext seed material or seed-owned private payload data is exposed

#### Scenario: Unlocking one seed does not expose another seed
- **WHEN** the user unlocks seed `"main_seed"` with its password
- **THEN** the secret payload and seed-owned private payloads of any other seed (e.g., `"cold_seed"`) remain encrypted
- **AND** reading another seed's payload requires that seed's own password

### Requirement: Future seed-owned rows cascade on delete
Future DB tables that store objects owned by a seed SHALL reference `seeds(id)` with `ON DELETE CASCADE` unless explicitly justified otherwise. Identity rows SHALL be treated as seed-owned rows and cascade when the owning seed is deleted.

#### Scenario: Add future seed-owned table
- **WHEN** a future migration adds a seed-owned table such as accounts, identities, or credentials
- **THEN** its seed foreign key uses `REFERENCES seeds(id) ON DELETE CASCADE`

#### Scenario: Deleting seed deletes owned identities
- **WHEN** a seed row is deleted
- **THEN** identity rows owned by that seed are deleted automatically by SQLite foreign-key cascade
- **AND** encrypted private payload rows for those identities are also deleted automatically
