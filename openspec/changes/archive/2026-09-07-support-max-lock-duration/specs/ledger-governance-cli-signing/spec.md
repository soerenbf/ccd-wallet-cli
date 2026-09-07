## ADDED Requirements

### Requirement: Known but device-unsupported governance updates fail before Ledger interaction
When the wallet can decode a governance update but the Governance Ledger APDU protocol does not define its update family or authorization version, Ledger-backed all-in-one and detached signing SHALL reject the request actionably before opening or prompting on the device. Local governance signing SHALL remain available.

#### Scenario: Ledger rejects maximum lock duration update
- **WHEN** the user requests Ledger signing for a Max Lock Duration update
- **THEN** the CLI rejects the request before opening a Ledger signing flow
- **AND** explains that the connected Governance Ledger protocol does not support that typed update
- **AND** does not classify the payload as unknown or blind

#### Scenario: Ledger rejects authorization V3 update
- **WHEN** the user requests Ledger signing for a root or level-1 V3 level-2 authorization update
- **THEN** the CLI rejects the request before device interaction
- **AND** explains that authorization V3 is not supported by the Governance Ledger protocol

#### Scenario: Local signing remains available
- **WHEN** the user processes either update family without `--ledger`
- **THEN** the existing local governance signer flow remains available
