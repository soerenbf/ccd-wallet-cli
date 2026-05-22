## Why

The repository now has both a browser-facing connect server and a TypeScript client library, but there is no runnable reference application showing how an actual web app should tie them together. A small example app would make the intended integration flow concrete, provide an end-to-end sanity check, and give developers something easy to copy from.

This example is most valuable as an integration reference, not as a polished product demo. Keeping it small and explicit will help validate the client API while showing the browser-side pairing and session-context flow in a realistic app setup.

## What Changes

- Add a new example web application package under the pnpm workspace.
- Implement the example app using Vite and React with TypeScript.
- Make the example app depend on `@ccd-wallet/connect-client` and use it directly.
- Demonstrate the current supported flow only:
  - configure the connect-server URL
  - generate or regenerate a six-digit pairing challenge
  - request pairing
  - display returned session token, network genesis hash, and account address
  - refresh approved session context and reset the local app state
- Position the app as an integration reference and executable example rather than a production-ready wallet UI.

## Capabilities

### New Capabilities
- `connect-example-web-app`: A minimal Vite + React TypeScript web application that demonstrates pairing with the connect server and retrieving approved session context through the TypeScript client library.

### Modified Capabilities

## Impact

- Affected code: a new example app package under `packages/`, root pnpm workspace wiring if needed, and developer-facing documentation referencing the example.
- Affected systems: browser-side integration flow, local development workflow for example apps, and end-to-end validation of the connect client package.
- Dependencies: Vite, React, and minimal browser-app tooling for a TypeScript reference app.
