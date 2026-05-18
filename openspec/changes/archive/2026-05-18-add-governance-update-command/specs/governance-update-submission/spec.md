## ADDED Requirements

### Requirement: Governance updates can be submitted from JSON payload files
The CLI SHALL provide `ccd-wallet governance update --json <FILE>` for governance update submission from a JSON payload file. If the JSON payload is omitted in interactive mode, the CLI SHALL allow the operator to paste JSON directly.

#### Scenario: Submit governance update from JSON file
- **WHEN** the user runs `ccd-wallet governance update --json payload.json`
- **THEN** the CLI reads the JSON payload from `payload.json`
- **AND** attempts to deserialize it into a governance update payload before signing

#### Scenario: Interactive JSON mode accepts pasted payload
- **WHEN** the user runs `ccd-wallet governance update --json` in interactive mode without a file path
- **THEN** the CLI prompts for JSON payload input
- **AND** accepts pasted JSON directly for the update payload

#### Scenario: Create PLT JSON initialization parameters are converted to CBOR
- **WHEN** the user supplies a JSON governance update payload for `createPlt`
- **AND** the payload's `initializationParameters` field is a JSON value rather than a hex string
- **THEN** the CLI converts that field to Concordium CBOR bytes before signing
- **AND** leaves the behavior of other update types unchanged

### Requirement: Governance updates can be submitted from serialized payloads
The CLI SHALL provide `ccd-wallet governance update --serialized <HEX>` for governance update submission from a serialized hex payload. If the serialized payload is omitted in interactive mode, the CLI SHALL allow the operator to paste hex directly.

#### Scenario: Submit governance update from serialized payload
- **WHEN** the user runs `ccd-wallet governance update --serialized <HEX>`
- **THEN** the CLI decodes the supplied hex payload bytes
- **AND** attempts to deserialize them into a governance update payload before signing

#### Scenario: Interactive serialized mode accepts pasted hex
- **WHEN** the user runs `ccd-wallet governance update --serialized` in interactive mode without a hex argument
- **THEN** the CLI prompts for serialized payload input
- **AND** accepts pasted hex directly for the update payload

### Requirement: Unknown serialized governance updates can be blind-signed
If the wallet cannot deserialize a serialized governance update payload, the CLI SHALL still support signing it through an explicit blind-sign flow.

#### Scenario: Blind-sign unknown serialized payload with explicit keys and sequence number
- **WHEN** the user runs `ccd-wallet governance update --serialized <HEX> --blind --key <VERIFY_KEY> --sequence-number <N>`
- **AND** the serialized payload cannot be deserialized by the wallet
- **THEN** the CLI warns that the update is being blind signed
- **AND** still permits signing with the selected local governance key material
- **AND** uses the explicitly supplied sequence number

#### Scenario: Optional sign-as helper assists blind signing
- **WHEN** the user runs `ccd-wallet governance update --serialized <HEX> --blind --sign-as <AUTH_FAMILY>`
- **AND** the serialized payload cannot be deserialized by the wallet
- **THEN** the CLI treats `--sign-as` as an authorization-family hint rather than a requirement
- **AND** may use it to derive eligible keys, threshold, and default sequence number behavior

### Requirement: Governance update signer selection is explicit and interactive
The CLI SHALL support explicit signer selection through repeatable `--key <VERIFY_KEY>` flags. When signer keys are omitted in interactive mode, the CLI SHALL present a fuzzy multiselect prompt over local governance keys.

#### Scenario: Explicit keys are used for signing
- **WHEN** the user runs `ccd-wallet governance update ... --key <VERIFY_KEY_A> --key <VERIFY_KEY_B>`
- **THEN** the CLI signs using the selected governance keys if they are available locally

#### Scenario: Interactive signer selection uses fuzzy multiselect
- **WHEN** the user runs `ccd-wallet governance update ...` in interactive mode without any `--key` flags
- **THEN** the CLI presents a fuzzy multiselect prompt over candidate governance keys

#### Scenario: Threshold-sized signer set is preselected when authorization is known
- **WHEN** the user runs `ccd-wallet governance update ...` in interactive mode
- **AND** the CLI knows the authorization structure and threshold for the update
- **THEN** the signer prompt preselects authorized local keys up to the required threshold

### Requirement: Governance update prompts reuse governance-key presentation patterns
Governance update signer prompts SHALL reuse the governance-key-management display style so signers are easy to identify.

#### Scenario: Signer prompt uses tag-first governance key rows
- **WHEN** the CLI prompts for governance update signers
- **THEN** each candidate row begins with the same authorization tag style used by governance key listing
- **AND** displayed verify keys are compact by default
- **AND** capability summaries are shown when known

#### Scenario: Signer prompt sorts authorized keys first
- **WHEN** the CLI prompts for governance update signers for a known authorization context
- **THEN** keys authorized for the specific update are shown before other stored governance keys
- **AND** keys within each authorization bucket are sorted by verify key

### Requirement: Governance update timing inputs accept multiple formats
Effective time and timeout inputs SHALL accept relative durations, RFC3339 datetimes, and unix seconds.

#### Scenario: Relative duration is accepted for effective time or timeout
- **WHEN** the user supplies `5m`, `30m`, `1h`, or `15d` as effective time or timeout input
- **THEN** the CLI parses it as a time relative to now

#### Scenario: RFC3339 timestamp is accepted for effective time or timeout
- **WHEN** the user supplies an RFC3339 datetime for effective time or timeout
- **THEN** the CLI parses it as an absolute timestamp

#### Scenario: Unix seconds are accepted for effective time or timeout
- **WHEN** the user supplies unix seconds for effective time or timeout
- **THEN** the CLI parses it as an absolute timestamp

### Requirement: Governance update omitted timing values are prompted with defaults
Effective time SHALL be promptable with a default of `0`, and timeout SHALL be promptable with a default derived from the effective time.

#### Scenario: Omitted effective time prompts with zero default
- **WHEN** the user runs `ccd-wallet governance update ...` without specifying effective time
- **THEN** the CLI prompts for effective time
- **AND** the prompt default is `0`

#### Scenario: Omitted timeout prompts with five-minute future default for immediate updates
- **WHEN** the user runs `ccd-wallet governance update ...` without specifying timeout
- **AND** the effective time is `0`
- **THEN** the CLI prompts for timeout
- **AND** the prompt default is five minutes in the future displayed in RFC3339 format

#### Scenario: Omitted timeout prompts with five-minute-before default for scheduled updates
- **WHEN** the user runs `ccd-wallet governance update ...` without specifying timeout
- **AND** the effective time is a nonzero future timestamp
- **THEN** the CLI prompts for timeout
- **AND** the prompt default is five minutes before the effective time displayed in RFC3339 format

#### Scenario: Timeout must be in the future
- **WHEN** the user supplies a timeout that is not greater than the current time
- **THEN** the CLI rejects the input before submission

#### Scenario: Scheduled update requires timeout not after effective time
- **WHEN** the user supplies a nonzero effective time
- **AND** the supplied timeout is after the effective time
- **THEN** the CLI rejects the input before submission

### Requirement: Governance update submission waits for finalization by default
After successful governance update submission, the CLI SHALL wait for finalization by default and support `--no-wait` to return immediately after submission.

#### Scenario: Default submission waits for finalization
- **WHEN** the user submits a governance update without `--no-wait`
- **THEN** the CLI waits for the submitted update to finalize before returning success

#### Scenario: No-wait returns after submission
- **WHEN** the user submits a governance update with `--no-wait`
- **THEN** the CLI returns after successful submission without waiting for finalization
- **AND** still prints the submitted transaction hash or equivalent submission identifier
