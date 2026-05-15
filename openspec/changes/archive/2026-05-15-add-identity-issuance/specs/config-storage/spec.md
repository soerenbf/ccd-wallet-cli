## MODIFIED Requirements

### Requirement: Config file schema stability
The config file SHALL include a top-level `version` field set to `1` to support future schema migrations.

Network entries SHALL now include `wallet_proxy` in addition to `node_endpoint` and `genesis_hash`.

#### Scenario: Inspect a saved config file
- **WHEN** a user opens `config.json` after at least one network has been added
- **THEN** the file contains `"version": 1` at the top level
- **AND** the file contains a `"networks"` object keyed by network name
- **AND** each network entry used for identity issuance contains `node_endpoint`, `genesis_hash`, and `wallet_proxy`
