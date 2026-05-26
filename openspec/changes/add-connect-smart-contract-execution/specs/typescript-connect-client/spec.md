## ADDED Requirements

### Requirement: TypeScript connect client supports contract init requests
The TypeScript connect client SHALL expose a `requestContractInit` method that sends a `requestContractInit` JSON-RPC request to the connect server and resolves with the returned transaction hash.

The method SHALL accept the following parameters corresponding to the server method:
- `sessionToken`
- `moduleRef`
- `initName`
- `amountMicroCcd`
- `maxContractExecutionEnergy`
- `parameterHex`
- `schema` (optional)

#### Scenario: Client sends requestContractInit request
- **WHEN** an application calls `requestContractInit` with valid parameters
- **THEN** the client sends a JSON-RPC 2.0 `requestContractInit` request to the connect server
- **AND** includes all required fields in the request parameters
- **AND** resolves with the transaction hash returned by the server

#### Scenario: Client rejects on server error for contract init
- **WHEN** the connect server returns a JSON-RPC error for a `requestContractInit` call
- **THEN** the client rejects the returned promise with a `ConnectClientError`
- **AND** the error carries the server-provided code and message

### Requirement: TypeScript connect client supports contract update requests
The TypeScript connect client SHALL expose a `requestContractUpdate` method that sends a `requestContractUpdate` JSON-RPC request to the connect server and resolves with the returned transaction hash.

The method SHALL accept the following parameters corresponding to the server method:
- `sessionToken`
- `contractAddress` (with `index` and `subindex` fields)
- `receiveName`
- `amountMicroCcd`
- `maxContractExecutionEnergy`
- `parameterHex`
- `schema` (optional)

#### Scenario: Client sends requestContractUpdate request
- **WHEN** an application calls `requestContractUpdate` with valid parameters
- **THEN** the client sends a JSON-RPC 2.0 `requestContractUpdate` request to the connect server
- **AND** includes all required fields in the request parameters
- **AND** resolves with the transaction hash returned by the server

#### Scenario: Client rejects on server error for contract update
- **WHEN** the connect server returns a JSON-RPC error for a `requestContractUpdate` call
- **THEN** the client rejects the returned promise with a `ConnectClientError`
- **AND** the error carries the server-provided code and message
