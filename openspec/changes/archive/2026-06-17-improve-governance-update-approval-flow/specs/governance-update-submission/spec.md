## ADDED Requirements

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
