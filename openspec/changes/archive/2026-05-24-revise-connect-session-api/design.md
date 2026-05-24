## Context

The current connect API is account-centric: pairing establishes trust and immediately returns approved network and account context. That works for the first account-only flow, but it does not scale cleanly to governance use cases where the relevant authority is a governance vault scoped only by network.

A better shape is to separate:
- session establishment
- network-specific authority requests
- future proposal methods

In this model, pairing only proves trust and returns a session token. Applications then explicitly request the authority they need for a specific network. Account-oriented applications request an account address on a network, while future governance-oriented applications can request chain-update actions against a network without pretending to have an account context.

## Goals / Non-Goals

**Goals:**
- Make pairing about session establishment only.
- Let applications explicitly request account authority for a target network after pairing.
- Remove the assumption that every paired session is inherently account-shaped.
- Create a protocol/client shape that can later support network-scoped governance actions cleanly.
- Keep the revised API small and easy to understand.

**Non-Goals:**
- Defining the full future chain-update or transaction proposal payloads in this change.
- Adding governance request methods yet.
- Introducing hidden mutable session state such as an implicit active account selected after one request.
- Redesigning the transport, challenge flow, or approval ceremony.

## Decisions

### 1. Pairing returns only a session token
The `pair` method will establish a trusted session and return only the session token. It will no longer return network or account data directly.

**Rationale:** pairing is fundamentally about trust establishment. Returning authority context in the pairing response overloads the method and bakes in assumptions that do not hold for governance-oriented uses.

### 2. Account access becomes an explicit network-scoped request
A follow-up method such as `requestAccount(sessionToken, networkGenesisHash)` will let an application ask the wallet for account authority on a specific network. The method returns only the selected account address.

**Rationale:** this keeps network intent application-driven and avoids embedding account context into pairing itself. It also matches the likely future governance pattern, where the application also knows which network it wants.

### 3. Keep the session API explicit rather than implicitly mutating account state
The returned account address is passed explicitly into future account transaction methods instead of silently becoming an implicit session default.

**Rationale:** explicit authority references are easier to reason about and reduce hidden state in the connect session.

### 4. Future proposal methods should build on the same shape
The long-term shape this change enables is:
- `pair -> sessionToken`
- `requestAccount(sessionToken, networkGenesisHash) -> accountAddress`
- future `proposeTransaction(sessionToken, networkGenesisHash, accountAddress, proposal)`
- future `proposeChainUpdate(sessionToken, networkGenesisHash, proposal)`

This change does not add those later methods, but it intentionally sets the API direction.

**Rationale:** the session-first shape works for both account and governance flows without forcing them into one context object.

## Risks / Trade-offs

- **[Separating pairing from authority requests adds an extra round trip]** → Mitigation: accept the extra call in exchange for a cleaner and more extensible protocol.
- **[Consumers must now understand that network selection is application-driven]** → Mitigation: make the client API and example app flow explicit and well-documented.
- **[Changing pairing response shape is a breaking API change]** → Mitigation: capture it clearly in the specs and update the example/client together.

## Migration Plan

1. Change the connect server so `pair` returns only a session token.
2. Add a new account-request method keyed by session token and network genesis hash.
3. Update the TypeScript client to expose the revised flow.
4. Update the example app to pair first, then request an account for the selected network.
5. Update docs to describe the new session-first model.

## Open Questions

None currently.
