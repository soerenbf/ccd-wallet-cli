## MODIFIED Requirements

### Requirement: Ledger command space includes app inspection
The canonical command taxonomy SHALL document a Ledger app inspection command under the `ledger` command space. The command SHALL let users inspect the connected Concordium Ledger app name and, when the app supports it, the app version used for version-gated Ledger flows such as identity issuance.

#### Scenario: Contributor reviews Ledger command grouping
- **WHEN** a contributor reads the Ledger section of `docs/commands.md`
- **THEN** they can find a Ledger app inspection command such as `ledger show`
- **AND** they can see that it reports connected app name and app version when supported
- **AND** they can understand that Ledger identity issuance requires a sufficiently recent Concordium Ledger app version
