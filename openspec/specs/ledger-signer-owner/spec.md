# ledger-signer-owner Specification

## Purpose
TBD - created by archiving change add-ledger-signer-owner-model. Update Purpose after archive.
## Requirements
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

### Requirement: Ledger signer owners are recognized by canonical public key
The wallet SHALL recognize a connected Ledger signer owner by retrieving the canonical public key at the stored enrollment path and matching it against stored Ledger owner details. The wallet SHALL NOT use USB path, HID transport identifier, or other transient device-location data as persistent owner identity.

#### Scenario: Connected Ledger matches stored owner
- **WHEN** the wallet needs to use Ledger signer owner `hardware-main`
- **AND** a connected Ledger returns the canonical public key stored for that owner
- **THEN** the wallet treats the connected Ledger as the requested signer owner

#### Scenario: Connected Ledger does not match requested owner
- **WHEN** the wallet needs to use Ledger signer owner `hardware-main`
- **AND** the connected Ledger returns a different canonical public key
- **THEN** the wallet rejects the operation with an owner-mismatch error
- **AND** does not proceed with signing or derivation-sensitive work

### Requirement: Ledger signer owners can own identities
The identity issuance flow SHALL support Ledger signer owners as identity owners when the connected Concordium Ledger app supports the required purpose-based export protocol and matches the selected enrolled Ledger signer owner. Ledger-owned identity issuance SHALL verify the connected device's canonical public key against the selected signer owner's stored canonical public key before exporting issuance material. Ledger-owned identity issuance SHALL use the explicit approved Ledger export flow required to obtain identity issuance material and SHALL store private identity payloads under the Ledger signer owner's local password domain. Exported issuance secrets SHALL be treated as transient host-memory material and SHALL NOT be stored as persistent Ledger-owned signing state.

#### Scenario: Create Ledger-owned identity
- **WHEN** the user starts identity issuance for a Ledger signer owner
- **AND** the connected Ledger device matches the selected signer owner's stored canonical public key
- **AND** the connected Concordium Ledger app supports purpose-based identity credential creation export
- **AND** the user explicitly approves the Ledger export flow
- **THEN** the wallet builds the identity request using Ledger-derived issuance material
- **AND** stores the identity row with the selected Ledger signer owner's id
- **AND** encrypts the pending identity private payload under the Ledger signer owner's DEK

#### Scenario: Connected Ledger mismatch fails before export and storage
- **WHEN** the user starts identity issuance for a Ledger signer owner
- **AND** the connected Ledger device's canonical public key differs from the selected signer owner's stored canonical public key
- **THEN** the wallet fails before exporting identity issuance material
- **AND** no pending identity row is written

#### Scenario: Ledger-owned identity requires owner vault for local payload storage
- **WHEN** Ledger-owned identity issuance stores a `code_uri` or identity object
- **THEN** the signer-owner vault for the Ledger owner is unlocked with the local password
- **AND** the private identity payload is encrypted under that owner DEK

#### Scenario: Declined Ledger export does not create a Ledger-owned identity row
- **WHEN** the user declines the explicit Ledger export approval during identity issuance
- **THEN** the wallet does not create a pending Ledger-owned identity row
- **AND** does not store exported Ledger issuance secrets as local persistent state

### Requirement: Ledger signer owners can own derived accounts
Account creation SHALL support Ledger signer owners as owners of derived accounts. Ledger-derived account creation SHALL use Ledger-backed credential/account signing operations where required and SHALL store derived-account private payloads under the Ledger signer owner's local password domain.

#### Scenario: Create Ledger-derived account
- **WHEN** the user creates an account from a Ledger-owned identity
- **AND** the connected Ledger matches the owning signer owner
- **THEN** the wallet creates a derived account row with the Ledger signer owner's id
- **AND** uses the Ledger flow to authorize or sign the credential deployment
- **AND** stores the account address in a derived-account private payload encrypted under the Ledger signer owner's DEK

#### Scenario: Ledger-derived account signing requires matching device
- **WHEN** a transaction targets a Ledger-derived account
- **THEN** the wallet requires a connected Ledger whose canonical public key matches the account's signer owner
- **AND** obtains the transaction signature from the Ledger instead of local private key material

