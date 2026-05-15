## MODIFIED Requirements

### Requirement: Identity issuance follows the Concordium v1 HTTP protocol
The CLI SHALL implement the v1 issuance protocol:
1. Resolve wallet-facing provider metadata from the selected network's `wallet_proxy`.
2. Prepare a callback session and obtain its `redirect_uri`.
3. Build and send the issuance start `GET` request to the provider's `issuanceStart` URL as a preflight step using that `redirect_uri`.
4. If the preflight returns a redirect, open the redirect target URL in the system browser (or print it as a fallback).
5. If the preflight does not return a redirect, open the original issuance URL in the system browser.
6. Receive the callback containing `code_uri` through the configured callback transport.
7. Poll `code_uri` until status is `done` or `error`.
8. Store the resulting identity object.

#### Scenario: Wallet proxy does not provide provider metadata
- **WHEN** the selected `wallet_proxy` does not return metadata for the chosen identity provider
- **THEN** the CLI exits with an actionable error before browser handoff

#### Scenario: Identity provider responds with a redirect
- **WHEN** the IP's issuance start endpoint responds with a redirect
- **THEN** the CLI opens the redirect target URL in the browser

#### Scenario: Identity provider responds with a browser entry page
- **WHEN** the IP's issuance start endpoint responds without a redirect
- **THEN** the CLI opens the original issuance URL in the browser instead of failing early

#### Scenario: Successful issuance flow
- **WHEN** the full issuance flow completes with status `done`
- **THEN** the CLI stores the identity object and prints a success message for the assigned identity label

#### Scenario: Identity provider reports error
- **WHEN** polling returns status `error`
- **THEN** the CLI exits with the error detail from the provider response

#### Scenario: Polling times out
- **WHEN** polling has not resolved within 5 minutes
- **THEN** the CLI exits with an error indicating the identity is still pending and the `code_uri` is stored for later retry

### Requirement: Browser handoff uses manual callback paste
The CLI SHALL keep manual callback paste available as an explicit callback transport selected by flag. In manual mode, the CLI prints the browser URL and prompts the user to paste the final redirect URL after completing browser-based identity verification. The CLI SHALL NOT automatically switch from loopback mode to manual paste after a loopback timeout.

#### Scenario: CLI prints provider URL and prompts for callback in manual mode
- **WHEN** the browser handoff step uses manual callback mode
- **THEN** the CLI prints the URL to open and instructs the user to paste the final redirect URL

#### Scenario: Pasted URL contains code_uri fragment
- **WHEN** the user pastes a URL of the form `<redirect_uri>#code_uri=<url>`
- **THEN** the CLI extracts `<url>` and proceeds to poll it

#### Scenario: Pasted URL contains error fragment
- **WHEN** the user pastes a URL containing `#error=<detail>`
- **THEN** the CLI exits with the error detail

#### Scenario: Pasted URL is unrecognisable
- **WHEN** the user pastes a URL that contains neither `#code_uri=` nor `#error=`
- **THEN** the CLI exits with an error asking the user to paste the correct URL

## ADDED Requirements

### Requirement: Browser handoff uses loopback callback by default
The CLI SHALL use a local loopback callback receiver by default for identity issuance. The receiver SHALL bind only to `127.0.0.1` on an ephemeral port, generate a single-use nonce-bearing callback path, and provide that URL as the issuance `redirect_uri`.

#### Scenario: Loopback callback session provides redirect URI
- **WHEN** identity issuance reaches browser handoff setup
- **THEN** the CLI starts a local loopback callback receiver
- **AND** uses a redirect URI of the form `http://127.0.0.1:<port>/callback/<nonce>` for the issuance request

#### Scenario: Browser callback fragment is bridged to CLI
- **WHEN** the browser lands on the loopback callback URL with `#code_uri=<url>` in the fragment
- **THEN** the local callback page reads the fragment in the browser
- **AND** posts it back to the loopback receiver
- **AND** the CLI extracts `<url>` and proceeds to poll it

#### Scenario: Browser callback error is bridged to CLI
- **WHEN** the browser lands on the loopback callback URL with `#error=<detail>` in the fragment
- **THEN** the local callback page posts the fragment back to the loopback receiver
- **AND** the CLI exits with the provider error detail

#### Scenario: Callback page is minimal
- **WHEN** the browser loads the loopback callback page
- **THEN** the page displays only minimal status text needed to complete the handoff and tell the user they may close the tab

#### Scenario: Loopback callback is single-use
- **WHEN** the loopback receiver accepts a callback result
- **THEN** it resolves the waiting issuance flow
- **AND** refuses or ignores subsequent completion attempts for the same nonce
- **AND** shuts down the local callback listener

#### Scenario: Unexpected callback path is rejected
- **WHEN** a request uses the wrong callback path or nonce
- **THEN** the loopback receiver rejects the request without completing the issuance flow

#### Scenario: Explicit manual mode bypasses loopback
- **WHEN** the user selects manual callback mode with the explicit flag
- **THEN** the CLI does not start a loopback callback receiver
- **AND** uses manual callback paste for browser handoff

#### Scenario: Loopback timeout does not automatically fall back
- **WHEN** loopback callback mode times out before receiving a callback
- **THEN** the CLI exits with an actionable error explaining that the user can retry with the manual callback flag

#### Scenario: Pending identity row is inserted after code URI is received
- **WHEN** loopback mode starts and browser handoff begins
- **THEN** no pending identity row is inserted yet
- **WHEN** the callback receiver obtains a `code_uri`
- **THEN** the CLI inserts the pending identity row as in the existing issuance flow
