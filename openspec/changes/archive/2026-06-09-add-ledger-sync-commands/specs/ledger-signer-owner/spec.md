## MODIFIED Requirements

### Requirement: Ledger signer owners can be enrolled
The CLI SHALL provide an enrollment flow for Ledger signer owners. Enrollment SHALL connect to the Concordium Ledger app, request the canonical public key at the configured enrollment derivation path, optionally require device confirmation where supported, derive a display fingerprint, prompt for a local signer-owner label and local password, and store the Ledger signer owner. The command SHALL support `--restore <NETWORK>` to run Ledger-backed recovery immediately after successful enrollment. If `--restore` is supplied, the named network SHALL be validated before any enrollment state is written, and successful enrollment SHALL continue directly into Ledger recovery for the newly enrolled key source.

#### Scenario: Enroll Ledger signer owner
- **WHEN** the user enrolls a Ledger signer owner
- **AND** a compatible Ledger device is connected and the Concordium Ledger app returns the canonical public key
- **THEN** the CLI creates a signer owner with `owner_kind = 'ledger'`
- **AND** stores Ledger owner details containing the canonical public key, fingerprint, and enrollment path
- **AND** creates a signer-owner vault protected by the local password

#### Scenario: Enrollment rejects duplicate Ledger root
- **WHEN** the connected Ledger returns a canonical public key already stored for another Ledger signer owner
- **THEN** enrollment fails with an actionable duplicate-owner error
- **AND** no new signer owner or vault is created

#### Scenario: Enrollment does not store Ledger signing secrets
- **WHEN** Ledger enrollment succeeds
- **THEN** the wallet stores public enrollment metadata and local vault metadata only
- **AND** does not store Ledger private keys or seed material

#### Scenario: Enroll and immediately restore on chosen network
- **WHEN** the user runs `ccd-wallet ledger setup ledger-main --restore testnet`
- **AND** a compatible Ledger device is connected and enrollment succeeds
- **AND** the network `testnet` exists
- **THEN** the CLI stores the Ledger key source successfully
- **AND** immediately starts recovery for Ledger key source `ledger-main` on network `testnet`
- **AND** exits with a recovery summary after the restore flow completes

#### Scenario: Restore network must exist before enrollment proceeds
- **WHEN** the user runs `ccd-wallet ledger setup ledger-main --restore missingnet`
- **AND** no configured network matches `missingnet`
- **THEN** the CLI exits with a non-zero status
- **AND** prints an actionable error indicating that the network is not configured
- **AND** does not create a signer owner, vault, or Ledger owner-details row
