# contract-instance-execution Specification

## Purpose
TBD - created by archiving change expand-contract-commands. Update Purpose after archive.
## Requirements
### Requirement: CLI supports contract initialization transactions
The CLI SHALL provide a `ccd-wallet contract init` command that prepares and submits a smart contract initialization transaction from a module reference, init function name, optional CCD decimal amount, optional maximum contract execution energy, and optional parameter bytes.

The command SHALL resolve the target network and signer-capable account through wallet CLI configuration and signing-source logic. The command SHALL present a review prompt and SHALL submit the transaction only after explicit user approval.

The command SHALL wait for finalization by default and SHALL support `--no-wait` to exit immediately after successful submission with the transaction hash.

#### Scenario: User approves contract initialization
- **WHEN** the user runs `ccd-wallet contract init --module-ref <module-ref> --init-name init_counter --energy 30000`
- **AND** the wallet resolves a signer-capable account and network
- **AND** the user approves the review prompt
- **THEN** the CLI submits a contract initialization transaction
- **AND** prints the submitted transaction hash

#### Scenario: Initialization prompts for energy when omitted
- **WHEN** the user runs `ccd-wallet contract init --module-ref <module-ref> --init-name init_counter`
- **AND** the wallet can simulate the initialization before approval
- **THEN** the CLI prompts the user for an energy amount
- **AND** uses the simulated energy amount as the default prompt value

#### Scenario: Initialization requires explicit energy in non-interactive mode
- **WHEN** the user runs `ccd-wallet contract init --module-ref <module-ref> --init-name init_counter --non-interactive`
- **AND** no energy value is supplied
- **THEN** the CLI exits with an actionable error
- **AND** does not submit any transaction

#### Scenario: User declines contract initialization
- **WHEN** the user runs `ccd-wallet contract init --module-ref <module-ref> --init-name init_counter --energy 30000`
- **AND** the user declines the review prompt
- **THEN** the CLI exits without submitting any transaction

#### Scenario: Initialization waits for finalization by default
- **WHEN** the user approves a `ccd-wallet contract init` transaction
- **AND** the transaction submission succeeds
- **THEN** the CLI waits for finalization before exiting
- **AND** prints a human-oriented finalized transaction summary

#### Scenario: Initialization no-wait exits after submission
- **WHEN** the user approves a `ccd-wallet contract init --no-wait` transaction
- **AND** the transaction submission succeeds
- **THEN** the CLI prints the transaction hash
- **AND** exits before rendering any finalization summary

### Requirement: CLI initialization supports optional parameters and validation
The `contract init` command SHALL accept raw serialized parameter bytes as hex through `--parameter-hex`, schema-shaped inline JSON through `--parameter-json`, and schema-shaped JSON files through `--parameter-json-file`. If JSON is supplied, the command SHALL fetch the module source for the supplied module reference, use the module's embedded schema to serialize the JSON for the init function, and fail with an actionable error if no compatible embedded schema is available. If no parameter is supplied, the command SHALL use an empty parameter.

The command SHALL reject requests that supply more than one of `--parameter-hex`, `--parameter-json`, and `--parameter-json-file`. The command SHALL support validation by simulation before prompting and SHALL show simulation failures as warnings without preventing explicit approval. When energy is omitted in interactive mode, the command SHALL use the same simulation flow to derive the default prompt value when an estimate is available.

#### Scenario: Initialization uses empty parameter by default
- **WHEN** the user runs `ccd-wallet contract init --module-ref <module-ref> --init-name init_counter --energy 30000`
- **THEN** the command prepares the initialization with an empty parameter

#### Scenario: Initialization accepts parameter hex
- **WHEN** the user runs `ccd-wallet contract init --module-ref <module-ref> --init-name init_counter --energy 30000 --parameter-hex 000102`
- **THEN** the command prepares the initialization with parameter bytes decoded from `000102`

#### Scenario: Initialization accepts parameter JSON string
- **WHEN** the user runs `ccd-wallet contract init --module-ref <module-ref> --init-name init_counter --energy 30000 --parameter-json '{"count": 1}'`
- **AND** the module source contains a compatible embedded schema for `init_counter`
- **THEN** the command prepares the initialization with parameter bytes serialized from the JSON value

