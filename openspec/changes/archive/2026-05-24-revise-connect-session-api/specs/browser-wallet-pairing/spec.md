## MODIFIED Requirements

### Requirement: Paired browsers can read the approved session context over the session channel
After a browser dApp has been successfully paired, the pairing response SHALL return only the approved session token for that session over the same WebSocket channel using JSON-RPC 2.0 semantics used for pairing. The pairing response SHALL NOT directly include selected network or account context.

A paired browser dApp SHALL be able to request account authority explicitly for a target network by supplying the session token and network genesis hash. When the wallet approves that request, it SHALL return the selected account address for that network.

#### Scenario: Successful pairing returns a session token only
- **WHEN** a browser dApp has completed pairing successfully
- **THEN** the pairing response includes the session token
- **AND** does not directly include selected network or account context

#### Scenario: Paired browser can request an account address for a network
- **WHEN** a browser dApp has completed pairing successfully
- **AND** it requests account authority for a specific network genesis hash using its session token
- **THEN** the wallet can approve an account for that network
- **AND** returns the selected account address

#### Scenario: Account requests remain scoped to the approved pairing
- **WHEN** one browser pairing has been approved
- **AND** another browser has not been approved
- **THEN** only the approved pairing can successfully request account authority for a network using its session token
