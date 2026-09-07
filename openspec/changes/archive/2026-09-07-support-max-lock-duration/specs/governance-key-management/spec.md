## ADDED Requirements

### Requirement: Governance key inspection includes token-parameters authority
Governance key listing and signer selection SHALL derive the P11 token-parameters capability from the live level-2 authorization structure.

#### Scenario: Authorized key lists token-parameters capability
- **WHEN** a stored level-2 governance key is included in the live token-parameters access structure
- **THEN** `governance keys list` includes a concise token-parameters capability label for that key

#### Scenario: Maximum lock duration signer selection prioritizes authorized keys
- **WHEN** the CLI prompts for signers for a Max Lock Duration update
- **THEN** locally stored keys authorized by the token-parameters access structure are sorted first
- **AND** authorized keys up to the token-parameters threshold are preselected

#### Scenario: Blind signing accepts token-parameters family hint
- **WHEN** the user supplies a documented token-parameters or maximum-lock-duration `--sign-as` alias for a blind update
- **THEN** the CLI resolves it to the token-parameters level-2 authorization family
- **AND** uses the maximum-lock-duration queue as that family's default sequence queue
