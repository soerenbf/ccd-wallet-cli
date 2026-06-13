## MODIFIED Requirements

### Requirement: Selectors minimize redundant interaction
Supported prompt-first command flows SHALL avoid unnecessary selector interaction when the effective choice is already obvious. When a selector is skipped because there is exactly one valid option, because the active network resolves an ambiguous explicit account label, or because an explicit local account label uniquely determines the account and network outside the active network, the CLI SHALL still display any silently selected contextual values that are relevant to understanding the command target.

#### Scenario: Single-option selector is skipped
- **WHEN** a supported command would present a selector with exactly one valid option
- **THEN** the CLI selects that option automatically
- **AND** does not render a one-item selector

#### Scenario: Single configured network is shown after skipped selector
- **WHEN** a supported interactive command automatically selects the only configured network
- **THEN** the CLI does not render a network selector
- **AND** displays the selected network in a resolved context header

#### Scenario: Active-network account match skips account selector
- **WHEN** a supported interactive account-consuming command receives an explicit local account label
- **AND** no explicit network was supplied
- **AND** the active network has an eligible matching account for that label
- **THEN** the CLI selects the active-network account
- **AND** does not render a network selector
- **AND** displays the resolved network and account context before performing command-specific work

#### Scenario: Unique explicit account label outside active network skips network selector
- **WHEN** a supported interactive account-consuming command receives an explicit local account label
- **AND** no explicit network was supplied
- **AND** the active network has no eligible matching account for that label
- **AND** that label uniquely identifies an eligible local account on another configured network
- **THEN** the CLI selects that account and its network automatically
- **AND** does not render a network selector
- **AND** displays the resolved network and account context before performing command-specific work

#### Scenario: Existing-entity choice uses selector instead of free-text input
- **WHEN** a supported command asks the user to choose from already configured seeds or networks
- **THEN** the CLI uses a `cliclack` selector instead of a free-text label prompt

#### Scenario: Seed delete chooses an existing seed through a selector
- **WHEN** the user runs `ccd-wallet seed delete` without a label
- **THEN** the CLI renders a `cliclack` selector over configured seeds
- **AND** uses the selected seed label for the destructive flow

#### Scenario: Network reset selector renders partition rows with hashes and aliases
- **WHEN** the user runs `ccd-wallet network reset` without a target
- **AND** the wallet has stored network data for genesis hash `abc`
- **AND** configured aliases `testnet` and `staging-testnet` reference `abc`
- **THEN** the CLI renders a partition-oriented row that shows the genesis hash and matching aliases

#### Scenario: Network reset selector can include orphaned hashes
- **WHEN** the user runs `ccd-wallet network reset` without a target
- **AND** the wallet has stored network data for a genesis hash not present in config
- **THEN** the CLI renders a selectable orphaned-hash target labeled as orphaned in addition to configured partitions

## ADDED Requirements

### Requirement: Account disambiguation selectors show network and source metadata
When an interactive command must disambiguate between multiple local accounts with the same label and neither an explicit network nor the active-network soft default resolves the choice, the CLI SHALL present an account selector rather than a network selector. Account selector rows SHALL include the local account label, the configured network name or genesis-hash fallback, and ownership/source metadata sufficient to distinguish derived, Ledger-derived, and imported accounts.

Applicable account-consuming command families SHALL use this shared account-selection behavior where their input semantics fit either account-reference resolution or signing-account/sender resolution, rather than keeping command-specific network-first selectors.

#### Scenario: Ambiguous account label selector shows network and key source
- **WHEN** an interactive command receives local account label `alice`
- **AND** no explicit network was supplied
- **AND** no active-network account match resolves the label
- **AND** matching finalized derived accounts exist on more than one configured network
- **THEN** the CLI opens an account selector over the matching accounts
- **AND** each row shows `alice`, the network, and the owning key-source label

#### Scenario: Ambiguous imported account selector shows imported source
- **WHEN** an interactive command receives local account label `genesis`
- **AND** no explicit network was supplied
- **AND** no active-network account match resolves the label
- **AND** matching finalized imported accounts exist on more than one configured network
- **THEN** the CLI opens an account selector over the matching accounts
- **AND** each imported row is marked as imported rather than displaying a seed key source

### Requirement: Resolved account context headers omit account addresses
When the CLI displays a resolved context header for a silently selected or inferred local account, the header SHALL identify the network, local account label, and source metadata needed to understand the command target. The header SHALL NOT include the account address solely as part of context rendering.

#### Scenario: Derived account header shows source metadata without address
- **WHEN** an interactive command silently resolves local account `alice` owned by key source `main-seed`
- **THEN** the resolved context header shows the selected network
- **AND** shows local account label `alice`
- **AND** shows key source `main-seed`
- **AND** does not include the account address

#### Scenario: Imported account header shows imported source without address
- **WHEN** an interactive command silently resolves imported local account `genesis`
- **THEN** the resolved context header shows the selected network
- **AND** shows local account label `genesis`
- **AND** marks the source as imported
- **AND** does not include the account address
