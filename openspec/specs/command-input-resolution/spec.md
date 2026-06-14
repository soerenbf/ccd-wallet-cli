# command-input-resolution Specification

## Purpose
TBD - created by archiving change streamline-command-input-resolution. Update Purpose after archive.
## Requirements
### Requirement: Commands use prepared input semantics for missing values
Refactored command implementations SHALL convert clap-parsed arguments into prepared command inputs before execution whenever an argument can be absent. Prepared command inputs SHALL distinguish promptable required values, defaultable values, and genuinely optional values.

#### Scenario: Promptable missing value is explicit in prepared input
- **WHEN** a refactored command has a required value that may be omitted interactively
- **THEN** its prepared input represents that value as promptable
- **AND** execution obtains the value only by resolving it with an explicit prompt provider or equivalent domain resolver

#### Scenario: Stateful required input prompts with current context
- **WHEN** a refactored stateful command omits a required mutation input
- **AND** current chain or wallet state exists for that input
- **THEN** its prepared input represents the field as promptable
- **AND** the prompt displays the current value as context
- **AND** the prompt may use the current value as an interactive default when defaults are allowed

#### Scenario: Optional missing value remains optional
- **WHEN** a refactored command has a genuinely optional value
- **THEN** its prepared input preserves absence as an optional value
- **AND** execution does not treat absence as a prompt or default request

### Requirement: Input mode centralizes prompt and default policy
Refactored command implementations SHALL derive prompt and default behavior from a shared input mode instead of passing independent raw booleans through command-specific helper stacks. The shared input mode SHALL prevent silent default filling in non-interactive mode.

#### Scenario: Non-interactive promptable value errors
- **WHEN** a refactored command resolves a missing promptable value
- **AND** the shared input mode is non-interactive
- **THEN** resolution fails with an actionable error
- **AND** no prompt is shown

#### Scenario: Non-interactive defaultable value does not use active default
- **WHEN** a refactored command resolves a missing defaultable value
- **AND** the shared input mode is non-interactive
- **THEN** resolution does not silently use an active default
- **AND** the command requires an explicit value or returns an actionable error according to the command's existing semantics

#### Scenario: No-defaults disables silent defaults while preserving prompts
- **WHEN** a refactored interactive command resolves a missing defaultable value
- **AND** the shared input mode has defaults disabled
- **THEN** resolution does not silently use an active default
- **AND** the command may still prompt or select interactively according to the command's existing semantics

### Requirement: Commands use shared common argument groups
Refactored clap command structs SHALL use shared argument groups for common flags where doing so preserves the public command surface and command-specific conflict rules.

#### Scenario: Shared input mode flags preserve public spelling
- **WHEN** a refactored command supports input mode flags
- **THEN** the command continues to expose `--non-interactive` and `--no-defaults` with their existing meanings
- **AND** the implementation receives those flags through the shared input mode argument group

#### Scenario: Shared network flags preserve public spelling
- **WHEN** a refactored command supports network or node selection
- **THEN** the command continues to expose `--network` and `--node` with their existing meanings
- **AND** the implementation receives those flags through the shared network/node argument group unless the command requires a specialized shape

#### Scenario: Shared submission flag preserves public spelling
- **WHEN** a refactored command supports returning before finalization
- **THEN** the command continues to expose `--no-wait` with its existing meaning
- **AND** the implementation maps it to a shared finalization policy

### Requirement: Refactored commands parse domain values early
Refactored command inputs SHALL use Concordium SDK/domain types or small CLI domain newtypes where the command syntax has an unambiguous domain meaning. Signing-account inputs SHALL use local-account label types, while read-only account-reference inputs SHALL use a type that can represent either a raw account address or a local account label.

#### Scenario: Signing sender input rejects raw address as a label
- **WHEN** a refactored signing command parses its sender or account input
- **THEN** the prepared input represents it as a local account label
- **AND** a raw account address is not accepted as a signing sender value

#### Scenario: Read-only account reference preserves address or label
- **WHEN** a refactored read-only command parses an account reference
- **THEN** the prepared input can represent either a Concordium account address or a local account label
- **AND** later resolution applies the command's existing account-reference rules

#### Scenario: CCD amount input uses a domain wrapper
- **WHEN** a refactored command accepts a user-facing CCD amount
- **THEN** the input is parsed into a domain amount wrapper before command execution
- **AND** invalid decimal CCD syntax is rejected before transaction construction

### Requirement: Async defaults and prompts are resolved explicitly
Prepared inputs that need node queries, wallet database access, or async candidate loading SHALL resolve through explicit async resolver methods or command-specific domain resolvers. Prepared input wrappers SHALL not require stored boxed prompt futures for normal usage.

#### Scenario: Async prompt provider is supplied at resolution
- **WHEN** a refactored command needs to prompt from candidates loaded asynchronously
- **THEN** the command resolver supplies an async prompt or selection provider at the point of resolution
- **AND** the dependency order remains visible in the command resolver

#### Scenario: Network before account dependency is explicit
- **WHEN** a refactored signing command must resolve network context before resolving a signing account
- **THEN** its resolver obtains the network context first
- **AND** resolves the signing account using that context rather than hiding the dependency inside clap parsing

