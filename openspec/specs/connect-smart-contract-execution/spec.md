# connect-smart-contract-execution Specification

## Purpose
TBD - created by archiving change add-connect-smart-contract-execution. Update Purpose after archive.
## Requirements
### Requirement: Connect server supports smart contract init transactions
The connect server SHALL expose a `requestContractInit` JSON-RPC method that allows a paired browser dApp to propose a smart contract initialization transaction. The wallet SHALL simulate the transaction before prompting for approval, and SHALL sign and submit only if the user approves.

The method SHALL accept:
- `sessionToken`: the active session token
- `moduleRef`: the hex module reference to initialize from
- `initName`: the init function name (e.g. `init_my_contract`)
- `amountMicroCcd`: the CCD amount to attach, as a decimal string
- `maxContractExecutionEnergy`: the caller-supplied energy ceiling as a number
- `parameterHex`: the serialized parameter bytes as a hex string
- `schema`: an optional schema object used for human-readable parameter display
- `validate`: an optional boolean (default `false`); when `true` the wallet runs a simulation before the approval prompt

On success the method SHALL return `{ "transactionHash": "<hash>" }`.

#### Scenario: Browser proposes a contract init and wallet approves
- **WHEN** a paired browser sends a valid `requestContractInit` request
- **AND** the user approves in the wallet terminal
- **THEN** the wallet signs and submits the init transaction
- **AND** returns the transaction hash to the browser

#### Scenario: Browser proposes a contract init with validate true and simulation succeeds
- **WHEN** a paired browser sends a `requestContractInit` request with `validate: true`
- **AND** the wallet simulation succeeds
- **THEN** the approval prompt includes the simulated energy used
- **AND** the user can approve or decline normally

#### Scenario: Browser proposes a contract init with validate true and simulation fails
- **WHEN** a paired browser sends a `requestContractInit` request with `validate: true`
- **AND** the wallet simulation fails or the node is unreachable
- **THEN** the wallet shows the simulation failure as a warning in the approval prompt
- **AND** still asks the user y/N whether to proceed
- **AND** does not return an error solely because simulation failed

#### Scenario: Browser proposes a contract init without validate and no simulation is run
- **WHEN** a paired browser sends a `requestContractInit` request without `validate` or with `validate: false`
- **THEN** the wallet does not perform any simulation
- **AND** the approval prompt shows the request details without simulation output

#### Scenario: Browser proposes a contract init and wallet declines
- **WHEN** a paired browser sends a valid `requestContractInit` request
- **AND** the user declines in the wallet approval prompt
- **THEN** the wallet does not submit any transaction
- **AND** the method returns a JSON-RPC error with code -32004

#### Scenario: Request with invalid or missing session token is rejected
- **WHEN** a browser sends a `requestContractInit` request with an invalid or absent session token
- **THEN** the method returns a JSON-RPC error and no transaction is submitted

### Requirement: Connect server supports smart contract update transactions
The connect server SHALL expose a `requestContractUpdate` JSON-RPC method that allows a paired browser dApp to propose a smart contract update transaction (receive function invocation). The wallet SHALL simulate the transaction before prompting for approval, and SHALL sign and submit only if the user approves.

The method SHALL accept:
- `sessionToken`: the active session token
- `contractAddress`: the contract instance address as `{ "index": <number>, "subindex": <number> }`
- `receiveName`: the fully-qualified receive name (e.g. `my_contract.transfer`)
- `amountMicroCcd`: the CCD amount to attach, as a decimal string
- `maxContractExecutionEnergy`: the caller-supplied energy ceiling as a number
- `parameterHex`: the serialized parameter bytes as a hex string
- `schema`: an optional schema object used for human-readable parameter display
- `validate`: an optional boolean (default `false`); when `true` the wallet runs a simulation before the approval prompt

On success the method SHALL return `{ "transactionHash": "<hash>" }`.

#### Scenario: Browser proposes a contract update and wallet approves
- **WHEN** a paired browser sends a valid `requestContractUpdate` request
- **AND** the user approves in the wallet terminal
- **THEN** the wallet signs and submits the update transaction
- **AND** returns the transaction hash to the browser

#### Scenario: Browser proposes a contract update with validate true and simulation succeeds
- **WHEN** a paired browser sends a `requestContractUpdate` request with `validate: true`
- **AND** the wallet simulation succeeds
- **THEN** the approval prompt includes the simulated energy used
- **AND** the user can approve or decline normally

#### Scenario: Browser proposes a contract update with validate true and simulation fails
- **WHEN** a paired browser sends a `requestContractUpdate` request with `validate: true`
- **AND** the wallet simulation fails or the node is unreachable
- **THEN** the wallet shows the simulation failure as a warning in the approval prompt
- **AND** still asks the user y/N whether to proceed
- **AND** does not return an error solely because simulation failed

