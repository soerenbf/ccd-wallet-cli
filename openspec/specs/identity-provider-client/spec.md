# identity-provider-client Specification

## Purpose
TBD - created by archiving change add-identity-issuance. Update Purpose after archive.
## Requirements
### Requirement: Identity provider HTTP client implements v1 issuance protocol
The dedicated identity provider crate SHALL implement the Concordium v1 identity issuance protocol start request and `code_uri` polling.

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

### Requirement: Identity provider client supports recovery requests
The dedicated identity provider client SHALL support Concordium identity recovery by constructing a recovery-start URL from provider metadata and issuing a recovery request whose serialized state contains the generated `idRecoveryRequest`.

#### Scenario: Recovery request returns identity object
- **WHEN** the client sends a recovery request to `<recoveryStart>?state=<json>` for a valid seed-derived identity candidate
- **AND** the provider returns a successful response containing an identity object
- **THEN** the client returns that identity object to the caller

#### Scenario: Recovery request reports missing identity without crashing the flow
- **WHEN** the client sends a recovery request for a candidate identity that does not exist
- **AND** the provider returns a non-success response indicating that no identity was recovered
- **THEN** the client reports a recoverable miss to the caller
- **AND** does not convert that miss into a fatal process error

#### Scenario: Missing recovery start metadata is rejected before request construction
- **WHEN** recovery is requested for a provider that lacks `recoveryStart` metadata
- **THEN** the client returns an actionable error
- **AND** does not attempt to synthesize a recovery URL from unrelated provider fields

