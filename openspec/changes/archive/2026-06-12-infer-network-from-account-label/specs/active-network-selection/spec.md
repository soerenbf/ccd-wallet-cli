## ADDED Requirements

### Requirement: Active network is a soft default for interactive account selection
When an interactive account-consuming command has no explicit network constraint, the active network SHALL act as a soft default rather than a hard filter. The CLI SHALL use the active network to filter eligible account selectors when the user has not supplied an account label. When the user supplies an account label that is ambiguous across configured networks, the CLI SHALL prefer an eligible active-network match. When the active network has no eligible matching account and the supplied label uniquely identifies an eligible account on another configured network, the CLI SHALL infer the network from that account.

#### Scenario: Missing account selector is scoped by active network
- **WHEN** an interactive command needs the user to choose a local account
- **AND** no account label was supplied
- **AND** no explicit network was supplied
- **AND** an active network is configured
- **THEN** the CLI uses the active network as the account selector scope

#### Scenario: Active network resolves ambiguous account label
- **WHEN** an interactive command receives account label `alice`
- **AND** no explicit network was supplied
- **AND** the active network is `testnet`
- **AND** matching eligible accounts named `alice` exist on `testnet` and another configured network
- **THEN** the CLI selects the `testnet` account
- **AND** displays the resolved network and account context

#### Scenario: Unique account label outside active network can infer network interactively
- **WHEN** an interactive command receives account label `alice`
- **AND** no explicit network was supplied
- **AND** the active network has no eligible account named `alice`
- **AND** exactly one eligible account named `alice` exists on another configured network
- **THEN** the CLI infers the network from that account
- **AND** displays the resolved network and account context

### Requirement: Interactive single-network fallback is silent but visible
When an interactive command needs a network and no explicit network, account-derived network, or active-network default is available, the CLI SHALL automatically select the only configured network if exactly one network is configured. The CLI SHALL NOT render a one-item network selector for this case, but it SHALL treat the result as a visible inferred/defaulted context value so the command output or context header identifies the selected network.

#### Scenario: Single configured network is selected without prompt
- **WHEN** an interactive command needs a network
- **AND** no `--network` argument was supplied
- **AND** no account label or active network is available for that command's network resolution path
- **AND** exactly one network is configured
- **THEN** the CLI selects that network automatically
- **AND** does not render a network selector
- **AND** displays the selected network in the resolved context header

#### Scenario: Multiple configured networks still require selection
- **WHEN** an interactive command needs a network
- **AND** no `--network` argument was supplied
- **AND** no account label or active network is available for that command's network resolution path
- **AND** more than one network is configured
- **THEN** the CLI prompts the user to select a network

### Requirement: Non-interactive network resolution remains deterministic
Non-interactive commands SHALL NOT use account-label uniqueness or a one-network selector shortcut as an implicit substitute for required network input unless that behavior is already part of the command's explicit non-interactive network rules. Non-interactive commands also SHALL NOT let a supplied account label override an active network. When no explicit or otherwise supported deterministic network is available, the command SHALL fail with an actionable error.

#### Scenario: Non-interactive command does not infer network from account uniqueness
- **WHEN** a non-interactive account-consuming command receives a local account label without `--network`
- **AND** the command's existing non-interactive network rules do not provide a concrete network
- **THEN** the CLI exits with an actionable error requiring network resolution
- **AND** does not infer the network from local account-label uniqueness

#### Scenario: Non-interactive command does not override active network from account label
- **WHEN** a non-interactive account-consuming command receives local account label `alice` without `--network`
- **AND** the active network is `testnet`
- **AND** `alice` exists only on another configured network
- **THEN** the CLI exits with an actionable error for `testnet`
- **AND** does not infer or switch to the other network
