## Context

The repository is currently Rust-first and uses a Cargo workspace, but it now has a browser-facing connect protocol that needs a client library on the application side. Browser applications should not have to hand-roll WebSocket handling, JSON-RPC 2.0 message formatting, challenge submission, session-token handling, and session-context retrieval for every integration.

This change is also the first introduction of JavaScript/TypeScript code into the repository. That makes the packaging and workspace decision part of the design, not just an implementation detail. The chosen setup should support one package now while leaving a clean path for future non-Rust modules such as a browser-extension adapter for existing Concordium wallet APIs.

The current server-side connect flow is intentionally narrow:
- open a localhost WebSocket connection
- send a pairing request with a challenge
- receive a session token and approved context
- query the approved session context

The first TypeScript package should mirror that narrow scope and avoid prematurely splitting transport, protocol, and browser session helpers across multiple packages.

## Goals / Non-Goals

**Goals:**
- Add a pnpm-managed TypeScript workspace to the repository.
- Add a first TypeScript package that combines all current connect-client functionality.
- Provide a web-compatible client API for pairing and session-context retrieval over WebSocket.
- Use JSON-RPC 2.0 consistently so the TypeScript client matches the Rust connect server contract.
- Keep the package environment-flexible by preferring standards-oriented, runtime-agnostic APIs and libraries.
- Establish a package layout that future browser-facing packages can reuse.

**Non-Goals:**
- Building the browser-extension adapter in this change.
- Adding transaction proposal, signing, or governance-specific client methods.
- Splitting the first client into multiple packages such as `transport`, `protocol-types`, or `browser-helpers`.
- Making the TypeScript workspace the primary build entry point for the whole repository.
- Replacing or restructuring the Rust Cargo workspace.

## Decisions

### 1. Use pnpm as the JavaScript/TypeScript workspace manager
The repository will gain a pnpm workspace alongside the existing Cargo workspace.

Expected root-level additions include:
- `package.json`
- `pnpm-workspace.yaml`
- `packages/`

**Rationale:** pnpm is a strong default for multi-package repositories, has good workspace ergonomics, and fits the expectation that this repository will eventually contain more than one non-Rust package.

**Alternatives considered:**
- **npm workspaces**: simpler baseline, but less attractive for a repository that already expects multiple packages.
- **Yarn**: viable, but not the preferred greenfield default here.

### 2. Start with a single combined client package
The first package will combine transport handling, JSON-RPC request/response typing, pairing flow, session-context retrieval, and browser-oriented convenience APIs in one library.

**Rationale:** the current connect API surface is intentionally small, and those concerns are tightly coupled. Splitting them now would introduce package boundaries before the protocol surface is large enough to justify them.

**Alternatives considered:**
- **Separate transport and protocol-types packages**: rejected as premature for the current scope.
- **Extension-first packaging**: rejected because the extension/adapter should depend on the canonical client library, not define it.

### 3. The package should be environment-flexible, not Node-specific
The client should depend on standards-oriented or environment-agnostic APIs and libraries wherever possible. In practice that means avoiding a design that assumes Node-only globals, Node-only networking primitives, or browser-extension-specific APIs in the core package.

The intended target is browser compatibility first, while remaining portable to other JavaScript runtimes that can provide equivalent WebSocket and standard-language support.

**Rationale:** the package is meant to be reused by web applications and later by adapter layers. A Node-shaped client would create avoidable portability friction.

### 4. Mirror the current server protocol rather than inventing a higher-level abstraction first
The first client package should model the current connect server contract directly:
- connect/disconnect
- send `pair`
- receive session token and approved context
- send `session.getContext`

The API can still be ergonomic, but it should not obscure the core protocol semantics or invent session behaviors the server does not have.

**Rationale:** keeping the first client close to the server contract reduces ambiguity, makes debugging easier, and avoids locking in a premature abstraction layer before more server capabilities exist.

### 5. Future adapter packages should build on this library
The later browser-extension compatibility layer should depend on the TypeScript connect client rather than reimplementing transport or protocol logic.

**Rationale:** this keeps the client library as the canonical integration surface and reduces duplication when adapter-oriented packages are added later.

## Risks / Trade-offs

- **[Adding a second workspace system increases repository complexity]** → Mitigation: keep the JS/TS workspace clearly scoped to `packages/` and avoid entangling it with Cargo workflows.
- **[A single combined client package may eventually need to be split]** → Mitigation: accept this for now; split later only when a real second consumer or package boundary emerges.
- **[Environment-flexible design may limit convenience helpers that rely on one runtime]** → Mitigation: keep the core package runtime-neutral and add runtime-specific adapters only when needed.
- **[Protocol mirroring can feel lower-level than some application developers want]** → Mitigation: provide a small ergonomic client API while keeping the underlying request model recognizable.

## Migration Plan

1. Add pnpm workspace files at the repository root.
2. Add the first package under `packages/`.
3. Configure TypeScript build/test/lint tooling for that package.
4. Implement connect-client WebSocket and JSON-RPC 2.0 support.
5. Implement pairing and session-context retrieval methods.
6. Add usage documentation for browser applications.

## Open Questions

None currently.
