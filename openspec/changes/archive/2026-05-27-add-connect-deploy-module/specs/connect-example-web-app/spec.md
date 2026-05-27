## ADDED Requirements

### Requirement: Smart Contracts section supports module deployment adjacent to init and update
The example application's Smart Contracts section SHALL support a Deploy Module flow alongside the existing Contract Init and Contract Update flows.

The Deploy Module flow SHALL:
- be reachable from the same Smart Contracts section navigation as init and update
- accept a module file upload as its input surface
- convert the uploaded file bytes to hex before sending the request through `@ccd-wallet/connect-client`
- allow the user to request deploy validation before submission
- display deploy-specific status and the last returned transaction hash

#### Scenario: Smart Contracts section shows Deploy Module adjacent to init and update
- **WHEN** a user opens the Smart Contracts section with account authority available
- **THEN** the application shows a Deploy Module flow alongside Contract Init and Contract Update

#### Scenario: Deploy Module flow uploads a file and submits through the connect client
- **WHEN** a user selects a smart contract module file in the Deploy Module flow
- **AND** submits the deploy request
- **THEN** the application converts the uploaded bytes to hex
- **AND** calls `@ccd-wallet/connect-client` rather than reimplementing the protocol directly

#### Scenario: Deploy Module flow supports optional validation
- **WHEN** a user enables validation in the Deploy Module flow
- **AND** submits the deploy request
- **THEN** the application sends the deploy request with `validate: true`

#### Scenario: Deploy Module flow requires a file before submission
- **WHEN** a user attempts to submit the Deploy Module flow without selecting a module file
- **THEN** the application rejects the attempt locally
- **AND** explains that a module file must be selected first
