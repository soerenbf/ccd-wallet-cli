# @ccd-wallet/connect-example

A minimal Vite 8 + React + TypeScript integration reference for `ccd-wallet connect`.

This example app is intentionally small and developer-oriented. It demonstrates the currently supported connect flow only:

- configure the connect-server URL
- generate or regenerate a six-digit challenge
- request pairing through `@ccd-wallet/connect-client`
- display the returned session token, network genesis hash, and account address
- refresh approved session context
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

## Validate

```bash
pnpm --filter @ccd-wallet/connect-example check
pnpm --filter @ccd-wallet/connect-example test
pnpm --filter @ccd-wallet/connect-example build
```
