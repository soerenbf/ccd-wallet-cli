# ledger-identity-account-construction Specification

## Purpose
Define the wallet-side construction layer that bridges Ledger-backed identity, credential, and account flows to low-level Concordium Ledger APDU commands while preserving explicit export policy and recoverability guarantees.

## Requirements
### Requirement: Ledger construction layer bridges wallet flows to APDU commands
The system SHALL provide a Ledger identity/account construction layer between CLI orchestration and the low-level Ledger APDU client. For identity issuance, the layer SHALL use the Concordium Ledger app `5.5.0+` purpose-based identity credential creation export protocol to retrieve IDCredSec, PRFKey, and deterministic signature blinding randomness after explicit user approval. For account creation, the layer SHALL use the Concordium Ledger app `5.6.4+` purpose-based account creation export protocol after explicit user approval to retrieve PRFKey, IDCredSec, and the extended commitment-randomness root, and the purpose-based identity recovery export to retrieve IDCredSec and signature blinding randomness. The layer MUST reject the material unless both exports contain the same IDCredSec. It SHALL construct the credential and zero-knowledge proofs transiently on the host and obtain the account public key and credential-deployment signature from the matching device. For account transaction signing, the layer SHALL continue to prepare Ledger request payloads and return raw signatures needed by higher-level wallet flows.

Exported account-creation material SHALL be parsed as exactly two `[length=32][key]` fields followed by one `[length=64][private-key || chain-code]` field. The extended commitment-randomness root SHALL derive attribute-specific randomness using hardened SLIP-10 child derivation. The account signing private key MUST remain on the device.

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
- **THEN** the CLI invokes the Ledger construction layer to prepare the credential deployment and Ledger signing request
- **AND** the resulting credential deployment is suitable for normal node submission by the existing account creation flow

#### Scenario: App 5.6.4 purpose-based account creation export succeeds
- **WHEN** account creation targets a Ledger-owned identity
- **AND** the matching connected device and user approve secret export
- **THEN** the construction layer sends `INS=0x37`, `P1=0x01`, the selected network in `P2`, and `CDATA=idp_index || identity_index || account_index`
- **AND** parses PRFKey, IDCredSec, and the extended commitment-randomness root in protocol order
- **AND** requests the purpose-based identity recovery export to obtain IDCredSec and signature blinding randomness
- **AND** verifies that both exported IDCredSec values match
- **AND** uses the material only for transient credential construction

#### Scenario: Exported identity secrets disagree
- **WHEN** account creation and identity recovery exports return different IDCredSec values
- **THEN** the construction layer rejects the material before proof construction or signing
- **AND** returns no deployment for submission

#### Scenario: Ledger signs the constructed credential deployment
- **WHEN** host-side credential and proof construction succeeds for a Ledger account
- **THEN** the construction layer stages the canonical credential deployment payload on the matching device
- **AND** obtains the account public key and final signature from that device
- **AND** returns an SDK-compatible normal credential deployment suitable for node submission

#### Scenario: Unsupported or malformed account creation export fails safely
- **WHEN** the connected app does not support the 5.6.4 account-creation export format
- **OR** the export does not contain fields of lengths 32, 32, and 64 in protocol order
- **THEN** the construction layer returns an actionable app-version or malformed-response error
- **AND** does not request a credential signature or return a deployment for submission

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

### Requirement: Ledger account-creation secrets remain transient
The construction layer SHALL keep exported PRFKey, IDCredSec, extended commitment-randomness key material, derived commitment randomness, and other host-resident secret intermediates transient. Wallet-owned secret containers SHALL be zeroized when no longer needed, and the implementation SHALL minimize secret copies and their lifetime. Copies created inside transport, allocator, cryptographic-library, hardware-driver, or operating-system boundaries MAY remain subject to those components' memory-clearing guarantees. The construction layer SHALL NOT persist exported material as Ledger-owned signing or recovery state.

#### Scenario: Account construction completes or fails
- **WHEN** Ledger-backed account construction returns successfully or with an error
- **THEN** wallet-owned secret containers are cleared when they leave scope
- **AND** lower-level library or system copies are retained no longer than required by those boundaries
- **AND** no exported account-creation secret is stored in the wallet database
