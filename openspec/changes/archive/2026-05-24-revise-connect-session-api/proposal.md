## Why

The current connect API makes pairing inherently account-shaped by returning approved network and account context immediately. That works for the first account flow, but it does not fit governance-oriented use cases where the relevant authority is network-scoped and has no account address.

A cleaner direction is to make pairing establish trust only, then let applications explicitly request the authority they need for a target network. This keeps the session model simpler and creates a better foundation for both future account transaction methods and future governance update methods.

## What Changes

- Change pairing so `pair` returns only a session token.
- Add a dedicated account-request step where applications request an account address for a specific network using the session token.
- Update the TypeScript client to expose the session-first flow.
- Update the example web application to pair first and then request an account for the target network.
- Establish the API direction for future methods such as transaction proposals and chain updates without implementing those methods in this change.

## Capabilities

### New Capabilities

### Modified Capabilities
- `browser-wallet-pairing`: revise pairing and session-follow-up behavior so pairing establishes a session and account authority is requested separately for a target network
- `typescript-connect-client`: revise the client API so pairing returns only a session token and account selection becomes an explicit network-scoped request
- `connect-example-web-app`: revise the example flow so it pairs first and then requests an account address for the chosen network

## Impact

- Affected code: connect server RPC methods, wallet-side session handling, the TypeScript client package, and the example web application.
- Affected systems: browser-wallet pairing flow, account selection flow, and future extensibility toward governance and proposal methods.
- User-facing behavior: applications pair first to obtain a session, then request an account for a network explicitly instead of receiving account/network context directly from pairing.
