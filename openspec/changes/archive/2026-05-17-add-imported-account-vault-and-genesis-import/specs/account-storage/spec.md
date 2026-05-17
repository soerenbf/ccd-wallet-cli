## ADDED Requirements

### Requirement: Account records distinguish derived and imported sources
The account storage layer SHALL represent whether each account is backed by seed derivation or imported secret material. Derived accounts SHALL retain their network, seed, identity provider, identity index, and credential counter metadata. Imported accounts SHALL reference imported account vault-backed secret material instead of requiring seed derivation metadata.

#### Scenario: Existing derived account metadata is preserved
- **WHEN** the wallet stores or migrates a derived account
- **THEN** the account remains associated with its `network_genesis_hash`, `seed_id`, `ip_identity`, `identity_index`, and `credential_counter`
- **AND** the account source is recorded as derived

#### Scenario: Imported account does not require derivation tuple
- **WHEN** the wallet stores an imported account
- **THEN** the account source is recorded as imported
- **AND** the account does not require a seed id, identity provider id, identity index, or credential counter to be present

### Requirement: Account labels remain unique across all account sources on a network
The account storage layer SHALL enforce a single account label namespace per `network_genesis_hash`. Derived and imported accounts SHALL NOT be allowed to share the same label on the same network.

#### Scenario: Imported label collides with derived account
- **WHEN** a derived account with label `alice` exists on a network
- **AND** the wallet attempts to import an account with label `alice` on the same network
- **THEN** the store rejects the imported account

#### Scenario: Derived label collides with imported account
- **WHEN** an imported account with label `alice` exists on a network
- **AND** the wallet attempts to create a derived account with label `alice` on the same network
- **THEN** the store rejects the derived account

#### Scenario: Same label on different networks remains allowed
- **WHEN** account label `alice` exists on network genesis hash `A`
- **AND** the wallet stores another account with label `alice` on network genesis hash `B`
- **THEN** both account records can be stored

### Requirement: Imported account payloads are encrypted under the imported vault domain
The account storage layer SHALL encrypt imported account private payloads using the imported accounts vault DEK for the account's `network_genesis_hash`. Imported account payload encryption SHALL use unique nonces and AAD binding the ciphertext to the account row, network, and imported vault context.

#### Scenario: Imported payload stores address and signing material encrypted
- **WHEN** the wallet imports an account from genesis JSON
- **THEN** the account address and signing material are stored in encrypted imported account payload data
- **AND** those values are not stored in plaintext account metadata columns

#### Scenario: AAD prevents imported payload transplantation
- **WHEN** an encrypted imported account payload is copied to another account row or network vault context
- **THEN** AEAD authentication fails during decryption

### Requirement: Account deletion cascades source-specific private data
The account storage layer SHALL remove source-specific private payload rows when an account row is deleted. Deleting a seed SHALL cascade derived accounts owned by that seed, but SHALL NOT delete imported accounts. Pruning a network partition SHALL delete all accounts on that network, including imported accounts and imported account payloads.

#### Scenario: Deleting imported account deletes imported payload
- **WHEN** an imported account metadata row is deleted
- **THEN** its encrypted imported account payload row is deleted automatically

#### Scenario: Deleting seed leaves imported accounts intact
- **WHEN** a seed is deleted
- **THEN** derived accounts owned by that seed are deleted
- **AND** imported accounts on the same networks remain stored

#### Scenario: Network prune deletes imported accounts and payloads
- **WHEN** the storage layer deletes all account rows for a network genesis hash
- **THEN** imported accounts for that network are removed
- **AND** their encrypted imported account payloads are removed
