## ADDED Requirements

### Requirement: Governance keys can be selected explicitly for governance update signing
Stored governance keys SHALL be reusable for governance update signer selection flows through explicit verify-key selection and interactive fuzzy multiselect prompts.

#### Scenario: Governance update accepts explicit verify-key selection
- **WHEN** the user runs `ccd-wallet governance update ... --key <VERIFY_KEY>`
- **THEN** the CLI treats the selected governance key as a requested signer for the update flow

#### Scenario: Governance update signer prompt reuses governance key rows
- **WHEN** the CLI prompts for governance update signers interactively
- **THEN** it displays governance keys using the same tag-first authorization-aware row style used by `governance keys list`
- **AND** abbreviates displayed verify keys by default

#### Scenario: Governance update signer prompt supports fuzzy multiselect
- **WHEN** the CLI prompts for governance update signers interactively
- **THEN** the prompt supports fuzzy filtering and multiple selected governance keys
