## Why

The repository now has a browser-pairing connect server, but no web-compatible client library for applications to consume it safely and ergonomically. A TypeScript client is the next natural step because browser applications need a small, typed, environment-flexible integration layer for pairing, session context retrieval, and future connect-protocol growth.

This is also the first non-Rust module in the repository. Setting up a JavaScript/TypeScript workspace now creates a clean foundation for future packages, including a possible browser-extension adapter for existing Concordium wallet APIs.

## What Changes

- Add a pnpm-managed JavaScript/TypeScript workspace to the repository for future non-Rust packages.
- Add a first package that combines all client-side connect functionality in one library.
- Implement a web-compatible TypeScript client for the existing connect server protocol over WebSocket using JSON-RPC 2.0.
- Support the current connected-wallet flow only:
  - open/close the WebSocket connection
  - request pairing with an application-provided challenge
  - retrieve approved session context
- Make the client environment-flexible by depending on standards-oriented or environment-agnostic APIs and libraries rather than Node-specific runtime assumptions.
- Keep extension-style adapter packages out of scope for this change; the client library is the foundation for those future integrations.

## Capabilities

### New Capabilities
- `typescript-connect-client`: A web-compatible TypeScript client library for pairing with the wallet connect server and retrieving approved session context.

### Modified Capabilities

## Impact

- Affected code: repository root workspace/tooling files, a new `packages/` workspace, and the first TypeScript client package.
- Affected systems: package-management workflow, TypeScript build/test tooling, browser-facing connect integration, and developer documentation for using the client library.
- Dependencies: pnpm workspace tooling and environment-flexible TypeScript/browser libraries for WebSocket and JSON-RPC client behavior.
