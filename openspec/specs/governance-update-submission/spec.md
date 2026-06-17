# governance-update-submission Specification

## Purpose
TBD - created by archiving change add-governance-update-command. Update Purpose after archive.
## Requirements
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

### Requirement: Governance updates can be submitted with Ledger-backed signatures
The governance update submission flow SHALL accept Ledger-backed signatures as an alternative to local governance key vault signatures when `--ledger` is selected, while preserving the same network resolution, sequence-number resolution, timing, submission, and finalization behavior used by local signing.

#### Scenario: Ledger-signed update uses existing submission flow
- **WHEN** the user runs `ccd-wallet governance update --ledger --json payload.json`
- **AND** Ledger signing succeeds with enough authorized governance signatures
- **THEN** the CLI submits the signed governance update to the resolved node endpoint
- **AND** waits for finalization unless `--no-wait` is supplied

#### Scenario: Ledger-signed update honors no-wait
- **WHEN** the user runs `ccd-wallet governance update --ledger --json payload.json --no-wait`
- **AND** Ledger signing and node submission succeed
- **THEN** the CLI returns after successful submission without waiting for finalization
- **AND** prints the submitted transaction hash or equivalent submission identifier

### Requirement: Governance update preparation is shared across signer backends
The governance update submission implementation SHALL prepare update payload, timing, sequence number, and chain authorization context before invoking either the local signer backend or the Ledger signer backend.

#### Scenario: Local backend receives the prepared update context
- **WHEN** the user submits a governance update without `--ledger`
- **THEN** the CLI signs the prepared update context with selected local governance key material
- **AND** preserves existing local governance signing behavior

#### Scenario: Ledger backend receives the prepared update context
- **WHEN** the user submits a governance update with `--ledger`
- **THEN** the CLI signs the prepared update context with the connected Governance Ledger app
- **AND** assembles the returned signatures into the same final governance update instruction structure used for submission

### Requirement: Governance proposal files can be created from JSON payload files
The CLI SHALL provide `ccd-wallet governance proposal create --json <FILE>` to create a detached governance proposal file from a governance update JSON payload. The created proposal file SHALL contain exactly a version number, the selected network genesis hash, the frozen update header, and the governance update payload JSON.

#### Scenario: Create proposal from JSON file
- **WHEN** the user runs `ccd-wallet governance proposal create --json payload.json --out proposal.json`
- **THEN** the CLI reads `payload.json` as a governance update payload
- **AND** resolves the selected network and current update sequence number from the node
- **AND** writes a proposal file containing `version`, `genesisHash`, `header`, and `payload`

#### Scenario: Create proposal requires explicit timing values
- **WHEN** the user runs `ccd-wallet governance proposal create --json payload.json`
- **AND** effective time or timeout has not been provided yet
- **THEN** the CLI requires the operator to enter both timing values explicitly
- **AND** does not apply detached-proposal defaults for either timing field

#### Scenario: Create proposal writes canonical pretty JSON
- **WHEN** the CLI writes a governance proposal file
- **THEN** it emits canonical pretty JSON derived from the parsed proposal data model
- **AND** does not preserve the original formatting of the input payload file

#### Scenario: Proposal payload size matches the frozen header
- **WHEN** the CLI writes a governance proposal file
- **THEN** it encodes the parsed governance update payload with the SDK update encoder
- **AND** stores a header whose `payloadSize` matches the encoded payload bytes

### Requirement: Detached governance proposals are revalidated before signing or submission
The CLI SHALL treat governance proposal files as frozen signing material but SHALL revalidate them online before detached signing or submission.

#### Scenario: Sign rejects proposal for the wrong network
- **WHEN** the user runs `ccd-wallet governance proposal sign proposal.json ...`
- **AND** the proposal file `genesisHash` does not match the selected network genesis hash
- **THEN** the CLI rejects the proposal before attempting to sign it

#### Scenario: Submit rejects proposal for the wrong network
- **WHEN** the user runs `ccd-wallet governance proposal submit proposal.json ...`
- **AND** the proposal file `genesisHash` does not match the selected network genesis hash
- **THEN** the CLI rejects the proposal before attempting to submit it

#### Scenario: Sign rejects stale proposal sequence number
- **WHEN** the user runs `ccd-wallet governance proposal sign proposal.json ...`
- **AND** the proposal header sequence number is no longer the current next sequence number for the payload's update queue
- **THEN** the CLI rejects the proposal as stale before producing a detached signature

