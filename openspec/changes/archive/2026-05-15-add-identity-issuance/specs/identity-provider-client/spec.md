## ADDED Requirements

### Requirement: Identity provider HTTP client implements v1 issuance protocol
The HTTP client module SHALL implement the Concordium v1 identity issuance protocol start request and `code_uri` polling.

#### Scenario: Issuance start returns redirect URL
- **WHEN** `GET <issuanceStartUrl>?scope=identity&response_type=code&redirect_uri=<uri>&state=<json>` is sent
- **THEN** the client follows the redirect chain and returns the final redirected URL

#### Scenario: Issuance start does not redirect
- **WHEN** the IP responds without a redirect (non-3xx)
- **THEN** the client returns an error

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
The client SHALL stop following redirects when the redirect target matches the `redirect_uri` sentinel, rather than issuing a network request to it.

#### Scenario: Redirect to sentinel URI is not fetched
- **WHEN** the IP issues a redirect whose location contains the `redirect_uri` value
- **THEN** the client returns that location URL to the caller rather than fetching it
