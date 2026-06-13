# account-signing-source Specification

## Purpose
TBD - created by archiving change add-imported-account-vault-and-genesis-import. Update Purpose after archive.
## Requirements
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

### Requirement: Sender account resolution requires local signing-capable accounts
Commands that submit signed transactions SHALL resolve sender inputs, including explicit `--sender` options and sender aliases such as `--account`, as local signing-capable accounts rather than generic account references. A sender value SHALL identify a finalized local account record whose source can provide signatures through seed, Ledger, or imported-account signing material. Raw Concordium account addresses SHALL NOT be accepted as transaction senders because they do not provide local signing authority.

Explicit network inputs SHALL be hard constraints for sender lookup. When a sender label is supplied interactively without an explicit network, the CLI SHALL apply account-assisted network resolution. The active network, when set, SHALL be a soft default: if the active network has an eligible matching sender account, the CLI SHALL prefer that match; if it does not and exactly one eligible match exists across configured networks, the CLI SHALL infer that account's network; if multiple eligible matches remain, prompting-capable commands SHALL disambiguate with an account selector. In non-interactive mode, sender resolution SHALL NOT infer a network solely from account-label uniqueness and SHALL NOT let a sender label override the active network.

#### Scenario: Sender label resolves to local signing account
- **WHEN** a transaction-submitting command receives `--sender alice`
- **AND** `alice` resolves to a finalized local account in the selected or inferred network context
- **THEN** the CLI uses that local account as the transaction sender
- **AND** resolves signing material according to the account source kind

#### Scenario: Explicit network constrains sender lookup
- **WHEN** a transaction-submitting command receives `--network testnet --sender alice`
- **AND** `alice` exists as a finalized local account on another network but not on `testnet`
- **THEN** the CLI exits with an actionable error for `testnet`
- **AND** does not infer or switch to the other network

#### Scenario: Raw sender address is rejected
- **WHEN** a transaction-submitting command receives `--sender 4UC8o4m8AgTxt5VBFMdLwMCwwJQVJwjesNzW7RPXkACynrULmd`
- **THEN** the CLI exits with an actionable error explaining that transaction senders must be local account labels
- **AND** no transaction is submitted

#### Scenario: Active network chooses among ambiguous sender labels
- **WHEN** a transaction-submitting interactive command receives `--sender alice`
- **AND** no explicit network was supplied
- **AND** the active network is `testnet`
- **AND** finalized local accounts named `alice` exist on both `testnet` and another configured network
- **THEN** the CLI selects the `testnet` sender account
- **AND** does not prompt for network selection
- **AND** displays the resolved network and sender account context

#### Scenario: Interactive unique sender label outside active network infers network
- **WHEN** a transaction-submitting interactive command receives `--sender alice`
- **AND** no explicit network was supplied
- **AND** the active network does not have a finalized local account named `alice`
- **AND** exactly one finalized local account named `alice` exists on another configured network
- **THEN** the CLI infers the network from that account
- **AND** does not prompt for network selection
- **AND** displays the resolved network and sender account context

#### Scenario: Non-interactive unique sender label does not infer network
- **WHEN** a transaction-submitting non-interactive command receives `--sender alice`
- **AND** no explicit or otherwise supported deterministic network context is available
- **AND** exactly one finalized local account named `alice` exists across configured networks
- **THEN** the CLI exits with an actionable network-resolution error
- **AND** does not infer the network from `alice`

#### Scenario: Non-interactive sender label does not override active network
- **WHEN** a transaction-submitting non-interactive command receives `--sender alice`
- **AND** the active network is `testnet`
- **AND** `alice` exists only on another configured network
- **THEN** the CLI exits with an actionable error for `testnet`
- **AND** does not infer or switch to the other network

