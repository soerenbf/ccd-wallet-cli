## ADDED Requirements

### Requirement: Account-reference prompts use cliclack autocomplete with raw-address fallback
Supported prompt-first command flows that resolve non-sender account references SHALL use a `cliclack` text input prompt with autocomplete suggestions for finalized local accounts while still accepting pasted raw account addresses.

#### Scenario: Prompted token recipient offers local-account autocomplete suggestions
- **WHEN** an interactive token command prompts for a missing recipient, source, or target account reference
- **THEN** the CLI uses a `cliclack` input prompt with autocomplete suggestions sourced from finalized local accounts on the resolved network
- **AND** the prompt still accepts pasted raw account addresses

#### Scenario: Prompt suggestions show account ownership context
- **WHEN** an interactive account-reference prompt renders suggestions for local accounts
- **THEN** each derived account suggestion shows its seed ownership in bracketed form
- **AND** each imported account suggestion shows `[imported]` before the account label
