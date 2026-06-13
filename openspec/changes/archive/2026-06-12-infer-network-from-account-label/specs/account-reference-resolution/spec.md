## MODIFIED Requirements

### Requirement: Explicit account references resolve raw addresses or finalized local account labels
Commands that adopt account-reference resolution SHALL accept either a raw Concordium account address or a finalized local account label for each supported non-signing account input. Resolution SHALL first attempt raw account-address parsing. For raw addresses, commands SHALL use the already resolved network context or existing network resolution rules. Read-only inputs such as `contract invoke --invoker` SHALL remain account references and SHALL NOT require signing keys.

For local account labels, explicit network inputs SHALL be hard constraints. If `--network` or a compatible node override supplies a concrete network, label lookup SHALL be constrained to that network and SHALL NOT choose an account from another network.

When no explicit network was supplied and the command runs interactively, the CLI SHALL use account-assisted network resolution. The active network, when set, SHALL be a soft default: if the active network has an eligible matching account, the CLI SHALL prefer that match. If the active network does not have a matching account and exactly one eligible match exists across configured networks, the CLI SHALL infer that account's network. If multiple eligible matches remain and the command can prompt, the CLI SHALL disambiguate with an account selector that includes network and ownership metadata.

In non-interactive mode, commands SHALL NOT infer a network solely from account-label uniqueness across the current wallet and SHALL NOT let an account label override the active network. Non-interactive commands SHALL use existing explicit/default network resolution rules and SHALL fail rather than guessing when those rules do not produce a concrete network.

#### Scenario: Explicit raw account address is used directly
- **WHEN** a supported command receives an explicit value that parses as a valid Concordium account address
- **THEN** the CLI uses that address directly
- **AND** does not perform local label lookup for that value

#### Scenario: Contract invoke invoker accepts raw address without keys
- **WHEN** the user runs `ccd-wallet contract invoke --invoker <ADDRESS>`
- **AND** `<ADDRESS>` is a raw Concordium account address
- **THEN** the CLI uses that address as the read-only invocation context
- **AND** does not require the address to be stored locally
- **AND** does not prompt for seed, Ledger, or imported-account signing material

#### Scenario: Explicit network constrains local account label lookup
- **WHEN** a supported command receives explicit local account label `alice`
- **AND** `--network testnet` was supplied or the command otherwise has an explicit network context
- **AND** `alice` matches a finalized local account on another network but not on `testnet`
- **THEN** the CLI exits with an actionable error for the selected network
- **AND** does not infer or switch to the other network

#### Scenario: Explicit local account label resolves within explicit network context
- **WHEN** a supported command receives explicit local account label `alice`
- **AND** `--network testnet` was supplied or the command otherwise has an explicit network context
- **AND** `alice` matches a finalized local account on `testnet`
- **THEN** the CLI resolves the corresponding local account on `testnet`
- **AND** decrypts its stored account-address payload through the owning seed, Ledger key source, or imported-account vault as needed

#### Scenario: Active network chooses among ambiguous local account labels
- **WHEN** a supported interactive command receives explicit local account label `alice`
- **AND** no explicit network was supplied
- **AND** the active network is `testnet`
- **AND** finalized local accounts named `alice` exist on both `testnet` and another configured network
- **THEN** the CLI selects the `testnet` account
- **AND** does not prompt for network selection
- **AND** displays the resolved network and account context

#### Scenario: Interactive unique local account label outside active network infers network
- **WHEN** a supported interactive command receives explicit local account label `alice`
- **AND** no explicit network was supplied
- **AND** the active network does not have a finalized local account named `alice`
- **AND** exactly one finalized local account named `alice` exists on another configured network
- **THEN** the CLI infers the network from that account's stored network genesis hash
- **AND** does not prompt the user to select a network
- **AND** continues with the resolved local account

#### Scenario: Interactive ambiguous local account label opens account selector
- **WHEN** a supported interactive command receives explicit local account label `alice`
- **AND** no explicit network was supplied
- **AND** no active-network match resolves the label
- **AND** finalized local accounts named `alice` exist on multiple configured networks
- **THEN** the CLI opens an account selector over the matching accounts
- **AND** each selector row shows enough network and ownership metadata to disambiguate the accounts
- **AND** the selected account determines the network context

#### Scenario: Non-interactive unique local account label does not infer network
- **WHEN** a supported non-interactive command receives explicit local account label `alice`
- **AND** no explicit or otherwise deterministic network context is available under existing non-interactive network rules
- **AND** exactly one finalized local account named `alice` exists across configured networks
- **THEN** the CLI exits with an actionable error requiring network resolution
- **AND** does not infer the network from the current uniqueness of `alice`

#### Scenario: Non-interactive account label does not override active network
- **WHEN** a supported non-interactive command receives explicit local account label `alice`
- **AND** the active network is `testnet`
- **AND** `alice` exists only on another configured network
- **THEN** the CLI exits with an actionable error for the active network
- **AND** does not infer or switch to the other network

#### Scenario: Explicit local account label is missing or not finalized
- **WHEN** a supported command receives an explicit value that is not a valid raw account address
- **AND** that value does not match any finalized local account label in the applicable network or account-selection scope
- **THEN** the CLI exits with an actionable error
- **AND** does not submit the command
