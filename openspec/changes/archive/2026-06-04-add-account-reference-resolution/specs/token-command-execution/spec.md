## ADDED Requirements

### Requirement: Token commands accept local account references for non-sender account inputs
Token and lock commands SHALL accept finalized local account labels anywhere they currently accept non-sender account-address inputs, while continuing to accept raw account addresses. Each such input SHALL use the shared account-reference resolution behavior in the already resolved network context.

Covered inputs include recipient, target, source, repeated target and recipient lists, and account-reference segments embedded inside lock grant arguments.

#### Scenario: Token transfer accepts recipient local account label
- **WHEN** a user runs `ccd-wallet token transfer` with `--recipient <local-account-label>`
- **AND** that label matches a finalized local account on the resolved network
- **THEN** the CLI resolves the recipient from the local account label
- **AND** submits the transfer using the resolved account address

#### Scenario: Token list update accepts mixed target labels and raw addresses
- **WHEN** a user runs `ccd-wallet token allow-list add` or `ccd-wallet token deny-list add` with repeated `--target` values
- **AND** some values are finalized local account labels and others are raw account addresses
- **THEN** the CLI resolves each value independently through the shared account-reference behavior
- **AND** submits the list update using the resulting account addresses

#### Scenario: Token lock create accepts local labels in recipients and grants
- **WHEN** a user runs `ccd-wallet token lock create` with recipient values or lock grant account references expressed as finalized local account labels
- **THEN** the CLI resolves those labels within the resolved network context
- **AND** submits the lock creation using the resulting account addresses

#### Scenario: Token lock send reuses an already unlocked sender seed for another local account reference
- **WHEN** `ccd-wallet token lock send` has already unlocked the signer account's derived seed
- **AND** a later `--source` or `--recipient` local account label is owned by that same seed
- **THEN** the CLI resolves the later local account reference without prompting again for the same seed password
