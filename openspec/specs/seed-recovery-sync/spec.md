# seed-recovery-sync Specification

## Purpose
TBD - created by archiving change add-seed-recovery-sync. Update Purpose after archive.
## Requirements
### Requirement: Recovery discovers identities from selected providers
The system SHALL recover identities for a resolved `(seed, network)` scope by fetching identity providers from wallet-proxy, building Concordium identity recovery requests from the seed-derived wallet material, and querying each selected provider's recovery endpoint. Recovery SHALL treat provider discovery as network-scoped and SHALL only consider providers that expose recovery metadata for the chosen network. The recovery engine SHALL support bounded parallel execution across selected providers.

#### Scenario: Recovery finds an identity from a provider
- **WHEN** recovery probes a selected provider for an identity index derived from the chosen seed
- **AND** the provider returns a successful recovery response
- **THEN** the system treats the identity as recovered
- **AND** passes it to local identity import

#### Scenario: Recovery skips provider without recovery metadata
- **WHEN** wallet-proxy returns a provider for the chosen network without usable recovery metadata
- **THEN** the system skips that provider
- **AND** records that the provider was skipped for the final recovery summary

#### Scenario: Recovery continues after provider-specific failure
- **WHEN** one selected provider returns an error or cannot be reached
- **THEN** the system records that provider failure
- **AND** continues attempting recovery against the remaining selected providers when safe to do so

### Requirement: Recovery discovers accounts from recovered identities
For each recovered or already-known identity in the chosen `(seed, network)` scope, the system SHALL derive candidate credential registration IDs from the seed and identity tuple and query the configured node for matching on-chain accounts. When a credential registration ID resolves to an account, the system SHALL treat that account as recovered and pass it to local account import. The recovery engine SHALL support bounded parallel account discovery across multiple recovered identities.

#### Scenario: Recovery finds account by credential registration id
- **WHEN** recovery derives a credential registration ID for a recovered identity
- **AND** the node returns account information for that credential registration ID
- **THEN** the system treats that account as recovered
- **AND** passes it to local account import

#### Scenario: Recovery continues when a credential probe has no account
- **WHEN** recovery probes a candidate credential registration ID
- **AND** the node reports that no account exists for that credential
- **THEN** the system continues the recovery scan
- **AND** does not treat the missing account as a fatal error

### Requirement: Recovery reports truthful progress and final outcomes
The recovery flow SHALL present progress in terms of known phases and live discovery counters rather than a fabricated total percentage over unknown identities or accounts. At minimum, the flow SHALL report provider-level progress, provider worker state, aggregate probe/discovery counts, and any skipped or failed providers. On completion, it SHALL print a summary of what was recovered and what was skipped or failed.

#### Scenario: Interactive recovery shows provider progress and aggregate discovery counters
- **WHEN** the user runs recovery interactively
- **THEN** the CLI displays a determinate provider-level progress indicator
- **AND** shows aggregate worker-state and discovery/probe counters for the running recovery
- **AND** does not claim a total identity or account count that is not actually known

#### Scenario: Recovery summary distinguishes success and partial failure
- **WHEN** recovery finishes after finding some entities and encountering some provider skips or failures
- **THEN** the CLI prints the recovered identity and account totals
- **AND** separately reports which providers were skipped or failed
- **AND** exits successfully if at least part of the recovery completed safely

#### Scenario: Parallel recovery progress includes active work snapshot
- **WHEN** recovery is running with multiple provider or account-discovery workers in flight
- **THEN** the CLI may show a compact snapshot of active work items
- **AND** limits that snapshot to a small readable subset instead of rendering one line for every concurrent worker

