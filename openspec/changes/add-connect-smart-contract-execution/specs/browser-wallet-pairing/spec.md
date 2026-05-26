## MODIFIED Requirements

### Requirement: Paired browsers can read the approved session context over the session channel
After a browser dApp has been successfully paired, the pairing response SHALL return only the approved session token for that session over the same WebSocket channel using JSON-RPC 2.0 semantics used for pairing. The pairing response SHALL NOT directly include selected network or account context.

A paired browser dApp SHALL be able to retrieve the approved account address and network genesis hash for the active session by calling `requestAccount` with the session token. The returned account address and network genesis hash SHALL reflect the account and network bound to the session at pairing time. The `networkGenesisHash` field in the `requestAccount` params SHALL be accepted for compatibility and MAY be used to confirm that the session-bound network matches the caller's expectation, but SHALL NOT trigger new interactive account selection.

#### Scenario: Successful pairing returns a session token only
- **WHEN** a browser dApp has completed pairing successfully
- **THEN** the pairing response includes the session token
- **AND** does not directly include selected network or account context

#### Scenario: Paired browser can retrieve the session-bound account address
- **WHEN** a browser dApp has completed pairing successfully
- **AND** it calls `requestAccount` with the session token
- **THEN** the wallet returns the account address bound to the session at pairing time
- **AND** does not trigger new interactive account selection

#### Scenario: requestAccount with mismatched genesis hash is rejected
- **WHEN** a browser dApp calls `requestAccount` with a `networkGenesisHash` that does not match the session-bound network
- **THEN** the wallet returns an error
- **AND** does not return an account address for the mismatched network

#### Scenario: Account requests remain scoped to the approved pairing
- **WHEN** one browser pairing has been approved
- **AND** another browser has not been approved
- **THEN** only the approved pairing can successfully call `requestAccount` using its session token

## ADDED Requirements

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
