## 1. Connect session model and pairing flow

- [ ] 1.1 Update `crates/ccd-wallet-connect` session state so active sessions store the approved network but can exist without account authority.
- [ ] 1.2 Update pairing approval and session installation to select only a network during `pair` and stop requiring account selection at pairing time.
- [ ] 1.3 Add or update connect-server tests covering pairing success without account authority, fixed session network context, and single-session behavior.

## 2. Account authority acquisition and account-backed request guards

- [ ] 2.1 Rework `requestAccount` handling so it validates the session token and bound network, prompts for account approval when authority is missing, and stores the approved account in session state.
- [ ] 2.2 Update smart contract init/update request handling to require previously granted session account authority and return an actionable missing-authority error when none is present.
- [ ] 2.3 Add Rust tests covering first-time account acquisition, repeated account requests for the same session, network mismatch rejection, and contract-request rejection before account authority is granted.

## 3. TypeScript client documentation and behavior alignment

- [ ] 3.1 Update `@ccd-wallet/connect-client` documentation and examples to describe pairing as session establishment and `requestAccount` as account-authority acquisition.
- [ ] 3.2 Adjust connect-client tests, fixtures, or error expectations as needed to match the new staged authority semantics.
- [ ] 3.3 Build and verify the connect-client package after the semantic/documentation updates.

## 4. Example app API showcase shell

- [ ] 4.1 Refactor `packages/ccd-wallet-connect-example` state model to represent paired-session context separately from optional account authority state.
- [ ] 4.2 Restructure the React UI into an unpaired pairing screen and a paired application shell with global session context and navigation.
- [ ] 4.3 Add `@concordium/web-sdk` to the example app and introduce schema/type helpers for the Smart Contracts section without coupling that dependency into `@ccd-wallet/connect-client`.
- [ ] 4.4 Implement the Smart Contracts section authority gate so it surfaces missing account authority and offers an explicit account-request action before account-backed forms are enabled.
- [ ] 4.5 Use `@concordium/web-sdk` in the Smart Contracts section to support schema-aware contract input preparation and connect-request payload construction.
- [ ] 4.6 Add placeholder navigation/pages for Transactions and Chain Updates to establish the showcase structure without implementing those API areas yet.
- [ ] 4.7 Update example-app tests, README, and any relevant styling to cover the new shell, navigation, deferred-account flow, and `web-sdk`-assisted Smart Contracts behavior.

## 5. Repository documentation and spec alignment

- [ ] 5.1 Update connect protocol documentation and README content to explain that pairing binds network trust while account authority is acquired later.
- [ ] 5.2 Verify the new OpenSpec artifacts align with implementation naming and adjust any cross-references during implementation if needed.
