## MODIFIED Requirements

### Requirement: TypeScript connect client returns approved session information from pairing
When pairing succeeds, the TypeScript connect client SHALL expose only the resulting session token returned by the connect server. The pairing API SHALL NOT require the client to treat network or account context as part of the pairing response.

#### Scenario: Successful pairing returns session token only
- **WHEN** the connect server approves a pairing request
- **THEN** the client resolves the pairing call with the returned session token
- **AND** does not require approved network or account context to be present in the pairing result

### Requirement: TypeScript connect client can retrieve approved session context
The TypeScript connect client SHALL allow an application to request account authority for a target network by calling a dedicated client API with the session token and network genesis hash. When the server approves that request, the client SHALL resolve with the returned account address.

#### Scenario: Client requests an account address for a network
- **WHEN** an application calls the client account-request API with an active session token and network genesis hash
- **THEN** the client sends the appropriate JSON-RPC 2.0 request to the connect server
- **AND** resolves with the approved account address returned by the server
