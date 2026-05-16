## ADDED Requirements

### Requirement: Identity provider client supports recovery requests
The dedicated identity provider client SHALL support Concordium identity recovery by constructing a recovery-start URL from provider metadata and issuing a recovery request whose serialized state contains the generated `idRecoveryRequest`.

#### Scenario: Recovery request returns identity object
- **WHEN** the client sends a recovery request to `<recoveryStart>?state=<json>` for a valid seed-derived identity candidate
- **AND** the provider returns a successful response containing an identity object
- **THEN** the client returns that identity object to the caller

#### Scenario: Recovery request reports missing identity without crashing the flow
- **WHEN** the client sends a recovery request for a candidate identity that does not exist
- **AND** the provider returns a non-success response indicating that no identity was recovered
- **THEN** the client reports a recoverable miss to the caller
- **AND** does not convert that miss into a fatal process error

#### Scenario: Missing recovery start metadata is rejected before request construction
- **WHEN** recovery is requested for a provider that lacks `recoveryStart` metadata
- **THEN** the client returns an actionable error
- **AND** does not attempt to synthesize a recovery URL from unrelated provider fields
