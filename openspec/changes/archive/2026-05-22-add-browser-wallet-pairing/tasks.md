## 1. Connect crate, CLI connect mode, and session lifecycle

- [x] 1.1 Add a dedicated `ccd-wallet-connect` crate for the WebSocket JSON-RPC pairing/session API.
- [x] 1.2 Wire the workspace and dependency layering so `ccd-wallet-connect` depends on `ccd-wallet-core`, and the CLI crate depends on both.
- [x] 1.3 Add a `connect` CLI command and top-level help text for browser pairing.
- [x] 1.4 Introduce temporary connected-wallet session startup/shutdown behavior for `ccd-wallet connect`.
- [x] 1.5 Implement the initial localhost WebSocket transport shape used by the pairing/session API.

## 2. Pairing handshake and approval

- [x] 2.1 Implement a browser pairing request flow that captures and validates caller origin.
- [x] 2.2 Add a richer pairing ceremony with a wallet-visible and browser-visible confirmation challenge/code.
- [x] 2.3 Add terminal approval/rejection UX for pairing requests.
- [x] 2.4 Reject new pairing requests while an active paired session already exists.

## 3. Session context selection and retrieval

- [x] 3.1 Reuse existing selection patterns so the user can choose the session network during pairing.
- [x] 3.2 Reuse existing selection patterns so the user can choose the session account during pairing.
- [x] 3.3 Persist temporary in-memory session context for the approved network/account and expose only the network genesis hash and account address through the browser API.

## 4. Validation, tests, and docs

- [x] 4.1 Add tests covering rejected pairing, successful pairing, and retrieval of approved session context.
- [x] 4.2 Add tests covering fixed session context semantics, including reconnect requirements for a different account/network.
- [x] 4.3 Update README and user-facing docs for `ccd-wallet connect`, pairing behavior, and the intentionally limited first API scope.
