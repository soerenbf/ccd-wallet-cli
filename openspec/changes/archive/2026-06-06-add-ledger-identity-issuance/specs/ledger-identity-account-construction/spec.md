## MODIFIED Requirements

### Requirement: Ledger construction layer bridges wallet flows to APDU commands
The system SHALL provide a Ledger identity/account construction layer between CLI orchestration and the low-level Ledger APDU client. For identity issuance, the layer SHALL use the Concordium Ledger app `5.5.0+` purpose-based identity credential creation export protocol to retrieve IDCredSec, PRFKey, and deterministic signature blinding randomness after explicit user approval. For account credential deployment and account transaction signing, the layer SHALL continue to prepare Ledger request payloads and return SDK-compatible values or raw signatures needed by higher-level wallet flows.

#### Scenario: CLI uses construction layer for Ledger identity issuance
- **WHEN** identity issuance targets a Ledger key source
- **THEN** the CLI invokes the Ledger construction layer rather than seed-derived `ConcordiumHdWallet` derivation methods
- **AND** the construction layer enforces Ledger-specific export and recovery-safety policy before preparing issuance material

#### Scenario: App 5.5.0 purpose-based identity export succeeds
- **WHEN** identity issuance targets a Ledger key source
- **AND** the connected Ledger device matches the selected signer owner's stored canonical public key
- **AND** the user explicitly approved secret export
- **AND** the connected app supports purpose-based identity credential creation export
- **THEN** the construction layer sends `INS=0x37`, `P1=0x00`, `P2=0x00` for mainnet or `P2=0x01` for testnet, and `CDATA=idp_index || identity_index`
- **AND** parses exactly three `[length=32][key]` fields ordered IDCredSec, PRFKey, and signature blinding randomness
- **AND** returns complete identity issuance material to the CLI

#### Scenario: App 5.4.1 legacy new-path export is insufficient for identity issuance
- **WHEN** identity issuance targets a Ledger key source
- **AND** the installed Concordium Ledger app implements `INS=0x37` as legacy new-path PRF/IDCredSec export
- **THEN** the construction layer does not treat raw 32-byte or 64-byte responses as complete identity issuance material
- **AND** no pending identity row is written

#### Scenario: Unsupported purpose-based export fails before provider contact
- **WHEN** identity issuance targets a Ledger key source
- **AND** the connected app rejects the purpose-based identity credential creation export command as unsupported or invalid
- **THEN** the construction layer fails with an actionable message telling the user that Concordium Ledger app `5.5.0` or newer is required
- **AND** no pending identity row is written

#### Scenario: CLI uses construction layer for Ledger account creation
- **WHEN** account creation targets a Ledger-owned identity
- **THEN** the CLI invokes the Ledger construction layer to prepare credential deployment payloads and Ledger signing requests
- **AND** the resulting credential deployment is suitable for normal node submission by the existing account creation flow

### Requirement: Ledger construction layer does not implicitly export private key material
Ledger-backed identity and account flows SHALL NOT use Ledger private-key export commands as an implicit fallback. Ledger identity issuance MAY use Ledger private-key export commands only through an explicit user-approved capability with security wording that distinguishes the flow from on-device signing and only when the connected app supports all required recovery-critical material. Exported secret material SHALL remain transient and SHALL NOT be stored in the wallet database as Ledger-owned persistent signing material.

#### Scenario: Missing approval does not silently export secrets
- **WHEN** a Ledger-backed identity issuance flow reaches the point where secret export would be required
- **AND** the user has not explicitly approved the export capability for the current command
- **THEN** the wallet does not call a Ledger private-key export command
- **AND** the flow fails before any pending identity row is written

#### Scenario: Host-generated blinding randomness is not accepted for Ledger-backed recoverable identities
- **WHEN** Ledger-backed identity issuance needs signature blinding randomness
- **AND** the connected Ledger app cannot derive or export that randomness deterministically
- **THEN** the wallet does not generate replacement randomness on the host to complete the flow
- **AND** the flow fails before any pending identity row is written

#### Scenario: Export-based identity issuance does not persist exported secrets
- **WHEN** a Ledger-backed identity issuance flow exports all required issuance material after explicit approval
- **THEN** the exported secret material is used only transiently to construct the issuance request
- **AND** the wallet does not store that exported material as Ledger-owned persistent secret state
