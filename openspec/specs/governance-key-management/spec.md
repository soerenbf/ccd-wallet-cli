# governance-key-management Specification

## Purpose
TBD - created by archiving change add-governance-key-vault-and-management. Update Purpose after archive.
## Requirements
### Requirement: Governance keys can be imported from single keypair files and directories
The CLI SHALL provide `governance keys import <file>` for importing one governance keypair JSON file and `governance keys import --dir <dir>` for importing multiple governance keypair JSON files from a directory. Aggregate governance snapshot files such as `governance-keys.json` SHALL NOT be imported as governance key material.

#### Scenario: Import a single governance key file
- **WHEN** the user runs `ccd-wallet governance keys import root-key-0.json`
- **THEN** the CLI imports that governance keypair JSON file into the selected network's governance key vault

#### Scenario: Directory import ignores aggregate governance snapshot file
- **WHEN** the user runs `ccd-wallet governance keys import --dir update-keys/`
- **AND** the directory contains `governance-keys.json` and individual `root-key-*.json` files
- **THEN** the CLI ignores `governance-keys.json`
- **AND** imports only recognized governance keypair files

#### Scenario: Malformed governance key file fails actionably
- **WHEN** the user imports a file that is not a valid governance keypair JSON payload
- **THEN** the CLI exits with an actionable parse error
- **AND** does not store that malformed key payload

### Requirement: Governance keys are listed only after vault unlock and live chain query
The CLI SHALL provide `governance keys list` as an unlock-and-query flow rather than a plaintext metadata listing. The command SHALL resolve its network using the active network by default when one exists, while still allowing explicit network selection and prompted fallback. After unlocking the governance key vault, the CLI SHALL decrypt stored key payloads, query live chain parameters, and derive the currently authorized level and authorization status for each imported key.

#### Scenario: Listing requires governance vault password
- **WHEN** the user runs `ccd-wallet governance keys list`
- **AND** a governance vault exists for the resolved network
- **THEN** the CLI prompts for the governance vault password before showing imported governance keys

#### Scenario: Listing without a configured governance vault fails before password prompt
- **WHEN** the user runs `ccd-wallet governance keys list`
- **AND** no governance vault exists for the resolved network
- **THEN** the CLI exits with an actionable no-keys-stored message
- **AND** does not prompt for a governance vault password

#### Scenario: Listing uses active network by default
- **WHEN** the user runs `ccd-wallet governance keys list`
- **AND** an active network is configured
- **THEN** the CLI uses the active network as the default target network unless an explicit network selector overrides it

#### Scenario: Listing derives levels from live chain state
- **WHEN** the user runs `ccd-wallet governance keys list`
- **AND** a stored key's `verifyKey` appears in the live chain's root governance keys
- **THEN** the CLI renders that key as a root governance key
- **AND** does not rely on locally stored level metadata

#### Scenario: Listing shows stored-but-not-authorized key
- **WHEN** the user runs `ccd-wallet governance keys list`
- **AND** a stored governance key does not appear in the live chain's current governance authorization state
- **THEN** the CLI includes the key in the output
- **AND** marks it as not currently authorized

#### Scenario: Listing uses tag-first rows and operator-oriented sorting
- **WHEN** the user runs `ccd-wallet governance keys list`
- **AND** imported governance keys resolve to `level 2`, `level 1`, `root`, and not-authorized states
- **THEN** each output row begins with a bracketed authorization tag followed directly by the displayed verify key
- **AND** the rows are sorted as `level 2`, then `level 1`, then `root`, then `not authorized`

#### Scenario: Listing abbreviates verify keys by default
- **WHEN** the user runs `ccd-wallet governance keys list`
- **THEN** the CLI abbreviates each displayed verify key to a compact form such as `1234...5678`

#### Scenario: Listing can show full verify keys explicitly
- **WHEN** the user runs `ccd-wallet governance keys list --show-full`
- **THEN** the CLI renders the full verify key for each listed governance key

#### Scenario: Listing summarizes current governance capabilities
- **WHEN** the user runs `ccd-wallet governance keys list`
- **AND** an imported key is currently authorized as `level 2`
- **THEN** the CLI appends a concise comma-separated summary of the update families that key can currently sign
- **AND** root and level 1 keys render governance-key update summaries appropriate to their authorization level

### Requirement: Governance keys can be removed individually or all at once
The CLI SHALL provide `governance keys remove <verify-key>` to remove one imported governance key, interactive `governance keys remove` selection after vault unlock, and `governance keys remove --all` to remove all imported governance keys for the selected network. If `--all` removes the last governance key for a network, the governance vault for that network SHALL also be removed.

#### Scenario: Remove one governance key by public key
- **WHEN** the user runs `ccd-wallet governance keys remove <verify-key>`
- **THEN** the CLI removes the matching imported governance key payload from the selected network's governance vault

#### Scenario: Interactive remove selects governance keys after unlock
- **WHEN** the user runs `ccd-wallet governance keys remove` in interactive mode without a verify key
- **THEN** the CLI unlocks the governance vault for the selected network
- **AND** queries live chain parameters for current governance authorization state
- **AND** presents a fuzzy multiselect picker over stored governance keys
- **AND** shows the same authorization tags and summaries used by `governance keys list`
- **AND** abbreviates displayed verify keys to a compact form such as `1234...5678`
- **AND** removes the selected governance key payloads

#### Scenario: Remove all governance keys deletes empty governance vault
- **WHEN** the user runs `ccd-wallet governance keys remove --all`
- **AND** all governance keys for the selected network are removed
- **THEN** the CLI deletes the governance key vault for that network if it becomes empty
