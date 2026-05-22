## Context

The repository now has two building blocks for browser-side integration:
- a connect server in the wallet CLI
- a TypeScript client library that wraps the current connect protocol

What is still missing is an executable example that shows how a real browser app should use those pieces together. A small example app is useful for three different reasons at once:
- it acts as an integration reference for developers
- it provides end-to-end validation of the current client API
- it gives future changes a concrete place to test browser-side UX assumptions

The goal is not to build a production wallet frontend. The example should stay narrowly focused on the currently supported workflow: connect to the local server, pair with a visible challenge, and display approved session context.

## Goals / Non-Goals

**Goals:**
- Add an example web application package under the pnpm workspace.
- Use Vite with React and TypeScript so the example looks like a realistic browser app while keeping the connect integration easy to read.
- Make the example depend on `@ccd-wallet/connect-client` and use the public client API directly.
- Demonstrate the current connect flow clearly: pairing plus session-context retrieval.
- Keep the example easy to read and copy from.

**Non-Goals:**
- Building a polished production-style UI.
- Introducing additional app frameworks or state-management layers beyond React.
- Demonstrating transaction proposal, signing, or governance flows.
- Replacing package-level docs in the client library; the example complements them.
- Designing the future browser-extension adapter in this change.

## Decisions

### 1. Use Vite with React and TypeScript
The example app will be a small Vite package using React and TypeScript.

**Rationale:** for an integration reference, React reduces the amount of manual DOM wiring that distracts from the actual wallet-client integration. It is familiar to many frontend developers and makes the flow easier to translate into real applications.

**Alternatives considered:**
- **Plain static HTML + TypeScript without Vite**: smaller, but manual DOM/state wiring takes too much attention away from the connect client usage.
- **Vite + vanilla TypeScript**: workable, but less effective as an integration reference because the example starts teaching imperative browser wiring.
- **Other frameworks such as Solid or Lit**: viable, but less broadly familiar for a general-purpose reference app.

### 2. Treat the example as an integration reference, not a product demo
The example should prioritize clarity over polish. It can be developer-oriented and somewhat plain as long as the flow is explicit and easy to inspect.

**Rationale:** the main audience is developers integrating with the wallet, not end users evaluating a finished product.

### 3. Keep the example scope aligned with the currently supported client/server flow
The example app will demonstrate:
- entering or using a default connect-server URL
- generating or regenerating a six-digit challenge
- pairing through the client library
- displaying session token, network genesis hash, and account address
- refreshing session context and resetting local state

It will not speculate about future proposal APIs.

**Rationale:** this keeps the example stable and ensures it remains a faithful reference for the currently shipped protocol surface.

### 4. Depend on the client library instead of reimplementing protocol logic
The example app should consume `@ccd-wallet/connect-client` as a normal package dependency from the workspace and avoid hand-rolling WebSocket or JSON-RPC behavior.

**Rationale:** the example should reinforce the intended consumer experience and validate the client package API directly.

## Risks / Trade-offs

- **[Even a simple example app adds another package to maintain]** → Mitigation: keep the app intentionally small and narrowly scoped.
- **[Vite introduces extra tooling compared with a minimal static example]** → Mitigation: accept the small tooling cost in exchange for a more realistic reference application shape.
- **[Example code can drift from the client API over time]** → Mitigation: keep the example in the same workspace and validate it as part of normal package changes.
- **[React adds framework concepts to a deliberately small example]** → Mitigation: keep the component structure minimal and use React only to reduce incidental UI wiring, not to introduce broader app architecture.
- **[A developer-facing reference app may still look too plain]** → Mitigation: optimize for clarity first and only add polish when it improves understanding of the integration.

## Migration Plan

1. Add a new example app package under `packages/`.
2. Configure Vite and TypeScript for the example package.
3. Wire the example package to depend on `@ccd-wallet/connect-client` from the workspace.
4. Implement the pairing and session-context UI flow.
5. Add documentation describing the example app's purpose and how to run it.

## Open Questions

None currently.
