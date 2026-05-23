# @ccd-wallet/connect-example

A minimal Vite 8 + React + TypeScript integration reference for `ccd-wallet connect`.

This example app is intentionally small and developer-oriented. It demonstrates the currently supported connect flow only:

- configure the connect-server URL
- generate or regenerate a six-digit challenge shown in the browser
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

During pairing, the browser is the source of truth for the challenge. The app shows the six-digit value and the wallet asks you to enter that same value in the terminal prompt.

## Validate

```bash
pnpm --filter @ccd-wallet/connect-example check
pnpm --filter @ccd-wallet/connect-example test
pnpm --filter @ccd-wallet/connect-example build
```
