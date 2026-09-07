## ADDED Requirements

### Requirement: Node command taxonomy includes chain-parameter inspection
The canonical command taxonomy SHALL document `node chain-parameters` as an implemented read-only command alongside `node info`.

#### Scenario: Contributor reviews node command grouping
- **WHEN** a contributor reads `docs/commands.md`
- **THEN** they can find `node info` and `node chain-parameters` under the implemented `node` command space
- **AND** they can identify chain-parameters as a block-selectable read-only query with human and JSON output

### Requirement: Node command taxonomy includes chain-update inspection
The canonical command taxonomy SHALL document `node chain-updates` as an implemented read-only command for pending governance updates and next update sequence numbers.

#### Scenario: Contributor reviews chain update inspection
- **WHEN** a contributor reads `docs/commands.md`
- **THEN** they can find `node chain-updates` under the implemented `node` command space
- **AND** they can identify it as a block-selectable, snapshot-consistent query with human and JSON output
