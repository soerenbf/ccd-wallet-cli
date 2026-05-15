## ADDED Requirements

### Requirement: Account creation can be initiated from a stored usable identity
`account new <LABEL>` SHALL create a new Concordium account from a stored issued identity. The command SHALL resolve network and seed context using the same interactive and non-interactive conventions as other wallet flows, then select an identity belonging to the resolved network and owning seed. The selected identity MUST contain a stored identity issuance payload, MUST not be expired at selection time or submission time, and MUST be locally usable either because it already has status `done` or because a lazy confirmation check can advance a `pending` identity to `done`.

If `--identity <LABEL>` is supplied, the command SHALL use that identity label directly after validating that the identity belongs to the resolved network, is owned by the resolved seed, is not expired, and is either already `done` or can be lazily confirmed from `pending`. If no identity is supplied and interactive mode is enabled, the CLI SHALL present a selector containing only identities that are not expired and have enough stored state to be used directly or lazily confirmed. In `--non-interactive` mode, omitting a required identity selection SHALL produce an actionable error.

Before unlocking the seed or submitting the credential deployment, the CLI SHALL display the effective context using the resolved seed label, network label/node endpoint, and chosen identity label when those values came from defaults, explicit overrides, or inference.

#### Scenario: Interactive account creation lists only usable identities
- **WHEN** the user runs `account new my-account`
- **AND** interactive mode is enabled
- **THEN** the CLI presents a selector containing only identities on the resolved network for the resolved seed that are unexpired and either already `done` or eligible for lazy confirmation from `pending`
- **AND** expired identities are not selectable

#### Scenario: Explicit identity label is rejected when expired
- **WHEN** the user runs `account new my-account --identity old-id`
- **AND** `old-id` has expired usability metadata
- **THEN** the CLI exits with an actionable error before prompting for the seed password or submitting any transaction

#### Scenario: Non-interactive mode requires explicit usable identity
- **WHEN** the user runs `account new my-account --non-interactive`
- **AND** no identity argument is supplied
- **THEN** the CLI does not prompt for identity selection
- **AND** exits with an actionable error describing how to provide a usable identity label

#### Scenario: Pending identity is lazily confirmed before account creation
- **WHEN** the user selects or supplies an identity that is still marked `pending`
- **AND** the identity has stored encrypted issuance state sufficient to resume confirmation
- **THEN** the CLI polls the identity provider before account creation proceeds
- **AND** updates the identity to `done` if the provider now reports completion

#### Scenario: Pending identity remains pending during lazy confirmation
- **WHEN** the user selects or supplies an identity that is still marked `pending`
- **AND** a lazy confirmation check still returns `pending`
- **THEN** the CLI exits with an actionable message that the identity is not yet ready for account creation
- **AND** does not submit an account creation transaction

#### Scenario: Pending identity fails during lazy confirmation
- **WHEN** the user selects or supplies an identity that is still marked `pending`
- **AND** a lazy confirmation check returns provider error
- **THEN** the CLI exits with the provider error detail
- **AND** does not submit an account creation transaction

### Requirement: Account creation deploys a normal credential derived from the selected identity
The account creation flow SHALL unlock the selected seed, derive the Concordium account credential material from the tuple `(network, ip_identity, identity_index, credential_counter)`, construct a normal credential deployment using the stored issued identity object plus current chain context, submit it to the resolved node, and treat the resulting block item as an account creation transaction.

The credential counter SHALL be allocated as the next available value within `(network_genesis_hash, seed_id, ip_identity, identity_index)`. Immediately before submission, the command SHALL revalidate that the selected identity is still unexpired. On successful finalization, the flow SHALL persist the new account address in the encrypted account private payload and mark the account record finalized.

#### Scenario: First account for an identity uses credential counter zero
- **WHEN** no prior account exists for a given `(network_genesis_hash, seed_id, ip_identity, identity_index)` tuple
- **THEN** `account new` allocates credential counter `0`
- **AND** submits a normal credential deployment for that counter

#### Scenario: Subsequent account increments credential counter for same identity tuple
- **WHEN** one finalized account already exists for a given `(network_genesis_hash, seed_id, ip_identity, identity_index)` tuple
- **THEN** the next `account new` run allocates credential counter `1`
- **AND** does not reuse credential counter `0`

#### Scenario: Identity expires before submission
- **WHEN** an identity was selectable earlier in the flow
- **AND** a final pre-submission validation determines the identity is now expired
- **THEN** the CLI exits with an actionable error
- **AND** does not submit the credential deployment transaction

#### Scenario: Successful account creation finalizes with stored encrypted address
- **WHEN** the node finalizes the credential deployment as an account creation
- **THEN** the CLI marks the account record finalized
- **AND** stores the created account address only inside the encrypted account private payload
- **AND** prints a success message for the account label

### Requirement: Account creation persists pending lifecycle state
The account creation flow SHALL create a pending account record before waiting for finalization and SHALL update that record as the on-chain outcome becomes known. The pending record SHALL capture enough plaintext metadata to preserve uniqueness and enough encrypted/private linkage to populate the encrypted payload after success.

By default, `account new` SHALL wait for finalization. A command flag SHALL allow the user to skip waiting after successful submission, leaving the local account record pending for later lazy finalization checks. If submission fails before the block item is accepted, the flow SHALL NOT leave behind a finalized account record. If the block item has been accepted but finalization has not yet been observed, the record SHALL remain pending for later inspection or reconciliation.

#### Scenario: Pending account record exists while waiting for finalization
- **WHEN** the credential deployment has been submitted successfully
- **AND** finalization has not yet been observed
- **THEN** the local account record remains in `pending` status
- **AND** retains the derivation tuple metadata for later lookup

#### Scenario: Submission failure does not create finalized account
- **WHEN** credential deployment submission fails before the node accepts the block item
- **THEN** the CLI exits with an actionable error
- **AND** no account record is marked finalized

#### Scenario: Account creation waits for finalization by default
- **WHEN** the user runs `account new my-account` without a skip-wait flag
- **THEN** the CLI waits for the submitted credential deployment to finalize before reporting success

#### Scenario: Account creation can skip waiting for finalization
- **WHEN** the user runs `account new my-account` with the skip-wait flag
- **THEN** the CLI returns after successful submission without waiting for finalization
- **AND** leaves the local account record in `pending` status
