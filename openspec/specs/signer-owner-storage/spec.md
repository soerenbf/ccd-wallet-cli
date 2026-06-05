# signer-owner-storage Specification

## Purpose
TBD - created by archiving change add-ledger-signer-owner-model. Update Purpose after archive.
## Requirements
### Requirement: Signer owners are persisted as derivation authorities
The storage layer SHALL persist signer owners as wallet-local derivation authorities. A signer owner SHALL have a stable id, an `owner_kind` of `seed` or `ledger`, a unique plaintext label, and timestamps. Signer owners SHALL be the ownership parent for signer-derived identities and accounts.

#### Scenario: Store seed signer owner
- **WHEN** the wallet stores a seed-backed derivation authority
- **THEN** the storage layer creates a signer owner with `owner_kind = 'seed'`
- **AND** the signer owner has a stable id and unique label

#### Scenario: Store Ledger signer owner
- **WHEN** the wallet enrolls a Ledger-backed derivation authority
- **THEN** the storage layer creates a signer owner with `owner_kind = 'ledger'`
- **AND** the signer owner has a stable id and unique label

#### Scenario: Duplicate signer owner label rejected
- **WHEN** a signer owner label already exists
- **AND** the wallet attempts to create another signer owner with the same label
- **THEN** the storage layer rejects the operation
- **AND** no signer owner row is inserted

### Requirement: Signer owners have independent password domains
Each signer owner SHALL have exactly one signer-owner vault. The vault SHALL store password-derived key wrapping metadata, an encrypted DEK, the DEK nonce, cipher version, and timestamps. The signer-owner DEK SHALL encrypt private payloads owned by that signer owner.

#### Scenario: Create signer owner vault
- **WHEN** a signer owner is created
- **THEN** the storage layer creates a signer-owner vault for the owner
- **AND** the vault protects an owner-specific DEK using the supplied local password

#### Scenario: Unlock signer owner vault
- **WHEN** the user supplies the correct local password for a signer owner
- **THEN** the storage layer decrypts the signer-owner DEK
- **AND** returns an unlock context usable for signer-owned private payload encryption and decryption

#### Scenario: Wrong signer owner password rejected
- **WHEN** the user supplies an incorrect local password for a signer owner
- **THEN** signer-owner vault unlock fails
- **AND** no signer-owned private payload plaintext is exposed

#### Scenario: Unlocking one signer owner does not unlock another
- **WHEN** the user unlocks signer owner `alice`
- **THEN** signer owner `bob` remains encrypted
- **AND** decrypting `bob`'s signer-owned private payloads requires `bob`'s local password

### Requirement: Seed owner secrets are seed-kind details
Seed signer owners SHALL store local seed secret bytes in a seed-specific encrypted detail row. Seed secret payloads SHALL be encrypted under the owning signer-owner DEK and SHALL NOT be stored in plaintext.

#### Scenario: Seed owner stores encrypted seed secret
- **WHEN** the wallet creates a seed signer owner
- **THEN** the storage layer stores the seed secret in `seed_owner_secrets`
- **AND** encrypts the secret under the signer-owner DEK
- **AND** does not store the plaintext seed secret in SQLite

#### Scenario: Seed owner secret removed with owner
- **WHEN** a seed signer owner is deleted
- **THEN** the corresponding seed owner secret row is deleted by cascade

### Requirement: Ledger owner details are Ledger-kind details
Ledger signer owners SHALL store Ledger enrollment metadata in a Ledger-specific detail row. Ledger owner details SHALL include the canonical public key used as the stable owner identity, a display fingerprint, and the enrollment derivation path. Ledger owner storage SHALL NOT store Ledger private signing material.

#### Scenario: Ledger owner stores canonical public key
- **WHEN** the wallet enrolls a Ledger signer owner
- **THEN** the storage layer stores the full canonical public key returned by the Ledger app
- **AND** stores the enrollment path used to obtain it
- **AND** stores a short display fingerprint derived from the canonical public key

#### Scenario: Duplicate Ledger canonical public key rejected
- **WHEN** a Ledger owner with canonical public key `K` already exists
- **AND** the wallet attempts to enroll another Ledger owner with canonical public key `K`
- **THEN** the storage layer rejects the duplicate Ledger owner

#### Scenario: Ledger owner details do not contain private key material
- **WHEN** Ledger owner details are persisted
- **THEN** the row contains public enrollment metadata only
- **AND** no Ledger private signing secret or exported private key is stored

### Requirement: Signer owner deletion cascades signer-owned data
Deleting a signer owner SHALL delete its owner vault, owner-kind detail row, signer-owned identities, signer-owned derived accounts, and related encrypted private payload rows.

#### Scenario: Delete signer owner cascades children
- **WHEN** signer owner `cold-ledger` is deleted
- **THEN** its signer-owner vault is deleted
- **AND** its seed or Ledger detail row is deleted
- **AND** identities owned by that signer owner are deleted
- **AND** derived accounts owned by that signer owner are deleted
- **AND** private payload rows for those identities and accounts are deleted