#### Scenario: Browser proposes a contract update without validate and no simulation is run
- **WHEN** a paired browser sends a `requestContractUpdate` request without `validate` or with `validate: false`
- **THEN** the wallet does not perform any simulation
- **AND** the approval prompt shows the request details without simulation output

#### Scenario: Browser proposes a contract update and wallet declines
- **WHEN** a paired browser sends a valid `requestContractUpdate` request
- **AND** the user declines in the wallet approval prompt
- **THEN** the wallet does not submit any transaction
- **AND** the method returns a JSON-RPC error with code -32004

#### Scenario: Request with invalid or missing session token is rejected
- **WHEN** a browser sends a `requestContractUpdate` request with an invalid or absent session token
- **THEN** the method returns a JSON-RPC error and no transaction is submitted

### Requirement: Wallet approval prompt shows simulation details when validation is requested
When `validate: true` is set and simulation succeeds, the wallet SHALL include in the approval prompt:
- the simulated energy used
- the simulation outcome

When `validate: true` and simulation fails, the wallet SHALL display the failure as a warning and still present a y/N prompt. When `validate` is absent or `false`, no simulation output is shown.

In all cases the approval prompt SHALL also display:
- the browser origin
- the session-bound network (by alias where registered, otherwise genesis hash)
- the session-bound account address
- for updates: the contract address and receive name
- for init: the module reference and init function name
- the amount in CCD
- the caller-supplied max energy
- the decoded parameter when a schema is provided; the raw hex otherwise

#### Scenario: Approval prompt includes simulation energy when validate true and simulation succeeds
- **WHEN** a contract execution request includes `validate: true`
- **AND** the wallet simulation succeeds
- **AND** the wallet presents the approval prompt
- **THEN** the prompt includes both the caller-supplied max energy and the simulated energy used

#### Scenario: Approval prompt shows simulation warning when validate true and simulation fails
- **WHEN** a contract execution request includes `validate: true`
- **AND** the wallet simulation fails
- **AND** the wallet presents the approval prompt
- **THEN** the prompt shows the simulation failure as a warning
- **AND** still presents a y/N option to proceed

#### Scenario: Approval prompt shows decoded parameter when schema present
- **WHEN** a `requestContractUpdate` or `requestContractInit` request includes a schema
- **AND** the wallet presents the approval prompt
- **THEN** the prompt displays a human-readable representation of the parameter derived from the schema
- **AND** does not display only raw hex bytes

#### Scenario: Approval prompt shows hex parameter when schema absent
- **WHEN** a `requestContractUpdate` or `requestContractInit` request does not include a schema
- **AND** the wallet presents the approval prompt
- **THEN** the prompt displays the parameter as a hex string

### Requirement: Wallet displays finalization outcome locally after submission
After submitting a contract execution transaction, the wallet SHALL wait for finalization in the background and print the outcome to the terminal. The JSON-RPC response SHALL NOT block on finalization.

#### Scenario: Wallet prints finalization outcome after contract transaction
- **WHEN** a contract execution transaction has been submitted
- **AND** the transaction is finalized on chain
- **THEN** the wallet prints the finalization outcome (success or reject reason) to the terminal
- **AND** the JSON-RPC response was already returned to the browser with the transaction hash

### Requirement: Contract execution uses session-bound network and account
The network and account used for signing and submitting a contract execution request SHALL be those bound to the active session at pairing time. The browser SHALL NOT be able to supply a different account address or genesis hash for execution requests; any such fields in the request params SHALL be ignored.

#### Scenario: Contract execution uses session-bound account
- **WHEN** a session was paired with account `alice` on network `testnet`
- **AND** the browser sends a `requestContractUpdate` request
- **THEN** the transaction is signed and submitted using `alice`'s keys on `testnet`
- **AND** the browser cannot redirect execution to a different account

#### Scenario: Contract execution fails if no session is active
- **WHEN** a browser sends a `requestContractUpdate` or `requestContractInit` request
- **AND** no active paired session exists
- **THEN** the method returns a JSON-RPC error with code -32002

### Requirement: Contract execution node endpoint is resolved from session network
The wallet SHALL resolve the Concordium node endpoint for contract simulation and submission from the registered network whose genesis hash matches the session-bound network genesis hash. If no matching registered network is found, the request SHALL be rejected with an actionable error.

#### Scenario: Node endpoint resolved from registered network
- **WHEN** a session is bound to a genesis hash that matches a registered network
- **AND** the browser sends a contract execution request
- **THEN** the wallet uses the registered network's node endpoint for simulation and submission

#### Scenario: Contract execution rejected when no matching network is registered
- **WHEN** a session is bound to a genesis hash
- **AND** no registered network has that genesis hash
- **THEN** the contract execution request is rejected with an actionable error
- **AND** the user is told to register the network using `ccd-wallet network add`

