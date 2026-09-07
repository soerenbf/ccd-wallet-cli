## ADDED Requirements

### Requirement: Token lock creation supports constrained user-facing metadata
The CLI SHALL let users supply optional immutable lock metadata through `--name` and `--description` on `ccd-wallet token lock create`. The CLI SHALL encode supplied values as the corresponding name and description fields of the protocol lock metadata model and SHALL NOT expose an interface for authoring arbitrary additional metadata fields. When neither value is present, the CLI SHALL create the lock without a metadata payload. The final creation review SHALL show the resolved metadata values before confirmation.

#### Scenario: User creates a lock with inline name and description
- **WHEN** a user runs `ccd-wallet token lock create` with `--name "Treasury vesting"` and `--description "Quarterly contributor allocation"`
- **THEN** the submitted lock configuration contains metadata with that name and description
- **AND** the creation review displays both values before confirmation

#### Scenario: User creates a lock with one metadata field
- **WHEN** a user supplies only `--name` or only `--description`
- **THEN** the submitted lock metadata contains the supplied field
- **AND** the other model field remains absent

#### Scenario: Interactive lock creation prompts for omitted metadata
- **WHEN** a user runs `ccd-wallet token lock create` interactively without one or both metadata options
- **THEN** the CLI presents empty-allowed prompts for each omitted name or description
- **AND** an empty response records that field as absent

#### Scenario: Non-interactive lock creation omits metadata
- **WHEN** a user runs `ccd-wallet token lock create --non-interactive` without `--name` and `--description`
- **THEN** the CLI does not prompt for metadata
- **AND** the submitted lock configuration has no metadata payload

#### Scenario: Lock creation does not accept arbitrary metadata fields
- **WHEN** a user attempts to supply lock metadata other than `--name` or `--description`
- **THEN** the CLI does not encode that input as lock metadata
- **AND** no generic arbitrary-field metadata authoring interface is available

### Requirement: Token lock inspection displays recognized metadata
The CLI SHALL display the recognized name and description fields from lock metadata when running `ccd-wallet token lock show`. Absent metadata SHALL be represented clearly. Metadata that cannot be decoded as the recognized lock metadata model SHALL be reported as unrecognized without preventing the remaining lock information from being displayed.

#### Scenario: User shows lock with recognized metadata
- **WHEN** a user shows a lock whose metadata contains a name and description
- **THEN** the lock summary displays that name and description
- **AND** the summary continues to display the lock's recipients, controller, and funds

#### Scenario: User shows lock without metadata
- **WHEN** a user shows a lock with no metadata payload
- **THEN** the lock summary clearly reports that metadata is absent

#### Scenario: User shows lock with unrecognized raw metadata
- **WHEN** a user shows a lock whose raw metadata cannot be decoded as the recognized lock metadata model
- **THEN** the lock summary reports the metadata as unrecognized
- **AND** still displays the remaining lock information
