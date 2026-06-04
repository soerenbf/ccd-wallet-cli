# account-reference-resolution Specification

## Purpose
Defines shared account-reference resolution behavior for non-sender command inputs that may refer to either raw on-chain account addresses or finalized locally stored wallet accounts.

## Requirements

### Requirement: Explicit account references resolve raw addresses or finalized local account labels
Commands that adopt account-reference resolution SHALL accept either a raw Concordium account address or a finalized local account label for each supported non-sender account input. Resolution SHALL first attempt raw account-address parsing and SHALL fall back to finalized local label lookup within the already resolved network context.

#### Scenario: Explicit raw account address is used directly
- **WHEN** a supported command receives an explicit value that parses as a valid Concordium account address
- **THEN** the CLI uses that address directly
- **AND** does not perform local label lookup for that value

#### Scenario: Explicit local account label resolves within network context
- **WHEN** a supported command receives an explicit value that is not a valid raw account address
- **AND** that value matches a finalized local account label on the resolved network
- **THEN** the CLI resolves the corresponding local account
- **AND** decrypts its stored account-address payload through the owning seed or imported-account vault

#### Scenario: Explicit local account label is missing or not finalized
- **WHEN** a supported command receives an explicit value that is not a valid raw account address
- **AND** that value does not match any finalized local account label on the resolved network
- **THEN** the CLI exits with an actionable error
- **AND** does not submit the command

### Requirement: Interactive account-reference prompts support local-account autocomplete and raw-address entry
When a supported command omits a required account-reference value in interactive mode, the CLI SHALL prompt with a `cliclack` text input that accepts pasted raw account addresses and offers autocomplete suggestions for finalized local accounts on the resolved network.

#### Scenario: Interactive prompt shows ownership-decorated local-account suggestions
- **WHEN** a supported interactive command prompts for a missing account reference
- **THEN** the CLI offers autocomplete suggestions for finalized local accounts on the resolved network
- **AND** renders derived accounts as `[<seed-label>] <account-label>`
- **AND** renders imported accounts as `[imported] <account-label>`

#### Scenario: Interactive prompt accepts pasted raw account address
- **WHEN** a user pastes a valid raw Concordium account address into an interactive account-reference prompt
- **THEN** the CLI accepts that value as the resolved account reference
- **AND** does not require the address to match a local account label

#### Scenario: Interactive prompt accepts typed local account label
- **WHEN** a user types a finalized local account label into an interactive account-reference prompt
- **THEN** the CLI resolves that local account within the resolved network context
- **AND** continues the command with the decrypted account address

### Requirement: Account-reference resolution reuses unlocked ownership domains within a command
When multiple account references in the same command require decrypting local account payloads, the CLI SHALL reuse already-unlocked ownership domains for later resolutions in that command.

#### Scenario: Same derived seed is not prompted twice
- **WHEN** a command has already unlocked the seed that owns a derived local account used in a later account-reference resolution
- **THEN** the CLI reuses the already-unlocked seed domain for that later resolution
- **AND** does not prompt again for that seed password

#### Scenario: Same imported-account vault is not prompted twice
- **WHEN** a command has already unlocked the imported-account vault that owns a later local account reference on the same network
- **THEN** the CLI reuses the already-unlocked imported-account domain for that later resolution
- **AND** does not prompt again for that imported-account vault password
