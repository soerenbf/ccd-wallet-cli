## MODIFIED Requirements

### Requirement: Identity issuance follows the Concordium v1 HTTP protocol
The CLI SHALL implement the v1 issuance protocol by orchestrating node queries, local storage, and the dedicated identity provider crate:
1. Resolve wallet-facing provider metadata from the selected network's `wallet_proxy`.
2. Unlock the selected seed once, yielding the seed phrase for request construction and the seed DEK for encrypted identity private payload storage.
3. Build and send the issuance start `GET` request to the provider's `issuanceStart` URL as a preflight step.
4. If the preflight returns a redirect, open the redirect target URL in the system browser (or print it as a fallback).
5. If the preflight does not return a redirect, open the original issuance URL in the system browser.
6. Receive the callback containing `code_uri`.
7. Store `code_uri` only inside the encrypted identity private payload.
8. Poll `code_uri` until status is `done` or `error`.
9. Store the resulting identity object only inside the encrypted identity private payload.

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
- **THEN** the CLI stores the identity object encrypted under the owning seed password domain
- **AND** prints a success message for the assigned identity label

#### Scenario: Identity provider reports error
- **WHEN** polling returns status `error`
- **THEN** the CLI deletes the pending identity row and its encrypted private payload
- **AND** exits with the error detail from the provider response

#### Scenario: Polling times out
- **WHEN** polling has not resolved within 5 minutes
- **THEN** the CLI exits with an error indicating the identity is still pending
- **AND** the stored `code_uri` remains encrypted under the owning seed password domain

#### Scenario: No plaintext identity private data is stored during issuance
- **WHEN** identity issuance stores `code_uri` or identity object data
- **THEN** neither value is written to SQLite as plaintext
