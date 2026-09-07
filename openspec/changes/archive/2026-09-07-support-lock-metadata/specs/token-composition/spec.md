## ADDED Requirements

### Requirement: Composer supports constrained lock creation metadata
The token compose REPL SHALL support optional `--name` and `--description` inputs for `lock create`, including context-appropriate completion and help. The composer SHALL prompt with empty-allowed input for either omitted field, persist supplied values in the lock-creation plan operation, display them in plan previews, and encode them as the corresponding protocol lock metadata fields during submission. The composer SHALL NOT expose arbitrary additional lock metadata fields.

#### Scenario: User adds lock metadata inline
- **WHEN** a user enters `lock create --name "Treasury vesting" --description "Quarterly contributor allocation"` with the remaining required lock arguments
- **THEN** the composer saves both metadata values on the lock-creation operation
- **AND** completion and help identify `--name` and `--description` as supported lock-create options

#### Scenario: User supplies one lock metadata field inline
- **WHEN** a user supplies only `--name` or only `--description` for a compose lock creation
- **THEN** the composer preserves the supplied value
- **AND** prompts with empty-allowed input for the omitted metadata field

#### Scenario: User supplies lock metadata through prompts
- **WHEN** a user adds a lock creation without inline metadata options
- **THEN** the composer prompts for an optional name and description
- **AND** non-empty responses are saved on the lock-creation operation
- **AND** empty responses are recorded as absent

#### Scenario: Compose preview shows lock metadata
- **WHEN** a composition plan contains a lock creation with a name or description
- **THEN** CLI and interactive plan previews display the supplied metadata alongside the lock-creation summary

#### Scenario: Compose submission encodes lock metadata
- **WHEN** a user submits a composition plan containing a lock name or description
- **THEN** the corresponding lock-create MetaUpdate operation contains metadata with those recognized fields
- **AND** arbitrary additional metadata fields are not encoded

#### Scenario: Metadata-free version-1 plan remains compatible
- **WHEN** the composer opens, previews, or submits an existing version-1 plan whose lock creation has no name or description
- **THEN** the plan remains valid without migration
- **AND** its lock-create operation is submitted without a metadata payload
