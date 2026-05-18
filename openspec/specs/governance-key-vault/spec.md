# governance-key-vault Specification

## Purpose
TBD - created by archiving change add-governance-key-vault-and-management. Update Purpose after archive.
## Requirements
### Requirement: Governance key vaults are scoped by network genesis hash
The system SHALL store imported governance signing keypairs in a governance key vault associated with a single `network_genesis_hash`. At most one governance key vault SHALL exist for a given network genesis hash in a wallet database.

#### Scenario: First governance key import creates network governance vault
- **WHEN** the user imports a governance key for a network that has no governance key vault
- **THEN** the system creates a governance key vault for that network's genesis hash
- **AND** stores the imported governance key payload under that vault

#### Scenario: Later governance key import reuses network governance vault
- **WHEN** the user imports another governance key for a network whose governance key vault already exists
- **THEN** the system stores the new governance key payload under the existing vault
- **AND** does not create a second governance key vault for that genesis hash

### Requirement: Governance key vault contents are fully encrypted
The system SHALL encrypt imported governance key JSON payloads under the governance key vault password domain. Both `signKey` and `verifyKey` from the imported keypair JSON SHALL remain encrypted at rest, and the database SHALL NOT expose governance public keys or governance levels without unlocking the vault.

#### Scenario: Governance key payload remains opaque without password
- **WHEN** governance keys have been imported into a governance key vault
- **THEN** the local database does not expose plaintext governance public keys or private keys without the governance vault password

#### Scenario: Correct governance vault password decrypts imported key material
- **WHEN** the user supplies the correct password for a governance key vault
- **THEN** the system can decrypt imported governance key JSON payloads for that network

### Requirement: Governance keys are identified by public key after unlock
The system SHALL treat the governance key public key (`verifyKey`) as the key identity for duplicate detection and targeted removal. Because the public key is encrypted at rest, duplicate detection and explicit verify-key removal SHALL happen after governance vault unlock.

#### Scenario: Duplicate public key import is rejected after unlock
- **WHEN** the user imports a governance key whose decrypted `verifyKey` already exists in the governance vault for that network
- **THEN** the import is rejected
- **AND** no duplicate governance key payload is stored

#### Scenario: Explicit verify-key removal requires governance vault unlock
- **WHEN** the user runs `ccd-wallet governance keys remove <verify-key>`
- **THEN** the CLI unlocks the governance key vault for the selected network before matching stored keys to the requested public key