#### Scenario: Initialization accepts parameter JSON file
- **WHEN** the user runs `ccd-wallet contract init --module-ref <module-ref> --init-name init_counter --energy 30000 --parameter-json-file ./init-params.json`
- **AND** `./init-params.json` contains valid JSON
- **AND** the module source contains a compatible embedded schema for `init_counter`
- **THEN** the command prepares the initialization with parameter bytes serialized from the JSON file contents

#### Scenario: Initialization parameter JSON requires embedded schema
- **WHEN** the user runs `ccd-wallet contract init --module-ref <module-ref> --init-name init_counter --energy 30000 --parameter-json '{"count": 1}'`
- **AND** the module source has no compatible embedded schema for `init_counter`
- **THEN** the CLI exits with an actionable error
- **AND** does not submit any transaction

#### Scenario: Initialization rejects conflicting parameter inputs
- **WHEN** the user runs `ccd-wallet contract init --module-ref <module-ref> --init-name init_counter --energy 30000 --parameter-hex 000102 --parameter-json '{"count": 1}'`
- **THEN** the CLI exits with an actionable error
- **AND** does not submit any transaction

#### Scenario: Initialization rejects multiple JSON parameter sources
- **WHEN** the user runs `ccd-wallet contract init --module-ref <module-ref> --init-name init_counter --energy 30000 --parameter-json '{"count": 1}' --parameter-json-file ./init-params.json`
- **THEN** the CLI exits with an actionable error
- **AND** does not submit any transaction

#### Scenario: Initialization validation warning is non-blocking
- **WHEN** the user runs `ccd-wallet contract init --module-ref <module-ref> --init-name init_counter --energy 30000 --validate`
- **AND** simulation returns a warning or failure
- **THEN** the CLI shows the simulation result before approval
- **AND** still allows the user to approve or decline the transaction

### Requirement: CLI supports contract update transactions
The CLI SHALL provide a `ccd-wallet contract update` command that prepares and submits a smart contract receive-function transaction from a contract instance address, receive name, optional CCD decimal amount, optional maximum contract execution energy, and optional parameter bytes.

The command SHALL resolve the target network and signer-capable account through wallet CLI configuration and signing-source logic. The command SHALL present a review prompt and SHALL submit the transaction only after explicit user approval.

The command SHALL wait for finalization by default and SHALL support `--no-wait` to exit immediately after successful submission with the transaction hash.

#### Scenario: User approves contract update
- **WHEN** the user runs `ccd-wallet contract update --contract 42,0 --receive counter.increment --energy 30000`
- **AND** the wallet resolves a signer-capable account and network
- **AND** the user approves the review prompt
- **THEN** the CLI submits a contract update transaction
- **AND** prints the submitted transaction hash

#### Scenario: Update prompts for energy when omitted
- **WHEN** the user runs `ccd-wallet contract update --contract 42,0 --receive counter.increment`
- **AND** the wallet can simulate the update before approval
- **THEN** the CLI prompts the user for an energy amount
- **AND** uses the simulated energy amount as the default prompt value

#### Scenario: Update requires explicit energy in non-interactive mode
- **WHEN** the user runs `ccd-wallet contract update --contract 42,0 --receive counter.increment --non-interactive`
- **AND** no energy value is supplied
- **THEN** the CLI exits with an actionable error
- **AND** does not submit any transaction

#### Scenario: User declines contract update
- **WHEN** the user runs `ccd-wallet contract update --contract 42,0 --receive counter.increment --energy 30000`
- **AND** the user declines the review prompt
- **THEN** the CLI exits without submitting any transaction

#### Scenario: Update waits for finalization by default
- **WHEN** the user approves a `ccd-wallet contract update` transaction
- **AND** the transaction submission succeeds
- **THEN** the CLI waits for finalization before exiting
- **AND** prints a human-oriented finalized transaction summary

#### Scenario: Update no-wait exits after submission
- **WHEN** the user approves a `ccd-wallet contract update --no-wait` transaction
- **AND** the transaction submission succeeds
- **THEN** the CLI prints the transaction hash
- **AND** exits before rendering any finalization summary

### Requirement: CLI update supports optional parameters and validation
The `contract update` command SHALL accept raw serialized parameter bytes as hex through `--parameter-hex`, schema-shaped inline JSON through `--parameter-json`, and schema-shaped JSON files through `--parameter-json-file`. If JSON is supplied, the command SHALL fetch the instance information, fetch the instance source module, use the module's embedded schema to serialize the JSON for the receive function, and fail with an actionable error if no compatible embedded schema is available. If no parameter is supplied, the command SHALL use an empty parameter.