#### Scenario: Submit rejects stale proposal sequence number
- **WHEN** the user runs `ccd-wallet governance proposal submit proposal.json ...`
- **AND** the proposal header sequence number is no longer the current next sequence number for the payload's update queue
- **THEN** the CLI rejects the proposal as stale before sending anything to the node

### Requirement: Detached governance signatures can be submitted with a proposal
The CLI SHALL provide `ccd-wallet governance proposal submit` to assemble a governance update from a proposal file plus one or more detached signature files and submit the resulting update to the node. The command SHALL accept detached signature input through repeated `--signature <FILE>` flags and through a directory input containing detached signature files.

#### Scenario: Submit detached signatures that satisfy threshold
- **WHEN** the user runs `ccd-wallet governance proposal submit proposal.json --signature sig-a.json --signature sig-b.json`
- **AND** the provided detached signatures are valid for the proposal signing hash
- **AND** the currently authorized signer set and threshold accept those signatures
- **THEN** the CLI assembles an update instruction and submits it to the resolved node

#### Scenario: Submit reads detached signatures from a directory
- **WHEN** the user runs `ccd-wallet governance proposal submit proposal.json --signature-dir signatures/`
- **AND** the directory contains detached signature files
- **THEN** the CLI loads detached signatures from that directory before threshold validation and submission

#### Scenario: Submitted detached proposal waits for finalization by default
- **WHEN** the user submits a detached governance proposal without `--no-wait`
- **THEN** the CLI waits for the governance update to finalize before returning success

#### Scenario: Submitted detached proposal honors no-wait
- **WHEN** the user submits a detached governance proposal with `--no-wait`
- **THEN** the CLI returns after successful submission without waiting for finalization
- **AND** still prints the submitted transaction hash or equivalent submission identifier

### Requirement: Detached signature files are minimal and signer-indexed
The CLI SHALL emit detached governance signature files that contain exactly a version number, the signer's governance verify key, and a `signature` object matching the logical `UpdateInstructionSignature` JSON shape. Each detached signature file SHALL contain exactly one signature entry keyed by the resolved `UpdateKeysIndex` for the signing verify key.

#### Scenario: Local detached signature stores resolved signer index
- **WHEN** the user signs a proposal with a local governance key
- **THEN** the CLI resolves the current signer index for that verify key from the node authorization context
- **AND** writes a detached signature file whose `signature.signatures` map contains exactly one entry for that index

#### Scenario: Detached signature file is written as canonical pretty JSON
- **WHEN** the CLI writes a detached governance signature file
- **THEN** it emits canonical pretty JSON derived from the parsed signature data model
- **AND** does not write a minified or formatting-preserving variant

#### Scenario: Submit rejects detached signature whose index does not match verify key
- **WHEN** the user runs `ccd-wallet governance proposal submit proposal.json --signature sig.json`
- **AND** the detached signature file's `verifyKey` no longer maps to the stored signature index in the current authorization context
- **THEN** the CLI rejects the detached signature before submission

#### Scenario: Submit rejects detached signatures below threshold
- **WHEN** the user runs `ccd-wallet governance proposal submit proposal.json --signature sig-a.json`
- **AND** the currently required threshold for the proposal update family is greater than the number of valid detached signatures provided
- **THEN** the CLI rejects the submission before sending anything to the node

### Requirement: Governance submissions require interactive review approval
The CLI SHALL render a governance update review and require explicit interactive approval before submitting an update to a node from `governance update` or `governance proposal submit`. The approval prompt SHALL use a cliclack yes/no confirmation prompt, SHALL initially select the non-submitting option, and SHALL decline submission without treating the command as a failure when the user chooses No.

#### Scenario: Interactive governance update is reviewed before submission
- **WHEN** the user runs `ccd-wallet governance update` in interactive mode with valid update input and signer selections
- **THEN** the CLI renders a review of the governance update before submitting it to the node
- **AND** prompts the user with a cliclack yes/no confirmation to approve submission

#### Scenario: Declined governance update is not submitted
- **WHEN** the user runs `ccd-wallet governance update` in interactive mode
- **AND** the CLI reaches the approval prompt
- **AND** the user chooses No
- **THEN** the CLI does not submit the governance update to the node
- **AND** returns without reporting a submission failure

#### Scenario: Interactive detached proposal submission is reviewed before submission
- **WHEN** the user runs `ccd-wallet governance proposal submit` in interactive mode with a valid proposal and sufficient valid detached signatures
- **THEN** the CLI renders a review of the governance update before submitting it to the node
- **AND** prompts the user with a cliclack yes/no confirmation to approve submission

