## MODIFIED Requirements

### Requirement: CLI invoke defaults to synthetic zero-account context
When `contract invoke` is run without `--invoker`, the CLI SHALL omit the invoker from the contract context so the node uses its synthetic zero-account invocation context. The CLI SHALL NOT require a wallet account or account address for this default path.

If `--invoker` is supplied, the CLI SHALL accept either a raw account address or a finalized local account label on the resolved network and SHALL use the resolved account address as the invocation sender context.

#### Scenario: Invoke omits invoker by default
- **WHEN** the user runs `ccd-wallet contract invoke --contract 42,0 --receive counter.view`
- **THEN** the CLI constructs the invocation without an explicit invoker
- **AND** does not require account selection or account unlocking

#### Scenario: Invoke accepts explicit raw-address invoker
- **WHEN** the user runs `ccd-wallet contract invoke --contract 42,0 --receive counter.view --invoker <account-address>`
- **THEN** the CLI constructs the invocation with the supplied account address as invoker

#### Scenario: Invoke accepts explicit local-label invoker
- **WHEN** the user runs `ccd-wallet contract invoke --contract 42,0 --receive counter.view --invoker <local-account-label>`
- **AND** that label matches a finalized local account on the resolved network
- **THEN** the CLI resolves that local account label to its account address
- **AND** constructs the invocation with the resolved account address as invoker
