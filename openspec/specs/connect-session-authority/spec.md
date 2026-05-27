# connect-session-authority Specification

## Purpose
TBD - created by archiving change decouple-connect-session-from-account-authority. Update Purpose after archive.
## Requirements
### Requirement: Paired browser sessions bind network context without requiring account authority
A successful browser pairing SHALL establish a trusted session token and bind exactly one approved network to that session. The session SHALL be valid even when no account authority has yet been granted.

#### Scenario: Pairing creates a network-bound session without account authority
- **WHEN** a browser dApp completes pairing successfully
- **THEN** the wallet returns a session token for an active paired session
- **AND** binds exactly one approved network to that session
- **AND** does not require an account to be selected during pairing

### Requirement: Paired sessions can acquire account authority after pairing
A paired browser session SHALL be able to acquire account authority explicitly after pairing for the session-bound network. When account authority is granted, the wallet SHALL return the approved account address and associate that authority with the active session.

#### Scenario: Paired session requests account authority for the bound network
- **WHEN** a browser dApp has completed pairing successfully
- **AND** it requests account authority for the network bound to the active session using its session token
- **THEN** the wallet prompts for account approval or selection as needed
- **AND** returns the approved account address
- **AND** associates that account authority with the active session

### Requirement: Account-backed connect methods require granted session account authority
Account-backed connect methods SHALL reject execution when the active paired session has not yet acquired account authority for its bound network. The rejection SHALL instruct the caller to request account authority first.

#### Scenario: Contract execution is rejected before account authority is granted
- **WHEN** a paired browser session calls an account-backed connect method
- **AND** the active session does not yet have granted account authority
- **THEN** the wallet rejects the request
- **AND** indicates that account authority must be requested before the method can succeed

