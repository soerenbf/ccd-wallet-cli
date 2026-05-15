## ADDED Requirements

### Requirement: Managed entities can be renamed through entity-family commands
The CLI SHALL provide `rename` subcommands for `network`, `seed`, `identity`, and `account`.

Rename operations SHALL change only the user-facing label/name of the selected entity. They SHALL NOT alter the entity's stable underlying identity.

#### Scenario: Rename network
- **WHEN** the user runs `ccd-wallet network rename old_name new_name`
- **THEN** the CLI renames the configured network from `old_name` to `new_name`
- **AND** preserves the stored network data

#### Scenario: Rename seed
- **WHEN** the user runs `ccd-wallet seed rename old_label new_label`
- **THEN** the CLI renames the seed label from `old_label` to `new_label`
- **AND** preserves the seed's stable internal id and encrypted payload

#### Scenario: Rename identity
- **WHEN** the user runs `ccd-wallet identity rename old_label new_label`
- **THEN** the CLI renames the identity label from `old_label` to `new_label`
- **AND** preserves the identity's underlying network/seed/provider/index identity

#### Scenario: Rename account
- **WHEN** the user runs `ccd-wallet account rename old_label new_label`
- **THEN** the CLI renames the account label from `old_label` to `new_label`
- **AND** preserves the account's derivation tuple and encrypted payload

### Requirement: Rename supports interactive source selection when the old label is omitted
If a `rename` command is run without the old label/name in interactive mode, the CLI SHALL allow the user to choose the source entity interactively before prompting for the new label/name.

For identities and accounts, omitting the source SHALL use a global fuzzy selector rather than active/default seed or network scope. The searchable text and displayed row metadata SHALL include enough information to disambiguate matches, including network and seed labels.

The selector rows SHALL be label-first. A status badge SHALL be shown only when the entity is not in its normal happy state.
- For identities, pending and expired identities show a badge, while normal done-and-unexpired identities do not.
- For accounts, pending accounts show a badge, while finalized accounts do not.

#### Scenario: Seed rename prompts for source seed
- **WHEN** the user runs `ccd-wallet seed rename` in interactive mode
- **THEN** the CLI opens a selector over configured seeds
- **AND** then prompts for the new seed label

#### Scenario: Identity rename uses global fuzzy selection
- **WHEN** the user runs `ccd-wallet identity rename` in interactive mode without an old label
- **THEN** the CLI opens a fuzzy selector over stored identities across all seeds and networks
- **AND** the searchable text includes the identity label, network, and seed metadata
- **AND** then prompts for the new identity label

#### Scenario: Searching by network finds matching identities
- **WHEN** the user runs `ccd-wallet identity rename` in interactive mode without an old label
- **AND** types `testnet` into the fuzzy selector
- **THEN** identities on `testnet` appear among the matches even when their labels do not contain `testnet`

#### Scenario: Happy-state identity row has no status badge
- **WHEN** the fuzzy selector renders an identity that is done and not expired
- **THEN** the row does not show a status badge before the label

#### Scenario: Unhappy-state identity row shows a status badge
- **WHEN** the fuzzy selector renders an identity that is pending or expired
- **THEN** the row shows a status badge before the label indicating that state

#### Scenario: Account rename uses global fuzzy selection
- **WHEN** the user runs `ccd-wallet account rename` in interactive mode without an old label
- **THEN** the CLI opens a fuzzy selector over stored accounts across all seeds and networks
- **AND** the searchable text includes the account label, network, and seed metadata
- **AND** then prompts for the new account label

#### Scenario: Account rename with addresses requires explicit or selected seed
- **WHEN** the user runs `ccd-wallet account rename --show-addresses`
- **THEN** the CLI requires a concrete seed scope chosen either through `--seed <LABEL>` or an interactive seed selector
- **AND** only then opens the fuzzy account selector with addresses available for display

#### Scenario: Non-interactive account rename with addresses requires explicit seed
- **WHEN** the user runs `ccd-wallet account rename --show-addresses --non-interactive`
- **AND** no `--seed <LABEL>` is supplied
- **THEN** the CLI exits with an actionable error instead of attempting a global rename search with addresses

#### Scenario: Happy-state account row has no status badge
- **WHEN** the fuzzy selector renders a finalized account
- **THEN** the row does not show a status badge before the label

#### Scenario: Unhappy-state account row shows a status badge
- **WHEN** the fuzzy selector renders a pending account
- **THEN** the row shows a pending status badge before the label

### Requirement: Rename validates collisions and updates active state where applicable
Rename operations SHALL reject invalid new labels/names and SHALL reject collisions in the target scope. If the renamed entity is currently active and the entity family supports active state, the corresponding wallet-state key SHALL be updated to the new name.

#### Scenario: Rename rejects duplicate target name
- **WHEN** the user attempts to rename an entity to a label/name that already exists in the relevant scope
- **THEN** the CLI exits with an actionable error
- **AND** does not modify the existing entity

#### Scenario: Renaming active seed updates active seed state
- **WHEN** the user renames the currently active seed
- **THEN** the CLI updates the active seed state to the new label
- **AND** subsequent commands resolve the renamed seed as active

#### Scenario: Renaming active network updates active network state
- **WHEN** the user renames the currently active network
- **THEN** the CLI updates the active network state to the new name
- **AND** subsequent commands resolve the renamed network as active
