## MODIFIED Requirements

### Requirement: Identity provider HTTP client implements v1 issuance protocol
The HTTP client module SHALL implement the Concordium v1 identity issuance protocol start request and `code_uri` polling.

#### Scenario: Issuance start returns redirect URL
- **WHEN** `GET <issuanceStartUrl>?scope=identity&response_type=code&redirect_uri=<uri>&state=<json>` is sent
- **THEN** the client follows the redirect chain until it reaches the final redirect URI
- **AND** returns the browser URL that should be opened or displayed to the user

#### Scenario: Issuance start does not redirect
- **WHEN** the IP responds without a redirect (non-3xx)
- **THEN** the client returns the original issuance URL for browser handoff

#### Scenario: Poll returns done
- **WHEN** `GET <code_uri>` returns `{"status":"done","token":{...}}`
- **THEN** the client returns the identity token

#### Scenario: Poll returns pending
- **WHEN** `GET <code_uri>` returns `{"status":"pending"}`
- **THEN** the client indicates the caller should retry after a delay

#### Scenario: Poll returns error
- **WHEN** `GET <code_uri>` returns `{"status":"error","detail":"..."}`
- **THEN** the client returns the error detail

### Requirement: Identity provider client does not follow the final redirect to redirect_uri
The client SHALL stop following redirects when the redirect target matches the configured `redirect_uri`, rather than issuing a network request to it. The configured `redirect_uri` MAY be the manual sentinel value or a full loopback callback URL.

#### Scenario: Redirect to manual sentinel URI is not fetched
- **WHEN** the IP issues a redirect whose location targets the manual sentinel redirect URI
- **THEN** the client returns that location URL to the caller rather than fetching it

#### Scenario: Redirect to loopback URI is not fetched
- **WHEN** the IP issues a redirect whose location targets `http://127.0.0.1:<port>/callback/<nonce>`
- **THEN** the client returns that location URL to the caller rather than fetching it

#### Scenario: Encoded redirect URI is detected
- **WHEN** the IP's redirect location represents the configured redirect URI in URL-encoded form
- **THEN** the client still recognizes it as the final redirect target and does not fetch it
