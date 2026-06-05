## ADDED Requirements

### Requirement: Ledger signer owners can be enrolled
The CLI SHALL provide an enrollment flow for Ledger signer owners. Enrollment SHALL connect to the Concordium Ledger app, request the canonical public key at the configured enrollment derivation path, optionally require device confirmation where supported, derive a display fingerprint, prompt for a local signer-owner label and local password, and store the Ledger signer owner.

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
The identity issuance flow SHALL support Ledger signer owners as identity owners. Ledger-owned identity issuance SHALL use Ledger-backed derivation/signing operations where required by the Concordium Ledger app and SHALL store private identity payloads under the Ledger signer owner's local password domain.

#### Scenario: Create Ledger-owned identity
- **WHEN** the user starts identity issuance for a Ledger signer owner
- **AND** the connected Ledger matches the requested signer owner
- **THEN** the wallet builds the identity request using Ledger-backed derivation/signing behavior
- **AND** stores the identity row with the Ledger signer owner's id
- **AND** encrypts the pending identity private payload under the Ledger signer owner's DEK

#### Scenario: Ledger-owned identity requires owner vault for local payload storage
- **WHEN** Ledger-owned identity issuance stores a `code_uri` or identity object
- **THEN** the signer-owner vault for the Ledger owner is unlocked with the local password
- **AND** the private identity payload is encrypted under that owner DEK

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
