# @ccd-wallet/connect-client

Environment-flexible TypeScript client for `ccd-wallet connect`.

The package wraps the current connect protocol:

- WebSocket transport
- JSON-RPC 2.0 requests/responses
- `pair` to establish a trusted browser session for one approved network
- `requestAccount` to acquire account authority for that paired session when a capability needs it
- `requestContractInit` for wallet-approved smart contract initialization
- `requestContractUpdate` for wallet-approved smart contract receive-function execution

The wallet signs and submits approved contract transactions and returns the submitted transaction hash. Finalization is displayed locally by the CLI wallet.

## Install

From this repository workspace:

```bash
pnpm install
pnpm --filter @ccd-wallet/connect-client build
```

When published, applications can depend on the package normally through their package manager.

## Usage

```ts
import { createConnectClient } from "@ccd-wallet/connect-client";

const client = createConnectClient({
  url: "ws://127.0.0.1:22771",
});

await client.connect();

// 1. Pair to establish a session for one approved network.
const pairing = await client.pair("123456");
console.log(pairing.sessionToken);

// 2. Request account authority later, when a feature actually needs it.
const accountAddress = await client.requestAccount(
  pairing.sessionToken,
  "network-genesis-hash",
);
console.log(accountAddress);

// 3. Submit account-backed smart contract requests using the paired session.
const init = await client.requestContractInit({
  sessionToken: pairing.sessionToken,
  moduleRef: "0123...abcd",
  initName: "init_my_contract",
  amountMicroCcd: "0",
  maxContractExecutionEnergy: 30000,
  parameterHex: "",
  validate: true,
});
console.log(init.transactionHash);

const update = await client.requestContractUpdate({
  sessionToken: pairing.sessionToken,
  contractAddress: { index: 42, subindex: 0 },
  receiveName: "my_contract.transfer",
  amountMicroCcd: "0",
  maxContractExecutionEnergy: 30000,
  parameterHex: "2a",
  validate: true,
});
console.log(update.transactionHash);

client.close();
```

The application is responsible for displaying the same six-digit challenge to the user while the CLI asks the user to enter it.

## Session and authority semantics

Successful pairing returns only a session token. Pairing binds browser trust and network context, but it does **not** select an account.

Call `requestAccount(sessionToken, networkGenesisHash)` when your feature needs account-backed authority. The wallet validates that the requested network matches the session-bound network, may prompt the user to approve an account, and returns the approved account address. The wallet can cache that granted authority for the rest of the active session.

Account-backed methods such as `requestContractInit` and `requestContractUpdate` depend on previously granted session account authority. If the session does not have account authority yet, the server rejects those requests with an actionable error telling the caller to invoke `requestAccount` first.

## Contract parameter display

Contract parameters are serialized by the dApp and passed as `parameterHex`. Optionally pass a base64-encoded versioned module schema (or an object containing it in `base64`, `moduleSchema`, or `schema`) to let the wallet render human-readable parameters; otherwise the wallet displays hex.

## Runtime compatibility

The core client uses the standard WebSocket shape. In browsers, it uses `globalThis.WebSocket` by default. Other runtimes can pass a compatible constructor explicitly:

```ts
const client = createConnectClient({
  url: "ws://127.0.0.1:22771",
  WebSocket: MyWebSocketImplementation,
});
```

The public API avoids Node-specific runtime primitives so it can be reused by browser applications and future adapter packages.
