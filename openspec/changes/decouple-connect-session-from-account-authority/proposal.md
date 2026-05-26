## Why

The current connect flow conflates browser-session trust with account authority by selecting and binding an account during pairing, even though not every present or future API area requires an account. As the example application grows into a sectioned API showcase for smart contracts, transactions, and chain updates, the protocol needs a session model that supports pairing first and acquiring account authority only when a feature actually needs it.

## What Changes

- Change pairing semantics so a successful `pair` approval establishes a trusted browser session and binds the approved network, but does not require selecting or storing an account.
- Change `requestAccount` from a read-back of pairing-bound context into an explicit account-authority acquisition step for an already paired session.
- Update account-requiring connect flows, including smart contract init/update requests, to require previously granted account authority in the active session instead of relying on account selection during pairing.
- Restructure the example web app into a session-gated API showcase shell with navigation between feature areas, with Smart Contracts implemented first and Transactions / Chain Updates prepared as future sections.
- Use `@concordium/web-sdk` in the example app's Smart Contracts area to derive embedded contract schemas from the chain or referenced module and prepare typed contract parameters while still sending requests through `@ccd-wallet/connect-client`.
- Update documentation to explain the new separation between session trust, network context, and account authority.

## Capabilities

### New Capabilities
- `connect-session-authority`: Defines the authority model for paired browser sessions, including session-bound network context and deferred account acquisition.

### Modified Capabilities
- `browser-wallet-pairing`: Pairing no longer binds an account; it establishes a trusted session and approved network only.
- `connect-example-web-app`: The example app becomes a sectioned API showcase whose paired shell can exist without account authority and whose feature pages request authority when needed.
- `typescript-connect-client`: Client semantics and documentation for `requestAccount` shift from session-context readback to account-authority acquisition for an existing session.

## Impact

- Affected code: `crates/ccd-wallet-connect`, `crates/ccd-wallet/src/commands/connect.rs`, `packages/ccd-wallet-connect-client`, and `packages/ccd-wallet-connect-example`.
- Affected dependencies: `packages/ccd-wallet-connect-example` will add `@concordium/web-sdk` for browser-side contract schema/type handling and require browser-reachable node access for embedded schema lookup.
- Affected behavior: pairing UX, session state, account-request handling, smart contract request preconditions, and example-app information architecture, including embedded-schema-only Smart Contracts preparation.
- Affected documentation: connect protocol docs, TypeScript client docs, and the example app README/spec coverage.
