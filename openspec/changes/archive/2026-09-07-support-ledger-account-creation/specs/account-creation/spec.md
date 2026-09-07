## ADDED Requirements

### Requirement: Account creation supports Ledger-owned identities
`account new <LABEL>` SHALL create an account from a stored usable identity owned by an enrolled Ledger signer without requiring or unlocking a local seed. The flow SHALL use the next credential counter in the Ledger owner's identity tuple, require explicit approval before exporting account-creation secret material, and otherwise use the existing pending, submission, and finalization lifecycle.

Interactive mode SHALL present the export warning and request confirmation. Non-interactive mode SHALL require an explicit command option authorizing Ledger secret export. Declining or omitting required approval SHALL fail before secret export, local account insertion, or node submission.

#### Scenario: Interactive Ledger account creation succeeds
- **WHEN** the user selects a usable Ledger-owned identity and approves the secret-export warning
- **AND** the matching connected device supports Ledger account creation
- **AND** the device approves export and credential signing
- **THEN** the CLI submits a normal credential deployment for the next credential counter
- **AND** processes account persistence and finalization through the existing account lifecycle

#### Scenario: Ledger account creation does not request a seed password
- **WHEN** account creation targets a Ledger-owned identity
- **THEN** the CLI uses the Ledger owner and connected device
- **AND** does not resolve, unlock, or derive material from a local seed

#### Scenario: Non-interactive Ledger account creation requires export authorization
- **WHEN** non-interactive account creation targets a Ledger-owned identity
- **AND** explicit Ledger secret-export authorization is absent
- **THEN** the CLI returns an actionable error identifying the required option
- **AND** exports no secrets, inserts no pending account, and submits no transaction

#### Scenario: Ledger account creation failure is safe
- **WHEN** the device is unavailable, mismatched, unsupported, returns malformed material, rejects an operation, or fails to sign
- **THEN** the CLI returns an actionable error
- **AND** submits no unsigned, partially signed, or fallback-signed credential deployment

### Requirement: Account creation supports explicit seed or Ledger source selection
`account new` SHALL accept `--seed <LABEL>` to preserve named key-source selection and `--ledger` to select the connected Ledger device without requiring its stored label. The options MUST be mutually exclusive. When `--ledger` is supplied, the CLI SHALL match the connected device to exactly one enrolled Ledger owner and limit identity resolution to identities owned by that owner.

When neither option is supplied, existing default and interactive source resolution SHALL remain available. `--allow-ledger-secret-export` SHALL authorize export only and SHALL NOT select a signing source.

#### Scenario: Named key source is selected
- **WHEN** the user runs `account new my-account --seed primary`
- **THEN** the CLI resolves the stored key source named `primary`
- **AND** does not use the connected device to discover which stored owner to select

#### Scenario: Connected Ledger is selected
- **WHEN** the user runs `account new my-account --ledger`
- **AND** the connected device matches exactly one enrolled Ledger owner
- **THEN** identity selection is limited to usable identities belonging to that owner

#### Scenario: Conflicting source options are rejected
- **WHEN** the user supplies both `--seed <LABEL>` and `--ledger`
- **THEN** argument parsing rejects the command before source or identity resolution

#### Scenario: Connected Ledger cannot be resolved uniquely
- **WHEN** `--ledger` is supplied
- **AND** no connected device matches exactly one enrolled Ledger owner
- **THEN** the CLI returns an actionable enrollment or ambiguity error
- **AND** creates no account state
