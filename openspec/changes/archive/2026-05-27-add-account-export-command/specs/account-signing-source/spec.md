## MODIFIED Requirements

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
