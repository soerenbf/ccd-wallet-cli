## 1. Workspace setup

- [x] 1.1 Add pnpm workspace files at the repository root for JavaScript/TypeScript packages.
- [x] 1.2 Create the initial `packages/` layout and the first connect client package skeleton.
- [x] 1.3 Add package-level TypeScript build, lint, and test configuration for the first client package.

## 2. Client library implementation

- [x] 2.1 Define the package's public TypeScript models for pairing, session context, and JSON-RPC interaction.
- [x] 2.2 Implement WebSocket connection lifecycle management in the client library.
- [x] 2.3 Implement JSON-RPC 2.0 request/response handling for the connect protocol.
- [x] 2.4 Implement the pairing API using an application-provided challenge.
- [x] 2.5 Implement session-context retrieval using the session token.

## 3. Compatibility and validation

- [x] 3.1 Ensure the core client API remains web-compatible and avoids Node-specific runtime assumptions.
- [x] 3.2 Add tests covering connection lifecycle, pairing, and session-context retrieval.
- [x] 3.3 Add developer documentation for installing and using the TypeScript connect client library.
