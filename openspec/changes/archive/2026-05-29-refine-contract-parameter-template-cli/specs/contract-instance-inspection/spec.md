## MODIFIED Requirements

### Requirement: CLI prints parameter templates from embedded module schemas
The CLI SHALL provide a `ccd-wallet contract parameter-template` command for printing pretty-printed JSON parameter templates derived from embedded module schemas.

The command SHALL support an `init` mode that resolves a parameter schema from a module reference and a positional init name, and a `receive` mode that resolves a parameter schema from either a contract instance address plus a positional fully-qualified receive name or a module reference plus a positional fully-qualified receive name. In `receive` mode, the command SHALL require exactly one of `--contract` or `--module-ref`.

The default output SHALL be pure JSON template content without additional prose so users can redirect it directly into a file. The command SHALL support network or node endpoint selection and SHALL NOT require signer account selection, account unlocking, transaction approval, or transaction submission.

#### Scenario: User prints init parameter template by module reference
- **WHEN** the user runs `ccd-wallet contract parameter-template init init_counter --module-ref <module-ref>`
- **AND** the module source contains a compatible embedded schema for `init_counter`
- **THEN** the CLI prints the JSON parameter template for `init_counter`

#### Scenario: User prints receive parameter template by contract address
- **WHEN** the user runs `ccd-wallet contract parameter-template receive counter.increment --contract 42,0`
- **AND** the instance source module contains a compatible embedded schema for `counter.increment`
- **THEN** the CLI prints the JSON parameter template for `counter.increment`

#### Scenario: User prints receive parameter template by module reference
- **WHEN** the user runs `ccd-wallet contract parameter-template receive counter.increment --module-ref <module-ref>`
- **AND** the module source contains a compatible embedded schema for `counter.increment`
- **THEN** the CLI prints the JSON parameter template for `counter.increment`

#### Scenario: Parameter template rejects conflicting schema sources
- **WHEN** the user runs `ccd-wallet contract parameter-template receive counter.increment --contract 42,0 --module-ref <module-ref>`
- **THEN** the CLI exits with an actionable error
- **AND** does not print a parameter template

#### Scenario: Parameter template fails when embedded schema is unavailable
- **WHEN** the user runs `ccd-wallet contract parameter-template receive counter.increment --contract 42,0`
- **AND** the resolved module source has no compatible embedded schema for `counter.increment`
- **THEN** the CLI exits with an actionable error
- **AND** does not print a parameter template
