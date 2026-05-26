## Why

The browser-wallet pairing protocol currently supports only session establishment and explicit account requests. Browser dApps have no way to propose smart contract transactions through the wallet, which means they cannot use the wallet as a signing authority for on-chain contract execution. Supporting contract init and update transactions is the next natural step in the connect protocol.

## What Changes

- Add a `requestContractInit` JSON-RPC method to the connect server for wallet-approved contract initialization transactions.
- Add a `requestContractUpdate` JSON-RPC method to the connect server for wallet-approved contract update (receive function invocation) transactions.
- Both methods accept an optional contract schema for human-readable parameter review; when omitted, the wallet falls back to hex display.
- Both methods require the caller to supply `maxContractExecutionEnergy`; the wallet dry-runs the transaction before prompting for approval and shows the estimated energy.
- The session is now authoritative: the network and account selected during pairing are bound to the session and used for all contract execution requests in that session without re-selection.
- The wallet returns only a transaction hash on success; finalization outcome is surfaced locally in the wallet terminal rather than through the RPC response.
- The TypeScript connect client exposes typed `requestContractInit` and `requestContractUpdate` methods.

## Capabilities

### New Capabilities

- `connect-smart-contract-execution`: JSON-RPC methods and wallet approval flow for submitting smart contract init and update transactions through a paired browser session, including schema-aware parameter review and energy estimation.

### Modified Capabilities

- `browser-wallet-pairing`: The paired session now binds one network and one account at pairing time, making the session context authoritative for subsequent contract execution requests.
- `typescript-connect-client`: New typed methods added for contract init and update requests.

## Impact

- Affected code: `crates/ccd-wallet-connect` (new JSON-RPC method constants, request/result types, dispatch), `crates/ccd-wallet/src/commands/connect.rs` (new approval handlers, simulation, signing, submission), `crates/ccd-wallet-core` (shared contract signing helpers if needed), `packages/ccd-wallet-connect-client` (new TS method types and implementations), and the connect example web app optionally.
- The `concordium-rust-sdk` contract client API (`send::init_contract`, `send::update_contract`, `invoke_instance`) and signer infrastructure are already available and used elsewhere in the codebase; no new SDK dependencies are required.
- The session state in `crates/ccd-wallet-connect` must be extended to carry the bound network genesis hash and account address alongside the existing session token.
