## MODIFIED Requirements

### Requirement: Config file schema stability
The config file SHALL include a top-level `version` field set to `1` to support future schema migrations.

Network entries SHALL include `wallet_proxy` in addition to `node_endpoint` and `genesis_hash`. Network names remain the keys inside the top-level `networks` object. The config layer SHALL support deleting one or more aliases by network name and listing aliases that reference a given genesis hash.

#### Scenario: Delete one alias by name preserves other aliases
- **WHEN** configured aliases `testnet-a` and `testnet-b` both reference genesis hash `abc`
- **AND** the config layer deletes alias `testnet-a`
- **THEN** the `testnet-a` key is removed from the `networks` object
- **AND** the `testnet-b` key remains present

#### Scenario: List aliases by genesis hash
- **WHEN** configured aliases `testnet-a` and `testnet-b` both reference genesis hash `abc`
- **THEN** the config layer can return both alias names for genesis hash `abc`
