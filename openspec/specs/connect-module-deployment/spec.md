# connect-module-deployment Specification

## Purpose
Define wallet-approved smart contract module deployment over the browser connect protocol, including session-bound execution context, optional validation, wallet review behavior, and local finalization reporting.

## Requirements
### Requirement: Connect server supports smart contract module deployment transactions
The connect server SHALL expose a `requestDeployModule` JSON-RPC method that allows a paired browser dApp to propose a smart contract module deployment transaction. The wallet SHALL sign and submit the deployment only if the user approves.

The method SHALL accept:
- `sessionToken`: the active session token
- `moduleHex`: the serialized smart contract module bytes as a hex string
- `validate`: an optional boolean (default `false`)

On success the method SHALL return `{ "transactionHash": "<hash>" }`.

#### Scenario: Browser proposes a module deployment and wallet approves
- **WHEN** a paired browser sends a valid `requestDeployModule` request
- **AND** the user approves in the wallet terminal
- **THEN** the wallet signs and submits the deploy-module transaction
- **AND** returns the transaction hash to the browser

#### Scenario: Browser proposes a module deployment and wallet declines
- **WHEN** a paired browser sends a valid `requestDeployModule` request
- **AND** the user declines in the wallet approval prompt
- **THEN** the wallet does not submit any transaction
- **AND** the method returns a JSON-RPC error with code -32004

#### Scenario: Request with invalid or missing session token is rejected
- **WHEN** a browser sends a `requestDeployModule` request with an invalid or absent session token
- **THEN** the method returns a JSON-RPC error and no transaction is submitted

### Requirement: Deploy module uses the session-bound network and account
The network and account used for signing and submitting a deploy-module request SHALL be those bound to the active session. The browser SHALL NOT be able to redirect deployment to a different network or account.

#### Scenario: Deploy request uses the active session context
- **WHEN** a session is paired for account `alice` on network `testnet`
- **AND** the browser sends a `requestDeployModule` request
- **THEN** the wallet signs and submits the deployment using `alice` on `testnet`
- **AND** the browser cannot override that context through request parameters

### Requirement: Deploy validation checks whether the module already exists on chain
When `validate: true` is set on a deploy request, the wallet SHALL derive the module reference from `moduleHex` and check whether that module already exists on chain before prompting for approval.

If the module is confirmed to already exist on chain, the wallet SHALL show that result as a warning in the approval prompt, using the message `Validation warning: module already exists on chain for this network.`, and SHALL still allow the user to approve or decline. If the validation check cannot be completed because of node unavailability or another transient failure, the wallet SHALL show that result as a warning in the approval prompt and SHALL still allow the user to approve or decline.

#### Scenario: Validation succeeds and module is not already deployed
- **WHEN** a browser sends `requestDeployModule` with `validate: true`
- **AND** the wallet confirms that the derived module reference is not already present on chain
- **THEN** the wallet proceeds to the approval prompt

#### Scenario: Validation finds an existing module on chain
- **WHEN** a browser sends `requestDeployModule` with `validate: true`
- **AND** the wallet confirms that the derived module reference already exists on chain
- **THEN** the wallet shows that finding as a warning in the approval prompt
- **AND** uses the warning message `Validation warning: module already exists on chain for this network.`
- **AND** still lets the user choose whether to proceed

#### Scenario: Validation cannot complete due to node failure
- **WHEN** a browser sends `requestDeployModule` with `validate: true`
- **AND** the wallet cannot complete the existence check because the node is unavailable
- **THEN** the wallet shows the validation failure as a warning in the approval prompt
- **AND** still lets the user choose whether to proceed

### Requirement: Wallet approval prompt shows deploy-specific review details
The wallet approval prompt for `requestDeployModule` SHALL display:
- the browser origin
- the session-bound network
- the session-bound account address
- the derived module reference
- the module size in bytes
- any validation warning or duplicate-module finding

The approval prompt SHALL NOT display a second hash summary beyond the derived module reference.

#### Scenario: Approval prompt shows derived module details
- **WHEN** the wallet presents a deploy-module approval prompt
- **THEN** the prompt includes the derived module reference
- **AND** includes the module size in bytes

### Requirement: Wallet displays deploy-module finalization outcome locally after submission
After submitting a deploy-module transaction, the wallet SHALL wait for finalization in the background and print a readable outcome summary to the terminal. The JSON-RPC response SHALL NOT block on finalization.

#### Scenario: Wallet prints deploy-module finalization outcome after submission
- **WHEN** a deploy-module transaction has been submitted
- **AND** the transaction is finalized on chain
- **THEN** the wallet prints a readable finalization summary to the terminal
- **AND** includes the finalized block and deployed module reference when available
- **AND** the JSON-RPC response was already returned to the browser with the transaction hash
