## MODIFIED Requirements

### Requirement: Seed phrase has a plaintext label and an encrypted secret payload
The CLI SHALL store each seed phrase as a seed-kind signer owner. The signer owner SHALL contain the plaintext label and timestamps, `signer_owner_vaults` SHALL contain KDF parameters, salt, encrypted DEK, and DEK nonce, and `seed_owner_secrets` SHALL contain the encrypted seed secret payload. The signer-owner label SHALL be unique across all signer owners.

Renaming a seed SHALL update only the signer owner's plaintext label. The signer owner's stable `id`, vault row, and encrypted seed secret payload row SHALL remain unchanged.

#### Scenario: Seed phrase stored with a unique signer owner label
- **WHEN** the user adds a seed phrase with label `"main_seed"`
- **THEN** a seed-kind signer owner is inserted with the given label
- **AND** a corresponding signer-owner vault is inserted with encrypted DEK metadata
- **AND** a corresponding seed owner secret row is inserted with the encrypted seed payload
- **AND** the plaintext mnemonic or seed entropy is not stored anywhere in the clear

#### Scenario: Duplicate signer owner label rejected
- **WHEN** the user attempts to add a seed phrase with a label already present on any signer owner
- **THEN** the CLI rejects the operation with an error
- **AND** no new signer owner, vault, or seed secret rows are inserted

#### Scenario: Renaming seed preserves stable signer owner id
- **WHEN** the user renames seed label `main_seed` to `daily`
- **THEN** the signer owner's `label` value changes to `daily`
- **AND** the signer owner's `id` remains unchanged
- **AND** the encrypted seed payload remains linked to the same signer owner

### Requirement: Each seed phrase is its own independently password-protected encryption domain
The CLI SHALL generate a unique Data Encryption Key (DEK) per seed-kind signer owner and protect it with a Key Encryption Key (KEK) derived from the user-supplied password using Argon2id. Unlocking one seed signer owner SHALL NOT require the password of any other signer owner. The signer-owner DEK SHALL also serve as the encryption key for private objects owned by that signer owner, including identity private payloads and derived account private payloads, with object-specific AEAD AAD separation.

#### Scenario: Seed signer owner unlocked with correct password
- **WHEN** the user provides the correct password for seed signer owner `"main_seed"`
- **THEN** the CLI derives the KEK from the password and stored KDF parameters
- **AND** decrypts the DEK from `signer_owner_vaults`
- **AND** uses the DEK to decrypt the seed owner secret payload
- **AND** returns the plaintext seed secret to the operation that requested it

#### Scenario: Seed unlock context supports signer-owned private payload encryption
- **WHEN** an operation unlocks seed signer owner `"main_seed"` with the correct password
- **THEN** the operation can use the signer-owner DEK to encrypt and decrypt private payloads owned by that signer owner
- **AND** each payload encryption uses object-specific AAD distinct from seed secret encryption AAD

#### Scenario: Wrong password rejected
- **WHEN** the user provides an incorrect password for a seed signer owner
- **THEN** the CLI fails to authenticate and returns an error
- **AND** no plaintext seed material or signer-owned private payload data is exposed

#### Scenario: Unlocking one seed does not expose another signer owner
- **WHEN** the user unlocks seed signer owner `"main_seed"` with its password
- **THEN** the secret payload and signer-owned private payloads of any other signer owner remain encrypted
- **AND** reading another signer owner's payloads requires that signer owner's own password

### Requirement: Future seed-owned rows cascade on delete
DB tables that store objects owned by a seed-kind signer owner SHALL reference `signer_owners(id)` with `ON DELETE CASCADE` unless explicitly justified otherwise. Identity rows and derived account rows SHALL be treated as signer-owner-owned rows and cascade when the owning seed signer owner is deleted.

#### Scenario: Add future signer-owned table
- **WHEN** a future schema adds a table such as accounts, identities, or credentials owned by seed and Ledger derivation authorities
- **THEN** its owner foreign key uses `REFERENCES signer_owners(id) ON DELETE CASCADE`

#### Scenario: Deleting seed signer owner deletes owned identities and accounts
- **WHEN** a seed-kind signer owner row is deleted
- **THEN** identity rows owned by that signer owner are deleted automatically by SQLite foreign-key cascade
- **AND** derived account rows owned by that signer owner are deleted automatically by SQLite foreign-key cascade
- **AND** encrypted private payload rows for those identities and accounts are also deleted automatically
