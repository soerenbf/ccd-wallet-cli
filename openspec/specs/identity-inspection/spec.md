# identity-inspection Specification

## Purpose
TBD - created by archiving change add-identity-show. Update Purpose after archive.
## Requirements
### Requirement: Identity show inspects a stored identity by label or interactive selection
The CLI SHALL provide `ccd-wallet identity show [LABEL] [--network <LABEL>]` to inspect a stored identity selected by explicit label or interactive picker.

Identity selection SHALL follow the same ambiguity convention as `identity rename`. If the supplied label matches exactly one stored identity in the requested scope, the CLI SHALL use that identity directly. If the supplied label matches multiple stored identities and the command can prompt, the CLI SHALL require interactive disambiguation through a fuzzy selector. If no identity label is supplied and the command can prompt, the CLI SHALL open a fuzzy selector over stored identities, optionally filtered by `--network <LABEL>`. If ambiguity or missing selection cannot be resolved interactively, the CLI SHALL fail with an actionable error instead of guessing.

#### Scenario: Show identity with unique label
- **WHEN** the user runs `ccd-wallet identity show my-id`
- **AND** exactly one stored identity has label `my-id`
- **THEN** the CLI selects that identity for inspection

#### Scenario: Show identity with ambiguous label interactively
- **WHEN** the user runs `ccd-wallet identity show my-id`
- **AND** multiple stored identities have label `my-id`
- **AND** the command runs interactively
- **THEN** the CLI opens a fuzzy selector using label, network, and key-source metadata to disambiguate the matches
- **AND** the selected identity is used for inspection

#### Scenario: Show identity with ambiguous label when prompting is unavailable
- **WHEN** the user runs `ccd-wallet identity show my-id`
- **AND** multiple stored identities have label `my-id`
- **AND** the command cannot prompt for interactive disambiguation
- **THEN** the CLI exits with an actionable ambiguity error
- **AND** it does not guess which identity to inspect

#### Scenario: Show identity without label prompts for identity
- **WHEN** the user runs `ccd-wallet identity show`
- **AND** the command can prompt
- **THEN** the CLI opens a fuzzy selector over stored identities
- **AND** the selected identity is used for inspection

#### Scenario: Show identity without label filters picker by network
- **WHEN** the user runs `ccd-wallet identity show --network testnet`
- **AND** the command can prompt
- **THEN** the CLI opens a fuzzy selector over stored identities on `testnet`
- **AND** identities from other networks are not offered

### Requirement: Identity show always requires key-source authentication
`identity show` SHALL authenticate the user against the owning key source's local password domain before revealing private identity payload data.

#### Scenario: Correct key-source password reveals selected identity
- **WHEN** the user runs `ccd-wallet identity show my-id`
- **AND** enters the correct password for the identity's owning key source
- **THEN** the CLI unlocks the owning signer-owner vault
- **AND** decrypts the identity private payload
- **AND** continues to the sensitive reveal view

#### Scenario: Wrong key-source password does not reveal selected identity
- **WHEN** the user runs `ccd-wallet identity show my-id`
- **AND** enters an incorrect password for the identity's owning key source
- **THEN** key-source unlock fails
- **AND** the CLI does not reveal any decrypted identity data

### Requirement: Identity show uses a temporary sensitive reveal view
The human output for `identity show` SHALL use a temporary terminal reveal view rather than normal command output.

The temporary reveal view SHALL follow the same sensitive-display model used for seed phrase reveal: it SHALL be temporary, it SHALL hide the content when the user presses any key or after 30 seconds, whichever happens first, and it SHALL be intended to avoid terminal scrollback and session persistence as far as that existing reveal mechanism does.

The reveal view SHALL render the complete selected identity, including plaintext metadata and the issued identity object when present. For pending identities, the reveal view SHALL also render the stored `code_uri`. For completed identities, the reveal view SHALL NOT render `code_uri` because it is no longer retained after completion. The human reveal view SHALL NOT include raw internal identifiers such as signer-owner id or network genesis hash.

#### Scenario: Completed identity uses temporary sensitive reveal view
- **WHEN** the user runs `ccd-wallet identity show my-id`
- **AND** the selected identity is completed
- **AND** an issued identity object is stored
- **THEN** the CLI enters a temporary sensitive reveal view for the selected identity
- **AND** renders identity metadata and the issued identity object there
- **AND** hides the sensitive content when the user presses any key or after 30 seconds, whichever happens first

#### Scenario: Pending identity uses temporary sensitive reveal view without issued object
- **WHEN** the user runs `ccd-wallet identity show my-id`
- **AND** the selected identity is pending
- **THEN** the CLI enters a temporary sensitive reveal view for the selected identity
- **AND** renders identity metadata and decrypted `code_uri`
- **AND** clearly indicates that no issued identity object is stored yet

### Requirement: Identity show renders the issued identity object as flattened key/value lines
When an issued identity object is present, the sensitive reveal view SHALL render it as line-by-line key/value output rather than raw JSON.

The renderer SHALL flatten nested values deterministically: object keys SHALL be visited in sorted key order, arrays SHALL be visited in index order, nested object paths SHALL use `.` separators, and array elements SHALL use `[index]` notation. Scalar leaf values SHALL render on single lines, and empty objects or arrays SHALL render as `{}` or `[]` at their path.

The human reveal view SHALL show only user-facing identity attributes from `value.attributeList`. The visible identity-object paths SHALL be limited to `value.attributeList.chosenAttributes.*`, `value.attributeList.createdAt`, `value.attributeList.maxAccounts`, and `value.attributeList.validTo`. The human view SHALL render only the final key segment, such as `countryOfResidence: DK` rather than the full path. This filtering applies only to the human reveal view; `identity export` SHALL preserve the full identity object JSON.

#### Scenario: Completed identity object is rendered as key/value lines
- **WHEN** the user runs `ccd-wallet identity show my-id`
- **AND** an issued identity object is present
- **THEN** the sensitive reveal view renders the issued identity object as flattened line-by-line key/value output
- **AND** nested object paths use `.` separators
- **AND** array elements use `[index]` notation
- **AND** it omits identity-object paths outside `value.attributeList.chosenAttributes.*`, `value.attributeList.createdAt`, `value.attributeList.maxAccounts`, and `value.attributeList.validTo`
- **AND** it does not display the raw JSON serialization as the human view

### Requirement: Identity show is not available as a non-prompting authenticated flow yet
The CLI SHALL NOT provide a fully non-prompting authenticated `identity show` flow until a deliberate password-input mechanism for such usage is designed.

#### Scenario: Show request fails when authentication cannot prompt
- **WHEN** the user runs `ccd-wallet identity show my-id`
- **AND** the command cannot prompt for the owning key-source password
- **THEN** the CLI exits with an actionable error explaining that prompted identity inspection is required
