## ADDED Requirements

### Requirement: Maximum lock duration is a typed governance update
The CLI SHALL decode, inspect, locally sign, and submit the P11 Max Lock Duration governance update through the existing all-in-one and detached proposal workflows. The update SHALL use the token-parameters level-2 authorization structure and the maximum-lock-duration sequence-number queue.

#### Scenario: JSON maximum lock duration update is resolved
- **WHEN** the user supplies a valid `maxLockDuration` governance update JSON payload
- **THEN** the CLI decodes it as a known typed update
- **AND** resolves authorization from the live token-parameters access structure
- **AND** resolves the next sequence number from the maximum-lock-duration queue unless explicitly overridden

#### Scenario: Serialized maximum lock duration update is resolved
- **WHEN** the user supplies a serialized Max Lock Duration governance payload
- **THEN** the CLI decodes it as a known typed update without requiring `--blind`
- **AND** applies the same authorization and sequence behavior as JSON input

#### Scenario: Local signer submits maximum lock duration update
- **WHEN** authorized local governance keys satisfy the token-parameters threshold
- **AND** the user approves a Max Lock Duration update
- **THEN** the CLI signs and submits it through the existing governance update flow

#### Scenario: Detached proposal supports maximum lock duration
- **WHEN** the user creates, signs, or submits a detached proposal containing a Max Lock Duration update
- **THEN** the CLI treats it as typed proposal material
- **AND** applies existing proposal freezing, review, revalidation, signature, and submission behavior

#### Scenario: Review displays maximum lock duration semantics
- **WHEN** the CLI renders a review for a Max Lock Duration update
- **THEN** the review identifies the update type
- **AND** displays the proposed duration in the parsed payload details

### Requirement: Authorization V3 updates are recognized
The CLI SHALL recognize root- and level-1-authorized V3 level-2 authorization updates containing both Create PLT and token-parameters access structures.

#### Scenario: Root authorization V3 update remains typed
- **WHEN** a root-governed payload contains a V3 level-2 authorization update
- **THEN** the CLI decodes and prepares it as a known root-authorized update

#### Scenario: Level-1 authorization V3 update remains typed
- **WHEN** a level-1-governed payload contains a V3 level-2 authorization update
- **THEN** the CLI decodes and prepares it as a known level-1-authorized update