The command SHALL reject requests that supply more than one of `--parameter-hex`, `--parameter-json`, and `--parameter-json-file`. The command SHALL support validation by simulation before prompting and SHALL show simulation failures as warnings without preventing explicit approval. When energy is omitted in interactive mode, the command SHALL use the same simulation flow to derive the default prompt value when an estimate is available.

#### Scenario: Update uses empty parameter by default
- **WHEN** the user runs `ccd-wallet contract update --contract 42,0 --receive counter.increment --energy 30000`
- **THEN** the command prepares the update with an empty parameter

#### Scenario: Update accepts parameter hex
- **WHEN** the user runs `ccd-wallet contract update --contract 42,0 --receive counter.increment --energy 30000 --parameter-hex 000102`
- **THEN** the command prepares the update with parameter bytes decoded from `000102`

#### Scenario: Update accepts parameter JSON string
- **WHEN** the user runs `ccd-wallet contract update --contract 42,0 --receive counter.increment --energy 30000 --parameter-json '{"delta": 1}'`
- **AND** the instance source module contains a compatible embedded schema for `counter.increment`
- **THEN** the command prepares the update with parameter bytes serialized from the JSON value

#### Scenario: Update accepts parameter JSON file
- **WHEN** the user runs `ccd-wallet contract update --contract 42,0 --receive counter.increment --energy 30000 --parameter-json-file ./update-params.json`
- **AND** `./update-params.json` contains valid JSON
- **AND** the instance source module contains a compatible embedded schema for `counter.increment`
- **THEN** the command prepares the update with parameter bytes serialized from the JSON file contents

#### Scenario: Update parameter JSON requires embedded schema
- **WHEN** the user runs `ccd-wallet contract update --contract 42,0 --receive counter.increment --energy 30000 --parameter-json '{"delta": 1}'`
- **AND** the instance source module has no compatible embedded schema for `counter.increment`
- **THEN** the CLI exits with an actionable error
- **AND** does not submit any transaction

#### Scenario: Update rejects conflicting parameter inputs
- **WHEN** the user runs `ccd-wallet contract update --contract 42,0 --receive counter.increment --energy 30000 --parameter-hex 000102 --parameter-json '{"delta": 1}'`
- **THEN** the CLI exits with an actionable error
- **AND** does not submit any transaction

#### Scenario: Update rejects multiple JSON parameter sources
- **WHEN** the user runs `ccd-wallet contract update --contract 42,0 --receive counter.increment --energy 30000 --parameter-json '{"delta": 1}' --parameter-json-file ./update-params.json`
- **THEN** the CLI exits with an actionable error
- **AND** does not submit any transaction

#### Scenario: Update validation warning is non-blocking
- **WHEN** the user runs `ccd-wallet contract update --contract 42,0 --receive counter.increment --energy 30000 --validate`
- **AND** simulation returns a warning or failure
- **THEN** the CLI shows the simulation result before approval
- **AND** still allows the user to approve or decline the transaction

### Requirement: CLI contract execution commands accept decimal CCD amounts
The `contract init`, `contract update`, and `contract invoke` commands SHALL accept amount values as decimal CCD strings and SHALL convert them to exact microCCD values internally. If no amount is supplied, the commands SHALL use `0` CCD. The commands SHALL reject amount values with more than six fractional decimal places or otherwise invalid decimal syntax.

#### Scenario: Contract initialization accepts decimal CCD amount
- **WHEN** the user runs `ccd-wallet contract init --module-ref <module-ref> --init-name init_counter --energy 30000 --amount 1.25`
- **THEN** the command prepares the initialization with an amount of `1_250_000` microCCD

#### Scenario: Contract update accepts decimal CCD amount
- **WHEN** the user runs `ccd-wallet contract update --contract 42,0 --receive counter.deposit --energy 30000 --amount 0.5`
- **THEN** the command prepares the update with an amount of `500_000` microCCD

#### Scenario: Contract invoke rejects too many decimal places
- **WHEN** the user runs `ccd-wallet contract invoke --contract 42,0 --receive counter.view --amount 0.0000001`
- **THEN** the CLI exits with an actionable error
- **AND** does not invoke the entrypoint

