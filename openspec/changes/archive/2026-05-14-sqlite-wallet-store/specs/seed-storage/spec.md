## ADDED Requirements

### Requirement: Seed phrase has a plaintext label and an encrypted secret payload
The CLI SHALL store each seed phrase as a row in the `seeds` table (plaintext label, timestamps) and a corresponding row in the `seed_vaults` table (KDF parameters, salt, encrypted DEK, encrypted payload). The label SHALL be unique across all seeds.

#### Scenario: Seed phrase stored with a unique label
- **WHEN** the user adds a seed phrase with label `"main_seed"`
- **THEN** a row is inserted into `seeds` with the given label
- **AND** a corresponding row is inserted into `seed_vaults` with the encrypted payload
- **AND** the plaintext mnemonic or seed entropy is not stored anywhere in the clear

#### Scenario: Duplicate seed label rejected
- **WHEN** the user attempts to add a seed phrase with a label already present in `seeds`
- **THEN** the CLI rejects the operation with an error
- **AND** no new rows are inserted

### Requirement: Each seed phrase is its own independently password-protected encryption domain
The CLI SHALL generate a unique Data Encryption Key (DEK) per seed and protect it with a Key Encryption Key (KEK) derived from the user-supplied password using Argon2id. Unlocking one seed SHALL NOT require the password of any other seed.

#### Scenario: Seed unlocked with correct password
- **WHEN** the user provides the correct password for seed `"main_seed"`
- **THEN** the CLI derives the KEK from the password and stored KDF parameters
- **AND** decrypts the DEK from `seed_vaults`
- **AND** uses the DEK to decrypt the seed payload
- **AND** returns the plaintext seed secret

#### Scenario: Wrong password rejected
- **WHEN** the user provides an incorrect password for a seed
- **THEN** the CLI fails to authenticate and returns an error
- **AND** no plaintext seed material is exposed

#### Scenario: Unlocking one seed does not expose another seed
- **WHEN** the user unlocks seed `"main_seed"` with its password
- **THEN** the secret payload of any other seed (e.g., `"cold_seed"`) remains encrypted
- **AND** reading another seed's payload requires that seed's own password

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
