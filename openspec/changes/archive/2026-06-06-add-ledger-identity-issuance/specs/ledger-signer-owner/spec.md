## MODIFIED Requirements

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
