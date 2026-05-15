## MODIFIED Requirements

### Requirement: Identity objects are persisted in SQLite
The system SHALL store issued identities in the local SQLite database as plaintext public metadata plus encrypted private payload data. Public metadata SHALL include the network identity (genesis hash), owning seed id, user-supplied identity label, identity provider index, identity index, issuance status, creation timestamp, and plaintext identity usability metadata required for account creation, including identity expiry. Private payload data SHALL include the `code_uri` and issued identity object and SHALL be encrypted under the owning seed's password domain. The `(network_genesis_hash, label)` pair SHALL be unique; the `(network_genesis_hash, seed_id, ip_identity, identity_index)` tuple SHALL also be unique.

#### Scenario: New identity row created when issuance starts
- **WHEN** the issuance flow receives a `code_uri`
- **THEN** a new identity metadata row is inserted with status `pending`
- **AND** the plaintext `code_uri` is not stored in the identity metadata row
- **AND** an encrypted private payload row is inserted containing the `code_uri`

#### Scenario: Identity row updated to done on successful poll
- **WHEN** polling returns status `done`
- **THEN** the identity metadata row is updated with status `done`
- **AND** plaintext usability metadata needed for account creation, including identity expiry, is extracted and stored on the identity metadata row
- **AND** the issued identity object JSON is stored only inside the encrypted private payload
- **AND** no plaintext identity object JSON is stored in SQLite

#### Scenario: Identity row deleted on failed poll
- **WHEN** polling returns status `error`
- **THEN** the pending identity row is deleted
- **AND** its encrypted private payload row is deleted by cascade

#### Scenario: Duplicate label within same network is rejected
- **WHEN** an identity with the same `(network_genesis_hash, label)` already exists
- **THEN** the store layer returns an error

#### Scenario: Same label on different networks is allowed
- **WHEN** two identities use the same `label` but different `network_genesis_hash` values
- **THEN** both identities can be stored

#### Scenario: Duplicate (network, seed, IP, identity_index) is rejected
- **WHEN** an identity row with the same `(network_genesis_hash, seed_id, ip_identity, identity_index)` tuple already exists
- **THEN** the store layer returns an error before contacting the identity provider

## ADDED Requirements

### Requirement: Identity usability metadata supports account creation prevalidation
The system SHALL persist enough plaintext identity usability metadata to let account creation determine whether an identity is eligible for use without decrypting all stored identity payloads. For this change, the only plaintext usability metadata promoted for account creation is identity expiry. This metadata MUST allow the wallet to reject expired identities during identity selection and again immediately before credential deployment submission.

#### Scenario: Expired identities are discoverable without decrypting private payloads
- **WHEN** the wallet prepares a list of candidate identities for account creation
- **THEN** it can determine from plaintext identity metadata whether each identity is expired
- **AND** it does not need to decrypt every stored identity payload first

#### Scenario: Identity with missing usability metadata cannot be used for account creation
- **WHEN** an identity does not have the required plaintext usability metadata for account creation
- **THEN** the wallet treats that identity as unusable for account creation
- **AND** reports an actionable error instead of attempting transaction submission

### Requirement: Pending identities support deferred completion
The system SHALL support identities remaining in `pending` status after issuance initiation and SHALL use the stored encrypted issuance state to complete them later.

Identity issuance SHALL wait for provider completion by default. Identity issuance SHALL also support an explicit skip-wait option that returns after the browser callback has provided `code_uri`, leaving the identity in `pending` status for later completion. If account creation attempts to use an identity that is still marked `pending`, the system SHALL use the stored encrypted issuance state to perform a lazy confirmation check with the identity provider before deciding whether the identity can be used.

If the provider now reports `done`, the wallet SHALL update the local identity record to `done`, persist any required plaintext expiry metadata, and continue account creation. If the provider still reports `pending`, the wallet SHALL leave the identity pending and stop account creation with an actionable message. If the provider reports `error`, the wallet SHALL surface the provider error and SHALL NOT proceed with account creation.

#### Scenario: Identity issuance waits for completion by default
- **WHEN** the user runs identity issuance without a skip-wait flag
- **THEN** the wallet continues polling the provider after receiving `code_uri`
- **AND** completes the identity flow only when the provider returns `done` or `error`

#### Scenario: Identity issuance can skip waiting after callback
- **WHEN** the user runs identity issuance with a skip-wait flag
- **AND** the browser callback has provided `code_uri`
- **THEN** the wallet stores the pending identity state
- **AND** returns without waiting for provider completion
- **AND** leaves the identity record in `pending` status

#### Scenario: Pending identity becomes done during lazy confirmation
- **WHEN** account creation selects an identity with local status `pending`
- **AND** the provider now reports `done`
- **THEN** the wallet updates the identity record to `done`
- **AND** persists plaintext expiry metadata needed for account creation
- **AND** allows account creation to continue

#### Scenario: Pending identity remains pending during lazy confirmation
- **WHEN** account creation selects an identity with local status `pending`
- **AND** the provider still reports `pending`
- **THEN** the wallet leaves the identity record pending
- **AND** stops account creation with an actionable message

#### Scenario: Pending identity returns provider error during lazy confirmation
- **WHEN** account creation selects an identity with local status `pending`
- **AND** the provider reports `error`
- **THEN** the wallet surfaces the provider error
- **AND** does not proceed with account creation