### Requirement: CLI supports read-only contract invocation
The CLI SHALL provide a `ccd-wallet contract invoke` command that invokes a contract entrypoint through a node query without signing, submitting a transaction, unlocking an account, or prompting for transaction approval.

The command SHALL require a contract instance address and receive name. The command SHALL allow optional CCD decimal amount, parameter hex, inline JSON parameter input through `--parameter-json`, JSON file parameter input through `--parameter-json-file`, energy, invoker, network, node endpoint, block selector, and JSON output options.

#### Scenario: User invokes no-argument view entrypoint
- **WHEN** the user runs `ccd-wallet contract invoke --contract 42,0 --receive counter.view`
- **THEN** the CLI invokes the entrypoint without submitting a transaction
- **AND** prints the invocation result

#### Scenario: Invoke uses optional parameter hex
- **WHEN** the user runs `ccd-wallet contract invoke --contract 42,0 --receive counter.balanceOf --parameter-hex 000102`
- **THEN** the CLI invokes the entrypoint with parameter bytes decoded from `000102`

#### Scenario: Invoke uses optional parameter JSON string
- **WHEN** the user runs `ccd-wallet contract invoke --contract 42,0 --receive counter.balanceOf --parameter-json '{"owner": {"Account": ["<account-address>"]}}'`
- **AND** the instance source module contains a compatible embedded schema for `counter.balanceOf`
- **THEN** the CLI invokes the entrypoint with parameter bytes serialized from the JSON value

#### Scenario: Invoke uses optional parameter JSON file
- **WHEN** the user runs `ccd-wallet contract invoke --contract 42,0 --receive counter.balanceOf --parameter-json-file ./invoke-params.json`
- **AND** `./invoke-params.json` contains valid JSON
- **AND** the instance source module contains a compatible embedded schema for `counter.balanceOf`
- **THEN** the CLI invokes the entrypoint with parameter bytes serialized from the JSON file contents

#### Scenario: Invoke parameter JSON requires embedded schema
- **WHEN** the user runs `ccd-wallet contract invoke --contract 42,0 --receive counter.balanceOf --parameter-json '{"owner": {"Account": ["<account-address>"]}}'`
- **AND** the instance source module has no compatible embedded schema for `counter.balanceOf`
- **THEN** the CLI exits with an actionable error
- **AND** does not invoke the entrypoint

#### Scenario: Invoke rejects conflicting parameter inputs
- **WHEN** the user runs `ccd-wallet contract invoke --contract 42,0 --receive counter.balanceOf --parameter-hex 000102 --parameter-json '{"owner": {"Account": ["<account-address>"]}}'`
- **THEN** the CLI exits with an actionable error
- **AND** does not invoke the entrypoint

#### Scenario: Invoke rejects multiple JSON parameter sources
- **WHEN** the user runs `ccd-wallet contract invoke --contract 42,0 --receive counter.balanceOf --parameter-json '{"owner": {"Account": ["<account-address>"]}}' --parameter-json-file ./invoke-params.json`
- **THEN** the CLI exits with an actionable error
- **AND** does not invoke the entrypoint

#### Scenario: Invoke uses optional energy
- **WHEN** the user runs `ccd-wallet contract invoke --contract 42,0 --receive counter.view --energy 50000`
- **THEN** the CLI invokes the entrypoint with the specified energy limit

### Requirement: CLI invoke defaults to synthetic zero-account context
When `contract invoke` is run without `--invoker`, the CLI SHALL omit the invoker from the contract context so the node uses its synthetic zero-account invocation context. The CLI SHALL NOT require a wallet account or account address for this default path.

If `--invoker` is supplied, the CLI SHALL use that account address as the invocation sender context.

#### Scenario: Invoke omits invoker by default
- **WHEN** the user runs `ccd-wallet contract invoke --contract 42,0 --receive counter.view`
- **THEN** the CLI constructs the invocation without an explicit invoker
- **AND** does not require account selection or account unlocking

#### Scenario: Invoke accepts explicit invoker
- **WHEN** the user runs `ccd-wallet contract invoke --contract 42,0 --receive counter.view --invoker <account-address>`
- **THEN** the CLI constructs the invocation with the supplied account address as invoker

