# contract-instance-inspection Specification

## Purpose
TBD - created by archiving change expand-contract-commands. Update Purpose after archive.
## Requirements
### Requirement: CLI shows contract instance information
The CLI SHALL provide a `ccd-wallet contract show` command that queries and displays smart contract instance metadata for a contract instance address.

The output SHALL include the contract address, contract name, owner account, amount held by the instance, source module reference, supported entrypoints, and module version information when available from the node response.

The command SHALL support network or node endpoint selection and SHALL NOT require signer account selection, account unlocking, transaction approval, or transaction submission.

#### Scenario: User shows contract instance info
- **WHEN** the user runs `ccd-wallet contract show --contract 42,0`
- **AND** the node returns instance information for `<42, 0>`
- **THEN** the CLI prints the contract address
- **AND** prints the owner, balance, source module, contract name, and entrypoints

#### Scenario: Contract instance does not exist
- **WHEN** the user runs `ccd-wallet contract show --contract 42,0`
- **AND** the node cannot find the instance at `<42, 0>`
- **THEN** the CLI exits with an actionable error
- **AND** does not prompt for account unlocking

#### Scenario: User requests JSON instance output
- **WHEN** the user runs `ccd-wallet contract show --contract 42,0 --json`
- **AND** the node returns instance information for `<42, 0>`
- **THEN** the CLI prints machine-readable JSON for the instance information

### Requirement: CLI prints parameter templates from embedded module schemas
The CLI SHALL provide a `ccd-wallet contract parameter-template` command for printing pretty-printed JSON parameter templates derived from embedded module schemas.

The command SHALL support an `init` mode that resolves a parameter schema from a module reference and init name, and a `receive` mode that resolves a parameter schema from either a contract instance address plus receive name or a module reference plus receive name. In `receive` mode, the command SHALL require exactly one of `--contract` or `--module-ref`.

The default output SHALL be pure JSON template content without additional prose so users can redirect it directly into a file. The command SHALL support network or node endpoint selection and SHALL NOT require signer account selection, account unlocking, transaction approval, or transaction submission.

#### Scenario: User prints init parameter template by module reference
- **WHEN** the user runs `ccd-wallet contract parameter-template init --module-ref <module-ref> --init-name init_counter`
- **AND** the module source contains a compatible embedded schema for `init_counter`
- **THEN** the CLI prints the JSON parameter template for `init_counter`

#### Scenario: User prints receive parameter template by contract address
- **WHEN** the user runs `ccd-wallet contract parameter-template receive --contract 42,0 --receive counter.increment`
- **AND** the instance source module contains a compatible embedded schema for `counter.increment`
- **THEN** the CLI prints the JSON parameter template for `counter.increment`

#### Scenario: User prints receive parameter template by module reference
- **WHEN** the user runs `ccd-wallet contract parameter-template receive --module-ref <module-ref> --receive counter.increment`
- **AND** the module source contains a compatible embedded schema for `counter.increment`
- **THEN** the CLI prints the JSON parameter template for `counter.increment`

#### Scenario: Parameter template rejects conflicting schema sources
- **WHEN** the user runs `ccd-wallet contract parameter-template receive --contract 42,0 --module-ref <module-ref> --receive counter.increment`
- **THEN** the CLI exits with an actionable error
- **AND** does not print a parameter template

#### Scenario: Parameter template fails when embedded schema is unavailable
- **WHEN** the user runs `ccd-wallet contract parameter-template receive --contract 42,0 --receive counter.increment`
- **AND** the resolved module source has no compatible embedded schema for `counter.increment`
- **THEN** the CLI exits with an actionable error
- **AND** does not print a parameter template

### Requirement: CLI downloads module source by module reference
The CLI SHALL provide a `ccd-wallet contract download-module <module-ref>` command that downloads smart contract module source bytes from the selected node and writes them to an explicit output file.

The command SHALL refuse to overwrite an existing output file unless an explicit overwrite option is supplied. The command SHALL support network or node endpoint selection and SHALL NOT require signer account selection, account unlocking, transaction approval, or transaction submission.

#### Scenario: User downloads module by reference
- **WHEN** the user runs `ccd-wallet contract download-module <module-ref> --out counter.wasm.v1`
- **AND** the node returns module source bytes for `<module-ref>`
- **THEN** the CLI writes the module source bytes to `counter.wasm.v1`
- **AND** prints a success message containing the module reference and output path

#### Scenario: Download refuses to overwrite existing file
- **WHEN** the user runs `ccd-wallet contract download-module <module-ref> --out counter.wasm.v1`
- **AND** `counter.wasm.v1` already exists
- **THEN** the CLI exits with an actionable error
- **AND** does not overwrite the file

#### Scenario: Module reference does not exist
- **WHEN** the user runs `ccd-wallet contract download-module <module-ref> --out counter.wasm.v1`
- **AND** the node cannot find the module source for `<module-ref>`
- **THEN** the CLI exits with an actionable error
- **AND** does not create the output file

### Requirement: CLI downloads module source by contract instance
The `contract download-module` command SHALL support resolving a module reference from a contract instance address. When a contract address is supplied instead of a module reference, the CLI SHALL query instance information, extract the source module reference, and download that module source.

#### Scenario: User downloads module by contract address
- **WHEN** the user runs `ccd-wallet contract download-module --contract 42,0 --out counter.wasm.v1`
- **AND** the node returns instance information for `<42, 0>`
- **AND** the node returns source bytes for the instance source module
- **THEN** the CLI writes the module source bytes to `counter.wasm.v1`
- **AND** prints a success message containing the resolved module reference and output path

#### Scenario: Contract module download cannot resolve instance
- **WHEN** the user runs `ccd-wallet contract download-module --contract 42,0 --out counter.wasm.v1`
- **AND** the node cannot find the instance at `<42, 0>`
- **THEN** the CLI exits with an actionable error
- **AND** does not create the output file

