## MODIFIED Requirements

### Requirement: TypeScript connect client can retrieve approved session context
The TypeScript connect client SHALL allow an application to request account authority for the network bound to an active paired session by calling a dedicated client API with the session token and network genesis hash. When the server approves that request, the client SHALL resolve with the returned account address.

#### Scenario: Client requests an account address for the paired session network
- **WHEN** an application calls the client account-request API with an active session token and the paired session's network genesis hash
- **THEN** the client sends the appropriate JSON-RPC 2.0 request to the connect server
- **AND** resolves with the approved account address returned by the server

## ADDED Requirements

### Requirement: TypeScript connect client supports paired sessions before account authority exists
The TypeScript connect client SHALL support successful pairing flows that return a session token even when no account authority has yet been granted for that session.

#### Scenario: Pairing succeeds before an account is requested
- **WHEN** an application calls the client pairing API with a challenge value
- **AND** the wallet approves pairing without selecting an account
- **THEN** the client resolves the pairing call with the returned session token
- **AND** the application can later call the client account-request API when account authority is needed
