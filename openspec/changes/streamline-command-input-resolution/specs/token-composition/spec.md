## ADDED Requirements

### Requirement: Token compose REPL uses shared input semantics
The token compose REPL SHALL participate in the shared command-input model where practical. REPL operation parsing SHALL distinguish promptable required values, optional values, and plan-specific symbolic values before validating and saving operations.

#### Scenario: Missing REPL operation field resolves through promptable input
- **WHEN** a user enters a token compose REPL operation without a required non-secret field
- **THEN** the REPL represents that field as promptable input
- **AND** resolves it through the same `cliclack` prompt or selector behavior required by the token composition flow

#### Scenario: Optional REPL operation field remains optional
- **WHEN** a user enters a token compose REPL operation without a genuinely optional field such as metadata checksum
- **THEN** the REPL does not prompt for that field
- **AND** the saved operation records the field as absent according to the existing plan format

#### Scenario: Plan-specific symbols parse as unresolved-domain values
- **WHEN** a user enters a token compose REPL value such as `@sender`, `@`, or `@2`
- **THEN** the REPL parses the value as a plan-specific unresolved-domain value
- **AND** does not force it into a final chain address or lock id until the existing plan validation or submission phase requires that resolution

### Requirement: Token compose submit shares prepared submit input
Both `ccd-wallet token compose submit <PLAN>` and the REPL `submit` command SHALL convert their arguments into a shared prepared submit input before resolving sender, network/node context, input mode, and finalization behavior.

#### Scenario: CLI compose submit and REPL submit share resolution
- **WHEN** a user submits a token composition plan through the CLI subcommand or through the REPL `submit` command
- **THEN** both paths use the same prepared submit input and resolver for sender, network/node context, non-interactive mode, default behavior, and finalization policy
- **AND** both paths preserve the existing public flags and command semantics

#### Scenario: Compose submit preserves plan network inference
- **WHEN** token compose submit is missing explicit network and node arguments
- **AND** the plan contains a network genesis hash that maps to a configured network
- **THEN** the shared prepared submit resolver can use the plan-derived network inference according to existing token composition semantics
- **AND** still validates that the resolved network matches the plan genesis hash before submitting
