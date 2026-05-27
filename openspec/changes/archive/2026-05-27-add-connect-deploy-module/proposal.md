## Why

The current connect flow supports smart contract initialization and updates, but it does not let a paired browser dApp ask the wallet to deploy a smart contract module. That leaves the Smart Contracts showcase incomplete and pushes deploy flows outside the same end-to-end connect path just as the connect command structure is becoming harder to extend safely.

## What Changes

- Add a `requestDeployModule` JSON-RPC method to the connect server for wallet-approved smart contract module deployment with readable wallet-side review and finalization output.
- Add typed `requestDeployModule` support to `@ccd-wallet/connect-client`.
- Extend the example app Smart Contracts section with a Deploy Module mode adjacent to Contract Init and Contract Update.
- Make the example app accept module uploads as files, convert the file bytes to hex in-browser, and submit the resulting deploy request through the connect client.
- Support optional deploy validation that checks whether the derived module reference already exists on chain before prompting for approval, showing duplicate findings as wallet-side warnings rather than blocking the request in the browser.
- Refactor connect-related code toward mirrored feature-oriented module boundaries across the TypeScript client and Rust connect command so each connect capability has a corresponding module in both layers where practical.
- Record the connect module-organization decision in the project guidance.

## Capabilities

### New Capabilities
- `connect-module-deployment`: wallet-approved smart contract module deployment over the browser connect protocol, including optional preflight validation against existing on-chain modules.

### Modified Capabilities
- `connect-example-web-app`: extend the Smart Contracts section to support Deploy Module alongside Contract Init and Contract Update using a file-upload-based deploy flow.
- `typescript-connect-client`: add a typed `requestDeployModule` API and preserve feature-oriented capability mirroring with the Rust connect flow.

## Impact

- Affected Rust code: `crates/ccd-wallet-connect` for protocol and dispatch, and `crates/ccd-wallet/src/commands/connect` for wallet approval, validation, submission, and module refactoring.
- Affected TypeScript code: `packages/ccd-wallet-connect-client` for new request types and feature modules, and `packages/ccd-wallet-connect-example` for the Deploy Module UI and file handling.
- Public API impact: new `requestDeployModule` method and related request/result types in the TypeScript client and connect server protocol.
- Operational impact: deploy requests introduce larger WebSocket payloads because uploaded module bytes are transported as hex.
