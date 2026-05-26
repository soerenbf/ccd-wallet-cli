## MODIFIED Requirements

### Requirement: Approved pairing binds one network and one account as session context
For account-capable browser pairing, the wallet SHALL require the user to choose a network during pairing approval. The approved session SHALL bind to exactly that selected network for the duration of the session.

The wallet SHALL NOT require account selection during pairing approval, and the paired session MAY exist without account authority until a later `requestAccount` call grants it.

#### Scenario: Pairing approval chooses a network without requiring an account
- **WHEN** the user approves a browser pairing request
- **THEN** the wallet requires selection of exactly one network before the session is finalized
- **AND** does not require selection of an account during pairing approval

#### Scenario: Session network context remains fixed after pairing
- **WHEN** a browser session has been paired with network `testnet`
- **THEN** later browser requests from that session remain scoped to `testnet`
- **AND** the session does not silently switch to a different network

### Requirement: Paired browsers can read the approved session context over the session channel
After a browser dApp has been successfully paired, the pairing response SHALL return only the approved session token for that session over the same WebSocket channel using JSON-RPC 2.0 semantics used for pairing. The pairing response SHALL NOT directly include selected network or account context.

A paired browser dApp SHALL be able to request account authority explicitly for the session-bound network by supplying the session token and network genesis hash. When the wallet approves that request, it SHALL return the selected account address for that network and associate that account authority with the active session.

#### Scenario: Successful pairing returns a session token only
- **WHEN** a browser dApp has completed pairing successfully
- **THEN** the pairing response includes the session token
- **AND** does not directly include selected network or account context

#### Scenario: Paired browser can request an account address for its bound network
- **WHEN** a browser dApp has completed pairing successfully
- **AND** it requests account authority for the session-bound network genesis hash using its session token
- **THEN** the wallet can approve an account for that session
- **AND** returns the selected account address

#### Scenario: Account requests remain scoped to the approved pairing and network
- **WHEN** one browser pairing has been approved for network `testnet`
- **AND** the browser requests account authority for a different network genesis hash
- **THEN** the wallet does not grant account authority for that different network
- **AND** the session remains scoped to its originally approved network
