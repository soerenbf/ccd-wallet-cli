## MODIFIED Requirements

### Requirement: Staking taxonomy groups validator and delegation flows
The canonical command taxonomy SHALL document staking as a grouped command space that separates validator-oriented flows from delegation-oriented flows. The implemented taxonomy SHALL expose a top-level `stake` command surface with generic `show` and `remove` actions, plus a nested `configure` area that includes a delegation branch and reserves a validator branch for validator-oriented staking flows.

#### Scenario: Contributor reviews staking command grouping
- **WHEN** a contributor reads the staking section of `docs/commands.md`
- **THEN** they can identify a validator branch and a delegation branch within the staking taxonomy
- **AND** they can understand that the staking area is not a flat list of unrelated stake actions
- **AND** they can identify `stake show`, `stake configure delegation`, and `stake remove` as part of the implemented command surface rather than only planned guidance

## ADDED Requirements

### Requirement: Implemented stake commands are documented under the stake command space
The canonical command taxonomy SHALL document `stake show`, `stake configure delegation`, and `stake remove` as implemented staking commands. It SHALL also reserve `stake configure validator` as the validator-oriented configuration branch for future work.

#### Scenario: Contributor reviews implemented stake command grouping
- **WHEN** a contributor reads the staking section of `docs/commands.md`
- **THEN** they can find `stake show`, `stake configure delegation`, and `stake remove` documented as implemented commands
- **AND** they can find `stake configure validator` documented as the validator-oriented configuration branch
- **AND** they can identify that validator configuration itself is not yet implemented
