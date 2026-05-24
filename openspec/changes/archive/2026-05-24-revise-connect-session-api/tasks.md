## 1. Connect server API revision

- [x] 1.1 Change the pairing response so `pair` returns only the session token.
- [x] 1.2 Add a dedicated account-request method that accepts a session token and network genesis hash and returns an approved account address.
- [x] 1.3 Update connect-server tests for the revised session-first flow.

## 2. TypeScript client revision

- [x] 2.1 Update the TypeScript client types so pairing returns only a session token.
- [x] 2.2 Add a client API for requesting an account address with a session token and network genesis hash.
- [x] 2.3 Update client tests and documentation for the revised flow.

## 3. Example app revision

- [x] 3.1 Update the example app flow to pair first and then request an account for the target network.
- [x] 3.2 Update the example app UI copy and tests to reflect the revised session-first model.
- [x] 3.3 Update root/example documentation to show the new pairing and account-request sequence.
