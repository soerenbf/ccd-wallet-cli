## MODIFIED Requirements

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
