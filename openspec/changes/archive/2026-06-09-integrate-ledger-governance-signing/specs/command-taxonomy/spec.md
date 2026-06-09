## ADDED Requirements

### Requirement: Governance Ledger signing is documented under governance update
The canonical command taxonomy SHALL document Ledger-backed on-chain governance signing as part of the implemented `governance update` command surface.

#### Scenario: Contributor reviews governance Ledger signing command
- **WHEN** a contributor reads `docs/commands.md`
- **THEN** they can find `governance update --ledger` documented under the governance command space
- **AND** they can see that Ledger-backed governance signing uses the Concordium Governance Ledger app rather than the local governance key vault

#### Scenario: Contributor reviews unsupported mixed signing
- **WHEN** a contributor reads the governance update documentation in `docs/commands.md`
- **THEN** they can see that Ledger governance signing is exclusive for a command invocation
- **AND** they can see that local governance key vault signatures are not mixed with Ledger signatures
