# typescript-connect-client Specification

## Purpose
Define the repository's TypeScript connect client package, including pnpm workspace support, the combined client package boundary, pairing and session-context retrieval behavior, and environment-flexible browser-oriented API expectations.
## Requirements
### Requirement: Repository provides a pnpm-managed TypeScript workspace for browser-facing packages
The repository SHALL provide a pnpm-managed JavaScript/TypeScript workspace for non-Rust packages. The workspace SHALL support the first connect client package while leaving a path for future additional packages.

#### Scenario: Repository contains workspace metadata for TypeScript packages
- **WHEN** a contributor inspects the repository root
- **THEN** the repository contains pnpm workspace metadata for JavaScript/TypeScript packages
- **AND** the workspace includes the package location used for the connect client library

### Requirement: Repository provides a single combined TypeScript connect client package
The repository SHALL provide one initial TypeScript package that combines the current connect-client responsibilities in a single library. That package SHALL cover WebSocket connection handling, JSON-RPC 2.0 request/response handling, pairing, and session-context retrieval.

#### Scenario: Connect client package covers current protocol responsibilities
- **WHEN** a contributor inspects the first TypeScript connect package
- **THEN** the package includes client support for WebSocket transport
- **AND** includes JSON-RPC 2.0 request/response handling
- **AND** includes pairing and session-context retrieval behavior

### Requirement: TypeScript connect client supports pairing with an application-provided challenge
The TypeScript connect client SHALL allow a web application to initiate pairing with the wallet connect server by submitting an application-provided challenge over the WebSocket JSON-RPC 2.0 channel.

#### Scenario: Client sends pairing request with challenge
- **WHEN** an application calls the client pairing API with a challenge value
- **THEN** the client sends a JSON-RPC 2.0 `pair` request over the WebSocket channel
- **AND** includes the supplied challenge in the request parameters

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

### Requirement: TypeScript connect client is web-compatible and environment-flexible
The TypeScript connect client SHALL be usable from browser-oriented applications and SHALL avoid requiring Node-specific runtime assumptions in its core API design.

#### Scenario: Core client API does not require Node-only primitives
- **WHEN** an application integrates the core TypeScript connect client
- **THEN** the public client API does not require Node-only globals or Node-only transport primitives
- **AND** remains suitable for browser-oriented use

