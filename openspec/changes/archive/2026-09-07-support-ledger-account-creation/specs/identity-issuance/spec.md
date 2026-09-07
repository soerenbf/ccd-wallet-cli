## ADDED Requirements

### Requirement: Identity issuance supports explicit seed or Ledger source selection
`identity new` SHALL accept `--seed <LABEL>` to preserve named key-source selection and `--ledger` to select the connected Ledger device without requiring its stored label. The options MUST be mutually exclusive. When `--ledger` is supplied, the CLI SHALL connect to the device and match its canonical identity to exactly one enrolled Ledger owner before constructing or storing identity issuance state.

When neither option is supplied, existing default and interactive source resolution SHALL remain available. `--allow-ledger-secret-export` SHALL authorize Ledger secret export only and SHALL NOT select a signing source.

#### Scenario: Named key source is selected
- **WHEN** the user runs `identity new my-identity --seed primary`
- **THEN** the CLI resolves the stored key source named `primary`
- **AND** does not use the connected device to discover which stored owner to select

#### Scenario: Connected Ledger is selected
- **WHEN** the user runs `identity new my-identity --ledger`
- **AND** the connected device matches exactly one enrolled Ledger owner
- **THEN** the CLI uses that owner for identity issuance
- **AND** applies the existing Ledger secret-export approval policy

#### Scenario: Conflicting source options are rejected
- **WHEN** the user supplies both `--seed <LABEL>` and `--ledger`
- **THEN** argument parsing rejects the command before source resolution

#### Scenario: Connected Ledger cannot be resolved uniquely
- **WHEN** `--ledger` is supplied
- **AND** no connected device matches exactly one enrolled Ledger owner
- **THEN** the CLI returns an actionable enrollment or ambiguity error
- **AND** writes no identity state

#### Scenario: Non-interactive Ledger selection is deterministic
- **WHEN** the user supplies `--ledger --non-interactive`
- **AND** the connected device matches exactly one enrolled Ledger owner
- **AND** all other required arguments and secret-export authorization are supplied
- **THEN** the CLI uses the matched owner without prompting
