# account-signing-source Specification

## Purpose
TBD - created by archiving change add-imported-account-vault-and-genesis-import. Update Purpose after archive.
## Requirements
### Requirement: Account material resolution is source-aware for signing and export
The system SHALL resolve account material through the account's source kind for operations that need signer-capable account data. Derived accounts SHALL use seed-derived signing material, and imported accounts SHALL use encrypted imported account secret material from the imported accounts vault for the account's network.

#### Scenario: Derived account material resolves through seed
- **WHEN** a signing or export operation targets a derived account
- **THEN** the system resolves the owning seed and derivation coordinates
- **AND** derives the account signing key material from the unlocked seed

#### Scenario: Imported account material resolves through imported vault
- **WHEN** a signing or export operation targets an imported account
- **THEN** the system resolves the imported accounts vault for the account's network genesis hash
- **AND** decrypts the imported account signing material from that vault

### Requirement: Imported signing material supports normal account transaction signing
Imported account secret material SHALL contain the signing keys and account credential metadata necessary to sign normal account transactions for the imported account.

#### Scenario: Imported signing payload contains account signing keys
- **WHEN** a genesis account JSON file is imported successfully
- **THEN** the stored imported account secret payload contains the account signing key material needed for transactions
- **AND** the signing key material is encrypted at rest

#### Scenario: Signing resolver rejects incomplete imported payload
- **WHEN** a signing operation targets an imported account whose encrypted payload is missing required signing material
- **THEN** signing material resolution fails with an actionable error
- **AND** no unsigned or partially signed transaction is submitted

### Requirement: Account labels identify signing source unambiguously within a network
The system SHALL rely on network-wide account label uniqueness to resolve a target account before selecting a signing source. It SHALL NOT permit separate derived and imported accounts with the same label on the same network.

#### Scenario: Label resolves to one account source
- **WHEN** a command targets account label `baker-0` on network `local`
- **AND** the wallet contains an account with that label on `local`
- **THEN** the system resolves exactly one account record
- **AND** chooses the signing source from that account record's source kind

#### Scenario: Cross-source duplicate label is rejected
- **WHEN** a derived account already uses label `baker-0` on a network
- **AND** the user imports an account with label `baker-0` on the same network
- **THEN** the import is rejected before any signing-source ambiguity is introduced

