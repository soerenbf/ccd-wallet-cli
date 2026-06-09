## ADDED Requirements

### Requirement: Transaction show can reveal the original submitted payload on request
The CLI SHALL support `ccd-wallet transaction show <HASH> --show-payload` as an opt-in diagnostic mode that attempts to display the original submitted block item payload in addition to the existing lifecycle and summary output.

If the resolved node reports the transaction as committed or finalized, the CLI SHALL retrieve the matching block item payload from the referenced block contents and render it. If the transaction is absent or only received, the CLI SHALL keep the normal status output and SHALL explain that the original submitted payload is not available from block contents yet.

#### Scenario: Finalized transaction shows submitted payload
- **WHEN** the user runs `ccd-wallet transaction show <HASH> --show-payload`
- **AND** the resolved node reports the transaction as finalized
- **THEN** the CLI prints the normal finalized status and summary output
- **AND** prints a distinct section containing the original submitted transaction payload

#### Scenario: Committed transaction shows submitted payload for each committed block section
- **WHEN** the user runs `ccd-wallet transaction show <HASH> --show-payload`
- **AND** the resolved node reports the transaction as committed in one or more blocks
- **THEN** the CLI prints the normal committed status and per-block summary output
- **AND** includes the original submitted transaction payload together with each matching committed block section

#### Scenario: Received transaction explains payload is not yet available
- **WHEN** the user runs `ccd-wallet transaction show <HASH> --show-payload`
- **AND** the resolved node reports the transaction status as received
- **THEN** the CLI prints `Status: received`
- **AND** explains that the original submitted payload cannot be shown until the transaction is included in a block

#### Scenario: Absent transaction does not invent a payload section
- **WHEN** the user runs `ccd-wallet transaction show <HASH> --show-payload`
- **AND** the resolved node does not know the transaction hash
- **THEN** the CLI prints `Status: absent`
- **AND** does not print a submitted-payload section

### Requirement: Submitted payload rendering is explicit and resilient
When `--show-payload` is supplied and a matching block item is available, the CLI SHALL render the original payload under a heading that is distinct from summary-derived sections already used by `transaction show`.

For account transactions, the submitted-payload rendering SHALL include the transaction header together with the payload. The CLI SHALL prefer a decoded structured representation when the payload can be interpreted under the current SDK types. If the payload cannot be decoded into a richer known form, the CLI SHALL still show a stable fallback representation instead of failing the entire command.

#### Scenario: Chain update output distinguishes summary payload from submitted payload
- **WHEN** the user runs `ccd-wallet transaction show <HASH> --show-payload`
- **AND** the resolved node reports a finalized chain update
- **THEN** the CLI keeps the existing summary-derived `Payload:` section for the update summary
- **AND** prints the original submitted block item payload under a different heading

#### Scenario: Account transaction payload output includes the header
- **WHEN** the user runs `ccd-wallet transaction show <HASH> --show-payload`
- **AND** the matching block item is an account transaction
- **THEN** the submitted-payload section includes the transaction header
- **AND** includes the account transaction payload

#### Scenario: Undecodable payload falls back without failing the command
- **WHEN** the user runs `ccd-wallet transaction show <HASH> --show-payload`
- **AND** the transaction payload bytes can be retrieved from block contents
- **AND** the wallet cannot decode those bytes into a richer structured payload view
- **THEN** the CLI still exits successfully
- **AND** prints a stable fallback representation of the submitted payload
