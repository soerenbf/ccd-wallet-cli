# Tasks

## 1. Callback abstraction
- [x] Replace or extend the current post-hoc `CallbackReceiver` shape with a session-oriented callback abstraction that can provide a `redirect_uri` before the issuance URL is built.
- [x] Keep manual paste available through an explicit callback flag.
- [x] Share callback fragment parsing between manual and loopback modes.

## 2. Loopback callback receiver
- [x] Add a loopback callback implementation bound to `127.0.0.1` on an ephemeral port.
- [x] Generate a high-entropy nonce and include it in the callback path.
- [x] Serve an ultra-minimal callback HTML page for `GET /callback/<nonce>`.
- [x] Have the page post `window.location.hash` back to a local completion endpoint.
- [x] Accept only one callback result and shut down after completion, timeout, cancellation, or fallback.
- [x] Reject unexpected paths/nonces.

## 3. Identity issuance integration
- [x] Start the callback session before building the issuance URL.
- [x] Use the session-provided loopback URL as `redirect_uri`.
- [x] Open the returned browser URL as today.
- [x] Wait for the callback session result instead of prompting immediately for manual paste.
- [x] Continue polling the returned `code_uri` and storing the identity as today.
- [x] Preserve manual paste through an explicit flag only; loopback timeout should fail with retry guidance.

## 4. Redirect handling hardening
- [x] Ensure issuance start redirect handling works with full loopback redirect URIs, including encoded forms where relevant.
- [x] Ensure the preflight HTTP client does not fetch the final loopback redirect target itself.

## 5. Tests
- [x] Test loopback callback page serving.
- [x] Test loopback completion with `#code_uri=...`.
- [x] Test loopback completion with `#error=...`.
- [x] Test wrong nonce/path rejection.
- [x] Test single-use callback behavior.
- [x] Test manual fallback remains available.
- [x] Test issuance start redirect handling with a full loopback redirect URI.

## 6. Documentation
- [x] Update README identity issuance documentation to describe automatic browser callback.
- [x] Document manual paste fallback for environments where loopback callbacks do not work.
