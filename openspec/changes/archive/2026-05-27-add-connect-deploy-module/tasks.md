## 1. Connect protocol and Rust server support

- [x] 1.1 Add `requestDeployModule` method constants, request/result types, and JSON-RPC dispatch in `crates/ccd-wallet-connect`.
- [x] 1.2 Add deploy-module session validation and response/error mapping in the connect server, ensuring deploy validation findings can be surfaced clearly to the wallet flow.
- [x] 1.3 Add unit tests in `crates/ccd-wallet-connect` covering valid deploy dispatch, invalid session rejection, and deploy-specific error mapping.

## 2. Rust wallet connect-command refactor and deploy flow

- [x] 2.1 Refactor `crates/ccd-wallet/src/commands/connect.rs` into `commands/connect/` with `mod.rs`, `shared.rs`, and feature modules for pairing, account, contract init, contract update, and deploy module.
- [x] 2.2 Move existing pairing, account, init, and update behavior into their feature modules without changing current behavior.
- [x] 2.3 Implement deploy-module request preparation and submission in `deploy_module.rs`, including module-hex parsing, derived module reference and module size display, account unlock, signing, submission, and readable background finalization printing.
- [x] 2.4 Implement optional deploy validation that derives the module reference and checks whether it already exists on chain, treating node failures and duplicate findings as approval warnings rather than automatic blockers.
- [x] 2.5 Add or update Rust tests for refactored helpers and deploy-module behavior, including duplicate-module validation and feature-specific finalization formatting where practical.

## 3. TypeScript client support

- [x] 3.1 Add `REQUEST_DEPLOY_MODULE_METHOD`, deploy request/result types, and exports in `packages/ccd-wallet-connect-client`.
- [x] 3.2 Add `src/features/deploy-module.ts` and a `ConnectClient.requestDeployModule` method with release-quality JSDoc.
- [x] 3.3 Add or update client tests covering successful `requestDeployModule` requests and server error rejection.

## 4. Example app deploy flow

- [x] 4.1 Extend the Smart Contracts example-app state model to support a Deploy Module mode adjacent to init and update.
- [x] 4.2 Add Deploy Module UI in `packages/ccd-wallet-connect-example`, including file upload, validate toggle, deploy status, and last transaction hash display.
- [x] 4.3 Implement browser-side file-to-hex conversion and connect-client submission for deploy requests, including local rejection when no file is selected.
- [x] 4.4 Add or update example-app tests covering deploy-mode state transitions and deploy request submission.

## 5. Documentation and project guidance

- [x] 5.1 Update connect-related Rust and TypeScript package documentation to describe `requestDeployModule` and the deploy flow.
- [x] 5.2 Record the mirrored feature-oriented connect module-organization decision in the repo `AGENTS.md`.
- [x] 5.3 Run the relevant Rust and TypeScript test suites for the affected connect packages and example app.
