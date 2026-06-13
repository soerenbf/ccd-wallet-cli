## ADDED Requirements

### Requirement: Sender account resolution requires local signing-capable accounts
Commands that submit signed transactions SHALL resolve sender inputs, including explicit `--sender` options and sender aliases such as `--account`, as local signing-capable accounts rather than generic account references. A sender value SHALL identify a finalized local account record whose source can provide signatures through seed, Ledger, or imported-account signing material. Raw Concordium account addresses SHALL NOT be accepted as transaction senders because they do not provide local signing authority.

Explicit network inputs SHALL be hard constraints for sender lookup. When a sender label is supplied interactively without an explicit network, the CLI SHALL apply account-assisted network resolution. The active network, when set, SHALL be a soft default: if the active network has an eligible matching sender account, the CLI SHALL prefer that match; if it does not and exactly one eligible match exists across configured networks, the CLI SHALL infer that account's network; if multiple eligible matches remain, prompting-capable commands SHALL disambiguate with an account selector. In non-interactive mode, sender resolution SHALL NOT infer a network solely from account-label uniqueness and SHALL NOT let a sender label override the active network.

#### Scenario: Sender label resolves to local signing account
- **WHEN** a transaction-submitting command receives `--sender alice`
- **AND** `alice` resolves to a finalized local account in the selected or inferred network context
- **THEN** the CLI uses that local account as the transaction sender
- **AND** resolves signing material according to the account source kind

#### Scenario: Explicit network constrains sender lookup
- **WHEN** a transaction-submitting command receives `--network testnet --sender alice`
- **AND** `alice` exists as a finalized local account on another network but not on `testnet`
- **THEN** the CLI exits with an actionable error for `testnet`
- **AND** does not infer or switch to the other network

#### Scenario: Raw sender address is rejected
- **WHEN** a transaction-submitting command receives `--sender 4UC8o4m8AgTxt5VBFMdLwMCwwJQVJwjesNzW7RPXkACynrULmd`
- **THEN** the CLI exits with an actionable error explaining that transaction senders must be local account labels
- **AND** no transaction is submitted

#### Scenario: Active network chooses among ambiguous sender labels
- **WHEN** a transaction-submitting interactive command receives `--sender alice`
- **AND** no explicit network was supplied
- **AND** the active network is `testnet`
- **AND** finalized local accounts named `alice` exist on both `testnet` and another configured network
- **THEN** the CLI selects the `testnet` sender account
- **AND** does not prompt for network selection
- **AND** displays the resolved network and sender account context

#### Scenario: Interactive unique sender label outside active network infers network
- **WHEN** a transaction-submitting interactive command receives `--sender alice`
- **AND** no explicit network was supplied
- **AND** the active network does not have a finalized local account named `alice`
- **AND** exactly one finalized local account named `alice` exists on another configured network
- **THEN** the CLI infers the network from that account
- **AND** does not prompt for network selection
- **AND** displays the resolved network and sender account context

#### Scenario: Non-interactive unique sender label does not infer network
- **WHEN** a transaction-submitting non-interactive command receives `--sender alice`
- **AND** no explicit or otherwise supported deterministic network context is available
- **AND** exactly one finalized local account named `alice` exists across configured networks
- **THEN** the CLI exits with an actionable network-resolution error
- **AND** does not infer the network from `alice`

#### Scenario: Non-interactive sender label does not override active network
- **WHEN** a transaction-submitting non-interactive command receives `--sender alice`
- **AND** the active network is `testnet`
- **AND** `alice` exists only on another configured network
- **THEN** the CLI exits with an actionable error for `testnet`
- **AND** does not infer or switch to the other network
