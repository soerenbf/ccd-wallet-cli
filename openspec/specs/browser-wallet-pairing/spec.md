# browser-wallet-pairing Specification

## Purpose
Define the browser-facing pairing flow for `ccd-wallet connect`, including temporary session hosting, origin-aware approval, challenge-based confirmation, fixed session context selection, and browser-readable approved context.
## Requirements
### Requirement: The connect API is hosted through a dedicated connect crate
The browser-pairing WebSocket JSON-RPC API SHALL be implemented in a dedicated `ccd-wallet-connect` crate. The CLI SHALL depend on that crate to host `ccd-wallet connect`, and the connect crate SHALL depend on `ccd-wallet-core` for wallet/domain integration logic rather than reimplementing storage behavior itself.

#### Scenario: CLI hosts connect through the dedicated connect crate
- **WHEN** the wallet starts `ccd-wallet connect`
- **THEN** the CLI hosts the browser-pairing API through the dedicated `ccd-wallet-connect` crate
- **AND** the connect implementation reuses wallet/domain logic from `ccd-wallet-core`

### Requirement: The wallet can enter an explicit browser-pairing session mode
The CLI SHALL provide a `ccd-wallet connect` command that starts a temporary localhost browser-pairing session. The wallet SHALL remain connectable only while that command is running and SHALL terminate the browser-facing session when the command exits.

#### Scenario: Connect mode starts a temporary browser-facing session
- **WHEN** the user runs `ccd-wallet connect`
- **THEN** the wallet starts a temporary localhost browser-facing session
- **AND** does not require a permanently running wallet daemon

#### Scenario: Connect mode ends the browser-facing session on exit
- **WHEN** the user stops `ccd-wallet connect`
- **THEN** the wallet closes the browser-facing session
- **AND** any paired browser session from that run becomes unusable

### Requirement: Browser pairing requires explicit approval with origin visibility
A browser dApp SHALL NOT be considered paired solely because it can reach the localhost session endpoint. The wallet SHALL validate the caller origin, display that origin to the user, and require explicit interactive approval before the browser becomes paired.

#### Scenario: Pairing request shows calling origin before approval
- **WHEN** a browser dApp requests pairing
- **THEN** the wallet shows the request as pending in the terminal
- **AND** includes the browser origin in the approval prompt

#### Scenario: Unapproved browser cannot read session context
- **WHEN** a browser dApp has requested pairing
- **AND** the user has not yet approved the request
- **THEN** the browser cannot retrieve the session network or account context

#### Scenario: Rejected pairing cannot create a usable session
- **WHEN** a browser dApp requests pairing
- **AND** the user rejects the request
- **THEN** the browser does not receive a usable paired session
- **AND** cannot read session context from that rejected attempt

### Requirement: Pairing uses a richer confirmation ceremony
The wallet SHALL require a pairing ceremony stronger than an origin-only approval. The browser pairing flow SHALL include an application-provided shared confirmation challenge or pairing code that is visible in the browser and validated by the wallet during approval.

The wallet approval UX SHALL prompt the user to enter the challenge shown in the calling application. The wallet SHALL validate the entered value against the pairing request challenge, but SHALL NOT redundantly display the challenge value itself in the wallet terminal during approval.

#### Scenario: Pairing approval prompts for the application-displayed challenge
- **WHEN** a browser dApp requests pairing
- **THEN** the wallet prompts the user to enter the challenge shown in the browser application
- **AND** validates the entered value against the challenge supplied in the pairing request
- **AND** does not display the challenge value itself in the wallet approval prompt

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

### Requirement: Only one paired browser session is active at a time
The first browser-pairing flow SHALL support only one active paired browser session at a time.

#### Scenario: A second browser cannot become simultaneously paired
- **WHEN** one browser dApp is already paired successfully
- **AND** another browser dApp attempts to pair
- **THEN** the wallet does not create a second simultaneous paired session
- **AND** rejects the new pairing request while the active session remains in place

### Requirement: Governance pairing is out of scope for the account pairing endpoint
The first browser-pairing endpoint SHALL cover account-oriented pairing only. Governance-key pairing SHALL NOT be multiplexed into the same endpoint or approval flow.

#### Scenario: Account pairing endpoint does not implicitly expose governance authority
- **WHEN** a browser dApp completes account-oriented pairing
- **THEN** the resulting paired session exposes only the approved account/network context for that flow
- **AND** does not imply governance-key pairing support

### Requirement: Approved pairing binds session context used for contract execution
The network and account selected during pairing approval SHALL be stored in the session state and used as the authoritative execution context for any contract execution request received in that session. The wallet SHALL NOT require or allow the browser to supply a different account or network for contract execution.

#### Scenario: Session state carries bound network and account after pairing
- **WHEN** the user approves a pairing request and selects network `testnet` and account `alice`
- **THEN** the active session state contains the genesis hash for `testnet` and the address for `alice`
- **AND** subsequent contract execution requests in the same session use those values

#### Scenario: Contract execution cannot override session-bound context
- **WHEN** a browser sends a contract execution request
- **AND** the request includes a different account address or network genesis hash than what is session-bound
- **THEN** the wallet ignores the supplied values and uses the session-bound context

