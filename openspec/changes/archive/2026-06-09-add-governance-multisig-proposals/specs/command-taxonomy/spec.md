## ADDED Requirements

### Requirement: Governance proposal workflow is documented under the governance command space
The canonical command taxonomy SHALL document a detached governance proposal workflow under the implemented `governance` command space alongside the existing all-in-one `governance update` command.

#### Scenario: Contributor reviews detached governance proposal command grouping
- **WHEN** a contributor reads `docs/commands.md`
- **THEN** they can find a documented `governance proposal` command family
- **AND** they can identify `create`, `sign`, and `submit` as its implemented detached-signing subcommands

#### Scenario: Contributor reviews relationship between proposal and update flows
- **WHEN** a contributor reads the governance section of `docs/commands.md`
- **THEN** they can see that `governance update` remains the all-in-one signing and submission flow
- **AND** they can see that `governance proposal` is the detached multi-party signing flow
