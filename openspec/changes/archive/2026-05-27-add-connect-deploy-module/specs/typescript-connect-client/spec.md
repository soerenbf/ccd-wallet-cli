## ADDED Requirements

### Requirement: TypeScript connect client supports deploy-module requests
The TypeScript connect client SHALL expose a `requestDeployModule` method that sends a `requestDeployModule` JSON-RPC request to the connect server and resolves with the returned transaction hash.

The method SHALL accept the following parameters corresponding to the server method:
- `sessionToken`
- `moduleHex`
- `validate` (optional)

#### Scenario: Client sends requestDeployModule request
- **WHEN** an application calls `requestDeployModule` with valid parameters
- **THEN** the client sends a JSON-RPC 2.0 `requestDeployModule` request to the connect server
- **AND** includes all required fields in the request parameters
- **AND** resolves with the transaction hash returned by the server

#### Scenario: Client rejects on server error for deploy-module request
- **WHEN** the connect server returns a JSON-RPC error for a `requestDeployModule` call
- **THEN** the client rejects the returned promise with a `ConnectClientError`
- **AND** the error carries the server-provided code and message
