## ADDED Requirements

### Requirement: Example app organizes connect capabilities into dedicated API sections
The example application SHALL present the paired experience as a sectioned API showcase with navigation between capability areas. The application SHALL include a Smart Contracts section and SHALL reserve visible navigation space for Transactions and Chain Updates.

#### Scenario: Paired application shell shows API-area navigation
- **WHEN** the example application has an active paired session
- **THEN** it displays navigation between dedicated API sections
- **AND** includes Smart Contracts in that navigation
- **AND** surfaces Transactions and Chain Updates as additional sections in the showcase structure

### Requirement: Example app surfaces feature-specific authority prerequisites
The example application SHALL distinguish between pairing state and account-authority state. Feature areas that require an account SHALL indicate when account authority has not yet been granted and SHALL offer an explicit way to request it.

#### Scenario: Smart Contracts section shows missing account authority state
- **WHEN** the example application has a paired session without granted account authority
- **AND** the user opens the Smart Contracts section
- **THEN** the application indicates that account authority is required for Smart Contracts actions
- **AND** offers an explicit action to request account authority for the active session

### Requirement: Smart Contracts section uses `@concordium/web-sdk` for embedded-schema-aware contract workflows
The example application's Smart Contracts section SHALL use `@concordium/web-sdk` for browser-side smart contract schema and type handling. The application SHALL derive schema-aware contract interaction data from embedded module schema while continuing to submit pairing, account, and contract requests through `@ccd-wallet/connect-client`.

The example application SHALL support only Smart Contracts flows whose relevant module exposes embedded schema. The application SHALL NOT require the user to paste schema bytes manually.

#### Scenario: Smart Contracts init flow derives embedded schema from the referenced module
- **WHEN** a user prepares a schema-aware contract init flow in the Smart Contracts section
- **AND** supplies a module reference for a module with embedded schema
- **THEN** the example application uses `@concordium/web-sdk` together with node access to fetch the embedded schema for that module
- **AND** uses the derived schema to serialize the provided JSON parameter value
- **AND** uses `@ccd-wallet/connect-client` to send the resulting connect request to the wallet

#### Scenario: Smart Contracts update flow derives embedded schema from the target instance module
- **WHEN** a user prepares a schema-aware contract update flow in the Smart Contracts section
- **AND** supplies a contract instance address whose source module has embedded schema
- **THEN** the example application queries the target instance to determine its source module
- **AND** fetches the embedded schema for that module using `@concordium/web-sdk`
- **AND** uses the derived schema to serialize the provided JSON parameter value
- **AND** uses `@ccd-wallet/connect-client` to send the resulting connect request to the wallet

#### Scenario: Smart Contracts page rejects modules without embedded schema
- **WHEN** a user prepares a Smart Contracts flow for a module that does not expose embedded schema
- **THEN** the example application rejects the preparation attempt
- **AND** explains that the showcase supports only contracts with embedded schema

## MODIFIED Requirements

### Requirement: Example app demonstrates pairing through the TypeScript connect client
The example application SHALL depend on `@ccd-wallet/connect-client` and SHALL use the package's public API to perform pairing with an application-provided challenge.

After pairing succeeds, the example application SHALL transition into a paired application shell that uses the returned session token together with the approved network context and browser-reachable node access to drive capability-specific actions. When a feature area requires an account, the example application SHALL request an account address through the client package for the active paired session.

#### Scenario: Example app pairs into a session-gated showcase shell
- **WHEN** a user triggers pairing in the example application
- **THEN** the example application calls the TypeScript connect client package rather than reimplementing the protocol directly
- **AND** sends a pairing request with a visible six-digit challenge
- **AND** enters a paired application shell when pairing succeeds

#### Scenario: Example app requests account authority when a feature needs it
- **WHEN** the example application already has a valid paired session
- **AND** the user enters an account-backed feature area and requests account authority
- **THEN** the example application uses the returned session token to request an account address for the target network

### Requirement: Example app displays approved session information
After a successful pairing, the example application SHALL display the returned session token, target network genesis hash, and the node access context used for Smart Contracts lookups as global paired-session context. If account authority has been granted for the session, the example application SHALL also display the approved account address.

#### Scenario: Successful pairing renders session context before account authority exists
- **WHEN** the connect server approves pairing for the example application
- **THEN** the example application displays the session token
- **AND** displays the target network genesis hash
- **AND** displays the node access context used for Smart Contracts lookups
- **AND** can show that no account authority has yet been granted

#### Scenario: Approved account authority updates the paired session display
- **WHEN** the connect server later approves an account request for the example application
- **THEN** the example application displays the approved account address alongside the paired session context

### Requirement: Example app supports session-context refresh and local reset
The example application SHALL allow the user to request account authority for the active paired session and reset the local UI state.

#### Scenario: User requests account authority after pairing
- **WHEN** the example application already has a valid session token
- **AND** the user requests account authority for the active session
- **THEN** the example application retrieves the approved account address through the client library

#### Scenario: User resets local example state
- **WHEN** the user requests a reset in the example application
- **THEN** the example application clears its local pairing and session display state
