# seed-storage Specification

## Purpose
TBD - created by archiving change sqlite-wallet-store. Update Purpose after archive.
## Requirements
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

### Requirement: Envelope encryption uses Argon2id KDF and ChaCha20-Poly1305 AEAD
The CLI SHALL derive the KEK using Argon2id with stored parameters (m_cost, t_cost, p_cost) and a random 16-byte salt. The DEK and all seed payloads SHALL be encrypted with ChaCha20-Poly1305 using a unique random 12-byte nonce per encryption operation. All encryption operations SHALL include Associated Additional Data (AAD) binding the ciphertext to its object identity.

#### Scenario: KDF parameters and salt are stored per seed vault
- **WHEN** a seed is added
- **THEN** `seed_vaults` stores the Argon2id algorithm name, parameter JSON, and salt used for that seed's KEK derivation

#### Scenario: Nonces are unique per encryption operation
- **WHEN** two separate seed payloads are encrypted
- **THEN** each uses a distinct randomly-generated nonce
- **AND** both nonces are stored alongside their respective ciphertexts

#### Scenario: AAD prevents ciphertext transplantation
- **WHEN** a seed payload ciphertext from one seed vault row is used to attempt decryption in the context of a different seed's identity
- **THEN** the AEAD authentication fails and the operation returns an error

### Requirement: Password change re-encrypts only the DEK
The CLI SHALL support changing the password for a seed by deriving a new KEK from the new password and re-encrypting the existing DEK, without altering the seed payload ciphertext.

#### Scenario: Password changed successfully
- **WHEN** the user provides the current password and a new password for a seed
- **THEN** the CLI decrypts the DEK using the old KEK
- **AND** re-encrypts the DEK with a new KEK derived from the new password
- **AND** updates `seed_vaults` with the new salt, KDF params, and encrypted DEK
- **AND** the payload ciphertext row is unchanged

### Requirement: Schema is forward-compatible with encrypted child objects
The `seeds` table primary key (`id`) SHALL be designed as a stable reference point for future tables (`accounts`, `credentials`, `identities`) that will store encrypted objects scoped to a seed's encryption domain. No foreign key enforcement is required in this change, but the `id` column type and naming SHALL be consistent with future use.

#### Scenario: Seed ID is a stable UUID
- **WHEN** a seed is created
- **THEN** its `id` is a UUIDv4 string
- **AND** the `id` does not change if the seed's label is updated

### Requirement: Key material is zeroized after use
The CLI SHALL wrap all in-memory key material (DEK, KEK, plaintext seed secret) in types that explicitly overwrite the underlying memory when dropped. Key material SHALL NOT persist in heap memory beyond the scope of the operation that required it.

#### Scenario: DEK is zeroized after seed unlock
- **WHEN** a seed is unlocked and the operation completes
- **THEN** the DEK bytes are overwritten in memory before deallocation
- **AND** the plaintext seed secret bytes are overwritten in memory before deallocation

### Requirement: Seed list is queryable without a password
The CLI SHALL be able to list all seed labels and their creation timestamps without prompting for a password, as this data is stored in plaintext in the `seeds` table.

#### Scenario: List seeds without password
- **WHEN** the user runs a command to list seeds (e.g., `ccd-wallet show seeds`)
- **THEN** the CLI displays each seed's label and creation timestamp
- **AND** no password is requested

### Requirement: Seed deletion cascades to seed vault
The seed storage layer SHALL delete seed-owned vault rows when a seed is deleted. The `seed_vaults.seed_id` foreign key SHALL reference `seeds(id) ON DELETE CASCADE`.

#### Scenario: Deleting seed deletes vault
- **WHEN** a seed row is deleted from `seeds`
- **THEN** the corresponding row in `seed_vaults` is deleted automatically by SQLite foreign-key cascade

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
DB tables that store objects owned by a seed-kind signer owner SHALL reference `signer_owners(id)` with `ON DELETE CASCADE` unless explicitly justified otherwise. Identity rows and derived account rows SHALL be treated as signer-owner-owned rows and cascade when the owning seed signer owner is deleted.

#### Scenario: Add future signer-owned table
- **WHEN** a future schema adds a table such as accounts, identities, or credentials owned by seed and Ledger derivation authorities
- **THEN** its owner foreign key uses `REFERENCES signer_owners(id) ON DELETE CASCADE`

#### Scenario: Deleting seed signer owner deletes owned identities and accounts
- **WHEN** a seed-kind signer owner row is deleted
- **THEN** identity rows owned by that signer owner are deleted automatically by SQLite foreign-key cascade
- **AND** derived account rows owned by that signer owner are deleted automatically by SQLite foreign-key cascade
- **AND** encrypted private payload rows for those identities and accounts are also deleted automatically

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

