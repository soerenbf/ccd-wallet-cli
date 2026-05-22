# @ccd-wallet/connect-client

Environment-flexible TypeScript client for `ccd-wallet connect`.

The package wraps the current connect protocol:

- WebSocket transport
- JSON-RPC 2.0 requests/responses
- `pair` with an application-provided challenge
- `session.getContext` with a session token

It intentionally does not include transaction proposal, signing, submission, or governance-key pairing APIs yet.

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

const pairing = await client.pair("123456");

console.log(pairing.sessionToken);
console.log(pairing.context.networkGenesisHash);
console.log(pairing.context.accountAddress);

const context = await client.getSessionContext(pairing.sessionToken);

client.close();
```

The application is responsible for displaying the same six-digit challenge to the user while the CLI asks the user to confirm it.

## Runtime compatibility

The core client uses the standard WebSocket shape. In browsers, it uses `globalThis.WebSocket` by default. Other runtimes can pass a compatible constructor explicitly:

```ts
const client = createConnectClient({
  url: "ws://127.0.0.1:22771",
  WebSocket: MyWebSocketImplementation,
});
```

The public API avoids Node-specific runtime primitives so it can be reused by browser applications and future adapter packages.