#### Scenario: Declined detached proposal submission is not submitted
- **WHEN** the user runs `ccd-wallet governance proposal submit` in interactive mode
- **AND** the CLI reaches the approval prompt
- **AND** the user chooses No
- **THEN** the CLI does not submit the detached governance proposal to the node
- **AND** returns without reporting a submission failure

#### Scenario: Non-interactive governance update skips approval prompt
- **WHEN** the user runs `ccd-wallet governance update --non-interactive` with all required inputs
- **THEN** the CLI validates, signs, and submits according to existing non-interactive governance update behavior
- **AND** does not prompt for review approval

#### Scenario: Non-interactive detached proposal submission skips approval prompt
- **WHEN** the user runs `ccd-wallet governance proposal submit --non-interactive` with all required inputs
- **THEN** the CLI validates and submits according to existing non-interactive detached proposal behavior
- **AND** does not prompt for review approval

### Requirement: Detached governance proposal signing requires interactive review approval
The CLI SHALL render a governance update review and require explicit interactive approval before producing a detached signature from `governance proposal sign`. The approval prompt SHALL use a cliclack yes/no confirmation prompt, SHALL initially select the non-signing option, and SHALL decline signing without treating the command as a failure when the user chooses No.

#### Scenario: Interactive detached proposal signing is reviewed before signing
- **WHEN** the user runs `ccd-wallet governance proposal sign` in interactive mode with a valid proposal and signer selection
- **THEN** the CLI renders a review of the governance update before producing a detached signature
- **AND** prompts the user with a cliclack yes/no confirmation to approve signing

#### Scenario: Declined detached proposal signing does not write a signature
- **WHEN** the user runs `ccd-wallet governance proposal sign` in interactive mode
- **AND** the CLI reaches the approval prompt
- **AND** the user chooses No
- **THEN** the CLI does not sign the governance proposal
- **AND** does not write the detached signature output file
- **AND** returns without reporting a signing failure

#### Scenario: Non-interactive detached proposal signing skips approval prompt
- **WHEN** the user runs `ccd-wallet governance proposal sign --non-interactive` with all required inputs
- **THEN** the CLI validates and signs according to existing non-interactive detached proposal behavior
- **AND** does not prompt for review approval

### Requirement: Governance reviews show resolved update context
The governance review SHALL include enough resolved information for an operator to validate the update before approving signing or submission, including the selected network, update payload identity, parsed payload details when available, sequence number when resolved, timing, and signer or signature context. For blind serialized payloads, the review SHALL clearly state that the wallet cannot display decoded payload semantics.

#### Scenario: Review includes core update context
- **WHEN** the CLI renders a governance review for a decoded governance update
- **THEN** the review includes the selected network and endpoint label
- **AND** includes the governance update type or authorization family
- **AND** includes parsed payload details derived from the decoded governance update payload
- **AND** includes the resolved sequence number when available
- **AND** includes effective time and timeout values

#### Scenario: Review includes all-in-one signer context
- **WHEN** the CLI renders a governance review for `governance update`
- **THEN** the review includes whether signing will use the local governance vault or Governance Ledger
- **AND** includes the selected local governance verify keys or selected Ledger key index context, as applicable

#### Scenario: Review includes detached signing signer context
- **WHEN** the CLI renders a governance review for `governance proposal sign`
- **THEN** the review includes whether detached signing will use the local governance vault or Governance Ledger
- **AND** includes the selected local governance verify key or selected Ledger key index context, as applicable

#### Scenario: Review includes detached signature context
- **WHEN** the CLI renders a governance review for `governance proposal submit`
- **THEN** the review includes the detached signature indices or equivalent signer context accepted for submission
- **AND** renders the review only after detached signatures have been loaded, verified, and checked against the required threshold

#### Scenario: Ledger signing review supports device comparison
- **WHEN** the CLI renders a governance review before Ledger signing
- **THEN** the review includes parsed payload details when the payload is decoded
- **AND** provides enough payload detail for the operator to compare the CLI output with the details shown on the Ledger device

#### Scenario: Review warns for blind serialized payloads
- **WHEN** the CLI renders a governance review for a blind serialized payload
- **THEN** the review states that the wallet could not decode the payload semantics
- **AND** includes the payload size or equivalent raw payload identifier
- **AND** warns the user to approve only if the payload was produced by trusted tooling and independently reviewed

