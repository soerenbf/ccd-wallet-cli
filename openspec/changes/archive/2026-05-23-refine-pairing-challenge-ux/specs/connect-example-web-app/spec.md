## MODIFIED Requirements

### Requirement: Example app demonstrates pairing through the TypeScript connect client
The example application SHALL depend on `@ccd-wallet/connect-client` and SHALL use the package's public API to perform pairing with an application-provided challenge.

The example application SHALL present the challenge in the browser UI as the value the user must enter into the wallet prompt during pairing.

#### Scenario: Example app pairs through the client package
- **WHEN** a user triggers pairing in the example application
- **THEN** the example application calls the TypeScript connect client package rather than reimplementing the protocol directly
- **AND** sends a pairing request with a visible six-digit challenge
- **AND** presents that challenge as the value the user should paste or type into the wallet prompt
