## Why

The current identity issuance flow requires the user to copy the final browser redirect URL and paste it back into the CLI. This works and is robust, but it is awkward for normal local CLI usage where the wallet and browser are on the same machine.

## What Changes

- Replace the default manual paste callback step in `ccd-wallet identity new` with an OAuth-style local loopback callback receiver.
- Start a temporary HTTP server bound to `127.0.0.1` on an ephemeral port.
- Use a nonce-bearing loopback callback URL as the issuance `redirect_uri`.
- Serve an ultra-minimal callback page that reads `window.location.hash` and posts it back to the local server.
- Accept exactly one callback result, then shut down the local listener.
- Keep manual paste available only through an explicit flag.
- Keep support for one in-flight identity issuance per CLI process.
- Keep identity storage timing unchanged: pending rows are inserted only after `code_uri` is received.

## Out of Scope

- A packaged desktop app.
- A long-running wallet daemon.
- A hosted callback relay service.
- Cross-device or remote-browser callback support.
- Multiple concurrent identity issuance sessions.
- Rich branded success/error callback pages.

## Risks

- Browser security model: URL fragments are not sent to HTTP servers, so the local callback page must bridge the fragment back with JavaScript.
- Some browser/network setups may block loopback access; explicit manual paste mode mitigates this.
- Local callback spoofing by another local process is possible in principle; a nonce-bearing single-use path reduces accidental or opportunistic spoofing.
- Redirect matching must handle full loopback URLs, not only the previous sentinel string.
