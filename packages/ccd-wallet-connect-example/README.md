# @ccd-wallet/connect-example

A minimal Vite 8 + React + TypeScript integration reference for `ccd-wallet connect`.

This example app is intentionally small and developer-oriented. It demonstrates the currently supported connect flow only:

- configure the connect-server URL
- enter the target network genesis hash
- generate or regenerate a six-digit challenge shown in the browser
- request pairing through `@ccd-wallet/connect-client`
- request an account address for the target network using the returned session token
- display the returned session token and account address
- reset local example state

It is not a production-ready wallet UI.

## Run

From the repository root:

```bash
pnpm install
pnpm --filter @ccd-wallet/connect-example dev
```

Then open the local Vite URL shown in the terminal. In another terminal, start the wallet connect server:

```bash
cargo run -p ccd-wallet -- connect
```

During pairing, the browser is the source of truth for the challenge. Pair first to establish a session, then request an account for the target network.

## Validate

```bash
pnpm --filter @ccd-wallet/connect-example check
pnpm --filter @ccd-wallet/connect-example test
pnpm --filter @ccd-wallet/connect-example build
```
