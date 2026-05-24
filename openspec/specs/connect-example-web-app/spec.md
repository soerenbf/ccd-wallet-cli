# connect-example-web-app Specification

## Purpose
Define the example web application package used as an integration reference for the current `ccd-wallet connect` flow, including package setup, UI scope, connect-client usage, and documentation expectations.
## Requirements
### Requirement: Repository provides a browser example app package for connect integration
The repository SHALL provide a dedicated example web application package under the pnpm workspace for demonstrating browser-side connect integration.

#### Scenario: Example app package exists in the workspace
- **WHEN** a contributor inspects the pnpm workspace packages
- **THEN** the repository includes a dedicated example web application package for connect integration

### Requirement: Example app uses Vite with React and TypeScript
The example application SHALL use Vite with React and TypeScript.

#### Scenario: Example app uses React-based browser application structure
- **WHEN** a contributor inspects the example application package
- **THEN** the package uses Vite for browser application tooling
- **AND** the example logic is implemented with React and TypeScript

### Requirement: Example app demonstrates pairing through the TypeScript connect client
The example application SHALL depend on `@ccd-wallet/connect-client` and SHALL use the package's public API to perform pairing with an application-provided challenge.

After pairing succeeds, the example application SHALL use the returned session token together with a target network to request an account address through the client package.

#### Scenario: Example app pairs and then requests account authority
- **WHEN** a user triggers pairing in the example application
- **THEN** the example application calls the TypeScript connect client package rather than reimplementing the protocol directly
- **AND** sends a pairing request with a visible six-digit challenge
- **AND** uses the returned session token to request an account address for the target network

### Requirement: Example app displays approved session information
After a successful pairing and account-request flow, the example application SHALL display the returned session token, network genesis hash, and account address.

#### Scenario: Successful flow renders session token and approved account data
- **WHEN** the connect server approves pairing and then approves an account request for the example application
- **THEN** the example application displays the session token
- **AND** displays the target network genesis hash
- **AND** displays the approved account address

### Requirement: Example app supports session-context refresh and local reset
The example application SHALL allow the user to refresh approved session context and reset the local UI state.

#### Scenario: User refreshes session context after pairing
- **WHEN** the example application already has a valid session token
- **AND** the user requests a refresh
- **THEN** the example application retrieves approved session context again through the client library

#### Scenario: User resets local example state
- **WHEN** the user requests a reset in the example application
- **THEN** the example application clears its local pairing and session display state

### Requirement: Example app is positioned as an integration reference
The example application SHALL be documented and structured as an integration reference rather than a production-ready wallet UI.

#### Scenario: Example documentation describes reference-oriented purpose
- **WHEN** a contributor reads the example application documentation
- **THEN** the documentation explains that the app is an integration reference for the current connect flow
- **AND** does not present it as a production-ready wallet frontend

