## ADDED Requirements

### Requirement: Imported account vaults are scoped by network genesis hash
The system SHALL store imported account secret material in an imported accounts vault associated with a single `network_genesis_hash`. At most one imported accounts vault SHALL be active for a given network genesis hash in a wallet database.

#### Scenario: First import creates network vault
- **WHEN** the user imports an account for a network that has no imported accounts vault
- **THEN** the system creates an imported accounts vault for that network's genesis hash
- **AND** stores imported secret material under that vault

#### Scenario: Later import reuses network vault
- **WHEN** the user imports another account for a network whose imported accounts vault already exists
- **THEN** the system stores the new imported account secret material under the existing vault
- **AND** does not create a second imported accounts vault for that genesis hash

### Requirement: Imported account vaults protect secrets with password-based encryption
The system SHALL protect imported account vault contents with password-based encryption comparable to seed vault protection. Imported account plaintext secrets SHALL NOT be stored in plaintext database columns.

#### Scenario: Correct imported vault password decrypts imported account payload
- **WHEN** the user supplies the correct password for the imported accounts vault of a network
- **THEN** the system can decrypt imported account secret material for accounts on that network

#### Scenario: Wrong imported vault password does not expose imported account payload
- **WHEN** the user supplies an incorrect imported vault password
- **THEN** imported account payload decryption fails
- **AND** no imported account address or signing key material is exposed

### Requirement: Imported account vault unlock is source-aware
The system SHALL unlock an imported accounts vault only when an operation needs encrypted imported account data for that vault's network. Operations on seed-derived accounts SHALL continue to unlock their owning seed instead.

#### Scenario: Imported account address reveal unlocks imported vault
- **WHEN** the user explicitly requests address display for an imported account
- **THEN** the CLI prompts for the imported accounts vault password for that account's network
- **AND** does not prompt for a seed password for that imported account

#### Scenario: Derived account address reveal still unlocks seed
- **WHEN** the user explicitly requests address display for a derived account
- **THEN** the CLI prompts for the owning seed password
- **AND** does not prompt for an imported accounts vault password for that derived account
