## 1. Connect Crate: Session and Protocol Extension

- [x] 1.1 Extend `ActiveSession` in `crates/ccd-wallet-connect/src/lib.rs` to store `network_genesis_hash: String` and `account_address: String` alongside the existing session token.
- [x] 1.2 Extend `PairingApproval` to carry `network_genesis_hash` and `account_address`, and update `install_session` to populate them from the approval.
- [x] 1.3 Update `handle_request_account` to return the session-bound `account_address` without triggering new interactive selection; reject with an error if the supplied `networkGenesisHash` does not match the session-bound one.
- [x] 1.4 Add `REQUEST_CONTRACT_INIT_METHOD` and `REQUEST_CONTRACT_UPDATE_METHOD` constants.
- [x] 1.5 Add `ContractInitRequest`, `ContractUpdateRequest` structs carrying session-bound context plus method params (contract target, name, amount, max energy, parameter hex, optional schema).
- [x] 1.6 Add `ContractInitApproval` and `ContractUpdateApproval` structs carrying the submitted transaction hash.
- [x] 1.7 Add `ContractInitApprover` and `ContractUpdateApprover` callback type aliases on `ConnectServer`.
- [x] 1.8 Add `handle_contract_init` and `handle_contract_update` methods to `ConnectServer`, dispatching to the approver callbacks and mapping results to JSON-RPC responses.
- [x] 1.9 Add new JSON-RPC error codes: -32004 (user declined), -32005 (submission failed); update `json_rpc_error` usage and `method_descriptions`. Note: there is no blocking simulation error code — simulation failure is surfaced as a warning in the approval prompt, not as an RPC error.
- [x] 1.10 Wire `requestContractInit` and `requestContractUpdate` into the `handle_text_message` dispatch match.
- [x] 1.11 Add unit tests covering: valid contract init/update dispatch, invalid session token rejection, parse errors, and `install_session` with the extended fields.

## 2. Wallet Connect Command: Pairing and Execution Handlers

- [x] 2.1 Update `approve_pairing` in `crates/ccd-wallet/src/commands/connect.rs` to interactively select a network and account after challenge confirmation, unlock the account address, and return a `PairingApproval` carrying the selected `network_genesis_hash` and `account_address`.
- [x] 2.2 Simplify `approve_account_request` so it validates that the request's `network_genesis_hash` matches the session-bound one and returns the session-bound address without interactive selection.
- [x] 2.3 Add a helper to resolve the registered `NetworkEntry` (and thus node endpoint) from a genesis hash, returning an actionable error if no matching entry is found.
- [x] 2.4 Implement `approve_contract_update_request`: resolve node endpoint; if `validate: true`, run a dry-run with `invoke_instance` and capture the result (success or failure) for display — a failed simulation produces a warning, not a blocking error; render the approval prompt (origin, network, account, contract, entrypoint, amount, max energy, optional simulation output with warning if failed, decoded or hex parameter); present y/N confirmation; on approval unlock account signer, build and submit the update transaction, spawn finalization display, and return the transaction hash.
- [x] 2.5 Implement `approve_contract_init_request` following the same pattern as 2.4, using `ContractInitBuilder::dry_run_new_instance` when `validate: true` and `send::init_contract` for submission, printing the resulting contract address after finalization.
- [x] 2.6 Add a helper to decode parameter bytes for display: if a base64-encoded versioned module schema is supplied, attempt to decode the parameter using the SDK schema utilities and render as JSON; fall back to hex on any failure.
- [x] 2.7 Wire both new approver callbacks into the `ConnectServer::new` call in `commands/connect.rs::run`, sharing the `Arc<Mutex<Connection>>` pattern already used for other approvers.
- [x] 2.8 Add non-UI tests for: network entry resolution by genesis hash (match, no match, multiple aliases), approval prompt rendering with and without schema, and the `requestAccount` session-bound return path.

## 3. TypeScript Client: New Methods and Types

- [x] 3.1 Add `REQUEST_CONTRACT_INIT_METHOD` and `REQUEST_CONTRACT_UPDATE_METHOD` string constants to `packages/ccd-wallet-connect-client/src/core/types.ts`.
- [x] 3.2 Define `ContractAddress`, `ContractInitParams`, `ContractInitResult`, `ContractUpdateParams`, and `ContractUpdateResult` interfaces in `types.ts`.
- [x] 3.3 Add `packages/ccd-wallet-connect-client/src/features/contract-init.ts` implementing `requestContractInit` as a thin wrapper over `ConnectClient.request`.
- [x] 3.4 Add `packages/ccd-wallet-connect-client/src/features/contract-update.ts` implementing `requestContractUpdate` as a thin wrapper over `ConnectClient.request`.
- [x] 3.5 Add `requestContractInit` and `requestContractUpdate` methods to `ConnectClient` in `client.ts`, delegating to the feature modules, with full JSDoc.
- [x] 3.6 Export all new types and constants from `packages/ccd-wallet-connect-client/src/index.ts`.
- [x] 3.7 Add tests in `test/connect-client.test.ts` covering: `requestContractInit` sends the correct JSON-RPC request and resolves with `transactionHash`; `requestContractUpdate` sends the correct JSON-RPC request and resolves with `transactionHash`; server error responses reject with `ConnectClientError`.
- [x] 3.8 Build and verify the package: `pnpm --filter @ccd-wallet/connect-client build && pnpm --filter @ccd-wallet/connect-client test`.

## 4. Documentation

- [x] 4.1 Update `crates/ccd-wallet-connect/src/lib.rs` top-level doc comment to list all supported JSON-RPC methods including the two new ones.
- [x] 4.2 Update `packages/ccd-wallet-connect-client/README.md` to document `requestContractInit` and `requestContractUpdate` with usage examples.
- [x] 4.3 Update the root `README.md` to document the expanded connect protocol, noting that the session now binds one network and account at pairing, and showing example browser flows for contract init and update.
