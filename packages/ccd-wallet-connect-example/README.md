# @ccd-wallet/connect-example

A Vite 8 + React + TypeScript API showcase for `ccd-wallet connect`.

This example app demonstrates the staged authority model introduced by the connect protocol:

- configure the connect-server URL
- choose the target network genesis hash
- generate or regenerate a six-digit challenge shown in the browser
- pair through `@ccd-wallet/connect-client` to establish a trusted browser session
- enter a paired application shell with global session context and feature navigation
- request account authority explicitly only when a feature needs it
- use `@concordium/web-sdk` in the Smart Contracts page to prepare schema-aware parameter bytes
- submit smart contract init/update requests through `@ccd-wallet/connect-client`
- keep placeholder navigation for Transactions and Chain Updates while those areas are still pending
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

During pairing, the browser is the source of truth for the challenge. Pair first to establish a session, then request account authority only when a capability needs it.

## Smart Contracts page

The Smart Contracts section is the first fully implemented capability area in the paired shell.

It uses:

- `@concordium/web-sdk` for schema-aware parameter preparation from JSON values
- `@ccd-wallet/connect-client` for `requestAccount`, `requestContractInit`, and `requestContractUpdate`

The page deliberately gates account-backed forms behind an explicit account-authority action so integrators can see the staged flow clearly.

## Validate

```bash
pnpm --filter @ccd-wallet/connect-example check
pnpm --filter @ccd-wallet/connect-example test
pnpm --filter @ccd-wallet/connect-example build
```
