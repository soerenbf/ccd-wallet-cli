## MODIFIED Requirements

### Requirement: Config file schema stability
The config file SHALL include a top-level `version` field set to `1` to support future schema migrations.

Network entries SHALL now include `wallet_proxy` in addition to `node_endpoint` and `genesis_hash`. Network names remain the keys inside the top-level `networks` object, and renaming a network SHALL move the stored entry to a new key without changing the entry's `node_endpoint`, `genesis_hash`, or `wallet_proxy` values.

#### Scenario: Inspect a saved config file
- **WHEN** a user opens `config.json` after at least one network has been added
- **THEN** the file contains `"version": 1` at the top level
- **AND** the file contains a `"networks"` object keyed by network name
- **AND** each network entry used for identity issuance contains `node_endpoint`, `genesis_hash`, and `wallet_proxy`

#### Scenario: Renaming network preserves stored entry data
- **WHEN** the user renames a configured network from `testnet` to `staging`
- **THEN** the `networks` object key changes from `testnet` to `staging`
- **AND** the stored `node_endpoint`, `genesis_hash`, and `wallet_proxy` values remain unchanged
