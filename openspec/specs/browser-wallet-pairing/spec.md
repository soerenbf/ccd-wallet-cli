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
For account-oriented browser pairing, the wallet SHALL require the user to choose a network and an account during pairing approval. The approved session SHALL bind to exactly that selected network and account for the duration of the session.

#### Scenario: Pairing approval chooses network and account
- **WHEN** the user approves an account-oriented pairing request
- **THEN** the wallet requires selection of one network and one account before the session is finalized

#### Scenario: Session context remains fixed after pairing
- **WHEN** an account-oriented browser session has been paired with network `testnet` and account `alice`
- **THEN** later browser context reads from that session report `testnet` and `alice`
- **AND** the session does not silently switch to a different network or account

### Requirement: Paired browsers can read the approved session context over the session channel
After a browser dApp has been successfully paired, it SHALL be able to query the approved session context for that session over the same WebSocket channel using JSON-RPC 2.0 semantics used for pairing. The context SHALL include the selected network identity as genesis hash and the selected account address needed for dApp preparation work.

#### Scenario: Paired browser can read selected network genesis hash and account address
- **WHEN** a browser dApp has completed pairing successfully
- **THEN** it can retrieve the approved session context
- **AND** that context includes the selected network genesis hash and selected account address

#### Scenario: Session context is scoped to the approved pairing
- **WHEN** one browser pairing has been approved
- **AND** another browser has not been approved
- **THEN** only the approved pairing can read its session context

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

