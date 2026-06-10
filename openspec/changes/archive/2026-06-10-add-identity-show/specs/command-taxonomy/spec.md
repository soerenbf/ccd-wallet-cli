## ADDED Requirements

### Requirement: Identity inspection and export are documented under the identity command space
The canonical command taxonomy SHALL document `identity show` and `identity export` as implemented identity commands for local identity inspection and explicit JSON export.

#### Scenario: Contributor reviews identity command grouping
- **WHEN** a contributor reads `docs/commands.md`
- **THEN** they can find `identity show` and `identity export` documented alongside identity list, issue, and rename commands
- **AND** they can identify both as implemented commands
