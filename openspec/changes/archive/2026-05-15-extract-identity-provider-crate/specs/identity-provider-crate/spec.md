## ADDED Requirements

### Requirement: Identity provider code is owned by a dedicated crate
The workspace SHALL provide identity provider issuance functionality from a dedicated crate named `ccd-wallet-identity-provider`.

#### Scenario: Workspace includes identity provider crate
- **WHEN** the workspace is built
- **THEN** Cargo includes `crates/ccd-wallet-identity-provider` as a workspace member

#### Scenario: Core crate no longer exports identity provider module
- **WHEN** consumers use `ccd-wallet-core`
- **THEN** identity provider issuance APIs are not exported from `ccd_wallet_core::identity_provider`

### Requirement: Identity provider crate exposes issuance APIs
The identity provider crate SHALL expose the request construction, HTTP client, and callback session APIs needed by the CLI identity issuance flow.

#### Scenario: CLI constructs identity issuance request through new crate
- **WHEN** the CLI needs to build a v1 identity object request
- **THEN** it imports the request construction API from `ccd-wallet-identity-provider`

#### Scenario: CLI performs provider HTTP operations through new crate
- **WHEN** the CLI needs wallet proxy metadata, issuance start, or code URI polling
- **THEN** it imports HTTP client APIs from `ccd-wallet-identity-provider`

#### Scenario: CLI receives callbacks through new crate
- **WHEN** the CLI prepares a manual or loopback callback session
- **THEN** it imports callback APIs from `ccd-wallet-identity-provider`

### Requirement: Identity provider crate preserves behavior
Moving identity provider code to its own crate SHALL NOT change identity issuance behavior.

#### Scenario: Existing identity issuance tests continue to pass
- **WHEN** the workspace test suite is run
- **THEN** existing identity provider request, client, and callback behavior remains covered and passing
