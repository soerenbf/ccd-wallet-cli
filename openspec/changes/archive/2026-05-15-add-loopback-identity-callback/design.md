# Design: Loopback identity callback receiver

## Current Flow

```text
ccd-wallet
   │
   ├─ builds identity request
   ├─ starts issuance with redirect_uri = ConcordiumRedirectToken
   ├─ opens browser URL
   │
browser
   │
   └─ finishes at ConcordiumRedirectToken#code_uri=...
              │
              └─ user copies URL and pastes into CLI
                         │
                         ▼
                   CLI parses code_uri
```

The existing `CallbackReceiver` abstraction receives a `browser_url` after the issuance URL has already been built. This is sufficient for manual paste, but too late for loopback, because a loopback receiver must first allocate a port and produce the `redirect_uri` used when building the issuance URL.

## Target Flow

```text
ccd-wallet
   │
   ├─ prepares callback session
   │    ├─ binds 127.0.0.1:<ephemeral-port>
   │    └─ generates /callback/<nonce>
   │
   ├─ redirect_uri = http://127.0.0.1:<port>/callback/<nonce>
   ├─ starts issuance using that redirect_uri
   ├─ opens browser URL
   │
browser
   │
   └─ lands at /callback/<nonce>#code_uri=...
             │
             ▼
       local callback page
             │
             ├─ reads window.location.hash
             └─ POSTs hash to /callback/<nonce>/complete
                         │
                         ▼
                   CLI receives code_uri
                         │
                         └─ polls code_uri and stores identity
```

## Callback Session Abstraction

The callback mechanism should become session-oriented:

```text
prepare session
    -> redirect_uri
    -> wait_for_result
```

Conceptually:

```text
CallbackSession
  redirect_uri() -> Url/String
  wait_for_callback(browser_url) -> Result<String code_uri>
```

The exact Rust shape can vary, but the important sequencing is:

1. create callback session
2. obtain redirect URI
3. build/start issuance with redirect URI
4. open browser URL
5. wait for callback result

Manual paste can be represented as a session whose redirect URI is still the legacy sentinel or as a fallback receiver invoked if loopback fails/times out.

## Loopback Receiver Behavior

- Bind to `127.0.0.1:0` to allocate an ephemeral port.
- Generate a high-entropy nonce and include it in the callback path.
- Accept only the expected callback path.
- Serve an ultra-minimal HTML page for `GET /callback/<nonce>`.
- The page reads `window.location.hash` and sends it to `POST /callback/<nonce>/complete`.
- The server parses the submitted fragment using the same callback parsing semantics as manual paste.
- The server returns a minimal completion page/message.
- The receiver resolves once with either `code_uri` or provider error.
- The receiver is single-use and shuts down after success, error, timeout, cancellation, or fallback.

## Minimal Browser Page

The page only needs to bridge the fragment:

```text
Finishing identity issuance...
```

Its script:

1. reads `window.location.hash`
2. POSTs the hash/body to the local completion endpoint
3. replaces the page body with either:
   - "Identity callback received. You can close this tab."
   - or "Identity callback failed. Return to ccd-wallet."

No rich styling or packaged desktop integration is required.

## Fallback Strategy

Manual paste remains available through an explicit flag only:

```text
ccd-wallet identity new <LABEL> --interactive --manual-callback
```

The CLI should not automatically switch to manual paste after a loopback timeout. If loopback callback mode times out or fails, the command should exit with an actionable error explaining that the user can retry with the manual callback flag.

## Redirect Handling

The issuance client currently detects the final redirect by checking whether the redirect `Location` contains the configured `redirect_uri`. With a full loopback URL, redirect detection should remain careful about:

- exact URI string matching where possible
- percent-encoded redirect URI values
- relative redirect locations resolved against the current URL
- not following the final redirect target as an HTTP request from the client

The browser, not the preflight HTTP client, should be the actor that eventually visits the loopback redirect URI.

## Security Notes

- Bind only to `127.0.0.1`, never `0.0.0.0`, `localhost`, or `::1`.
- Use a nonce-bearing path.
- Accept a single callback result.
- Time-bound the callback session.
- Validate callback content before polling:
  - `#code_uri=` must be present for success
  - `#error=` should surface provider error
  - `code_uri` should parse as an absolute URL
- Do not log sensitive callback fragments unnecessarily.

## Non-Goals

- No hosted relay.
- No cross-device browser support.
- No concurrent issuance sessions in one process.
- No background daemon.
- No change to identity storage timing; pending rows are still inserted only after `code_uri` is received.

## Testing Approach

- Unit-test callback URL/fragment parsing remains shared between manual and loopback modes.
- Unit/integration-test the local server using a local HTTP client:
  - GET callback page returns HTML
  - POST completion with `#code_uri=...` resolves the waiting session
  - POST completion with `#error=...` resolves as an error
  - wrong nonce/path is rejected
  - second completion attempt is rejected or ignored
- Keep existing manual paste parser tests.
- Keep existing issuance client redirect tests and add coverage for full loopback redirect URIs.
