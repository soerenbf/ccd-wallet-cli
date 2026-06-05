## MODIFIED Requirements

### Requirement: Account material resolution is source-aware for signing and export
The system SHALL resolve account material through the account's source kind and signer-owner kind for operations that need signer-capable account data. Seed-backed derived accounts SHALL use seed-derived signing material from the unlocked seed signer owner. Ledger-backed derived accounts SHALL use a matching connected Ledger device for signing and SHALL NOT require local private signing material. Imported accounts SHALL use encrypted imported account secret material from the imported accounts vault for the account's network.

#### Scenario: Seed-backed derived account material resolves through seed signer owner
- **WHEN** a signing or export operation targets a seed-backed derived account
- **THEN** the system resolves the owning seed signer owner and derivation coordinates
- **AND** derives the account signing key material from the unlocked seed secret

#### Scenario: Ledger-backed derived account material resolves through Ledger signer owner
- **WHEN** a signing operation targets a Ledger-backed derived account
- **THEN** the system resolves the owning Ledger signer owner and derivation coordinates
- **AND** verifies that the connected Ledger matches the signer owner's canonical public key
- **AND** obtains the required signature from the Ledger device
- **AND** does not load local private signing key material for the account

#### Scenario: Imported account material resolves through imported vault
- **WHEN** a signing or export operation targets an imported account
- **THEN** the system resolves the imported accounts vault for the account's network genesis hash
- **AND** decrypts the imported account signing material from that vault

### Requirement: Account labels identify signing source unambiguously within a network
The system SHALL rely on network-wide account label uniqueness to resolve a target account before selecting a signing source. It SHALL NOT permit separate seed-derived, Ledger-derived, or imported accounts with the same label on the same network.

#### Scenario: Label resolves to one account source
- **WHEN** a command targets account label `baker-0` on network `local`
- **AND** the wallet contains an account with that label on `local`
- **THEN** the system resolves exactly one account record
- **AND** chooses the signing source from that account record's source kind and signer-owner kind

#### Scenario: Cross-source duplicate label is rejected
- **WHEN** a derived or imported account already uses label `baker-0` on a network
- **AND** the user attempts to create or import another account with label `baker-0` on the same network
- **THEN** the operation is rejected before any signing-source ambiguity is introduced

## ADDED Requirements

### Requirement: Ledger signing failures do not submit transactions
The system SHALL treat Ledger unavailability, owner mismatch, unsupported Ledger command flows, and user rejection on the device as signing failures. The wallet SHALL NOT submit unsigned, partially signed, or locally fallback-signed transactions for Ledger-backed accounts when Ledger signing fails.

#### Scenario: User rejects Ledger signing
- **WHEN** a transaction targets a Ledger-backed account
- **AND** the user rejects the signing operation on the Ledger device
- **THEN** the wallet reports the rejection
- **AND** no transaction is submitted

#### Scenario: Unsupported Ledger flow fails safely
- **WHEN** a transaction targets a Ledger-backed account
- **AND** the wallet cannot map the transaction to a supported Ledger app signing command
- **THEN** signing fails with an actionable error
- **AND** no transaction is submitted
