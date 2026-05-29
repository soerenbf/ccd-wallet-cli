# contract-module-deployment Specification

## Purpose
Define first-class CLI smart contract module deployment, including wallet-resolved network and signer context, duplicate-module validation, explicit user approval, submission, and optional finalization waiting.

## Requirements
### Requirement: CLI supports deploy-module transactions from a local module file
The CLI SHALL provide a `ccd-wallet contract deploy-module <file-path>` command that reads a local smart contract module file, presents a deploy review prompt, and submits a deploy-module transaction only after explicit user approval.

The command SHALL wait for finalization by default and SHALL support `--no-wait` to exit immediately after successful submission with the transaction hash.

If the supplied file cannot be read or does not contain a valid Concordium module, the command SHALL fail before any approval prompt or transaction submission occurs.

#### Scenario: User approves deployment from a module file
- **WHEN** the user runs `ccd-wallet contract deploy-module ./counter.wasm.v1`
- **AND** the file contains a valid Concordium smart contract module
- **AND** the user approves the review prompt
- **THEN** the CLI submits a deploy-module transaction
- **AND** prints the submitted transaction hash

#### Scenario: User submits deployment with no-wait enabled
- **WHEN** the user runs `ccd-wallet contract deploy-module ./counter.wasm.v1 --no-wait`
- **AND** the file contains a valid Concordium smart contract module
- **AND** the user approves the review prompt
- **THEN** the CLI submits a deploy-module transaction
- **AND** prints the submitted transaction hash
- **AND** exits without waiting for finalization

#### Scenario: Invalid module file is rejected before approval
- **WHEN** the user runs `ccd-wallet contract deploy-module ./broken.wasm.v1`
- **AND** the file does not decode as a valid Concordium smart contract module
- **THEN** the CLI exits with an actionable error
- **AND** does not present an approval prompt
- **AND** does not submit any transaction

#### Scenario: User declines deployment
- **WHEN** the user runs `ccd-wallet contract deploy-module ./counter.wasm.v1`
- **AND** the file contains a valid Concordium smart contract module
- **AND** the user declines the review prompt
- **THEN** the CLI exits without submitting any transaction

### Requirement: CLI deploy review uses wallet-resolved network and signer context
The deploy-module command SHALL resolve the target network and signer-capable account through wallet CLI configuration and signing-source logic rather than through module-file contents. The review prompt SHALL display the selected network, resolved account address, derived module reference, and module size in bytes.

#### Scenario: Review prompt shows resolved deploy context
- **WHEN** the user runs `ccd-wallet contract deploy-module ./counter.wasm.v1`
- **AND** the wallet resolves a concrete network and signer-capable account for the command
- **THEN** the review prompt shows the selected network context
- **AND** shows the resolved account address
- **AND** shows the derived module reference and module size

### Requirement: CLI deploy validation is enabled by default and warns on duplicate modules without blocking approval
By default, the wallet SHALL derive the module reference from the module bytes and check whether that module already exists on chain before submission. If the module already exists, the CLI SHALL show a warning stating `Validation warning: module already exists on chain for this network.` and SHALL still allow the user to approve or decline.

If validation cannot be completed because of node unavailability or another transient failure, the CLI SHALL show that result as a warning and SHALL still allow the user to approve or decline. If `--no-validate` is supplied, the CLI SHALL skip the module existence check.

#### Scenario: Default validation finds an existing module on chain
- **WHEN** the user runs `ccd-wallet contract deploy-module ./counter.wasm.v1`
- **AND** the wallet confirms that the derived module reference already exists on chain
- **THEN** the review prompt shows a duplicate-module warning
- **AND** still allows the user to approve or decline the deployment

#### Scenario: Default validation cannot complete before approval
- **WHEN** the user runs `ccd-wallet contract deploy-module ./counter.wasm.v1`
- **AND** the wallet cannot complete the module existence check because the node is unavailable
- **THEN** the review prompt shows a validation warning
- **AND** still allows the user to approve or decline the deployment

#### Scenario: No-validate skips module existence check
- **WHEN** the user runs `ccd-wallet contract deploy-module ./counter.wasm.v1 --no-validate`
- **THEN** the CLI skips duplicate-module validation
- **AND** still shows the deploy review prompt before submission

### Requirement: CLI deploy waits for finalization by default and prints a human-oriented transaction summary
After submitting a deploy-module transaction, the CLI SHALL wait for finalization inline by default and print a human-oriented finalization summary consistent with the wallet's transaction summary rendering.

When `--no-wait` is supplied, the CLI SHALL skip the finalization wait and terminate after printing the submitted transaction hash.

The finalization output SHALL include the finalized block hash, transaction outcome, and deploy-specific details available from the finalized summary.

#### Scenario: Successful deployment prints finalized summary
- **WHEN** the user submits a deploy-module transaction through `ccd-wallet contract deploy-module ./counter.wasm.v1`
- **AND** the transaction finalizes successfully
- **THEN** the CLI waits for finalization before exiting
- **AND** prints the finalized block hash
- **AND** prints a success outcome together with deploy-module summary details

#### Scenario: Rejected deployment prints finalized rejection summary
- **WHEN** the user submits a deploy-module transaction through `ccd-wallet contract deploy-module ./counter.wasm.v1`
- **AND** the transaction finalizes with rejection
- **THEN** the CLI waits for finalization before exiting
- **AND** prints the finalized block hash
- **AND** prints the rejection outcome and reject details from the finalized summary

#### Scenario: No-wait deploy exits after submission
- **WHEN** the user submits a deploy-module transaction through `ccd-wallet contract deploy-module ./counter.wasm.v1 --no-wait`
- **AND** the transaction submission succeeds
- **THEN** the CLI prints the transaction hash
- **AND** exits before any finalization summary is rendered
