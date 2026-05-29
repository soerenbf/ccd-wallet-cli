## Context

`contract parameter-template` is a read-only inspection command used to scaffold JSON files for contract init and receive parameters. The current CLI shape requires `--init-name` and `--receive`, which makes the core target name look secondary compared with the schema-source flags.

## Goals / Non-Goals

**Goals:**
- Make the init name positional for `contract parameter-template init`.
- Make the fully-qualified receive name positional for `contract parameter-template receive`.
- Preserve existing schema-source resolution rules and output behavior.
- Keep the command self-descriptive and easy to script.

**Non-Goals:**
- Do not change parameter-template output format.
- Do not change schema-source flags such as `--module-ref` or `--contract`.
- Do not change init/update/invoke parameter input flags in this change.

## Decisions

### Use positional target names for the primary template subject
The init name and receive name are the central subject of the command, so they should be positional values rather than optional-looking flags. This yields:

- `contract parameter-template init <INIT_NAME> --module-ref <MODULE_REF>`
- `contract parameter-template receive <CONTRACT.FUNCTION> --contract <ADDRESS>`
- `contract parameter-template receive <CONTRACT.FUNCTION> --module-ref <MODULE_REF>`

Alternative considered: keep `--init-name` and `--receive`. That is explicit, but it adds ceremony without improving clarity for a command that already has subcommands separating init and receive cases.

### Keep schema-source selection as flags
Schema-source selection remains secondary context and should stay as flags:
- init: `--module-ref`
- receive: exactly one of `--contract` or `--module-ref`

Alternative considered: make schema sources positional too. That would make the command shorter, but it would be harder to read and more error-prone because the source and target values have different shapes.

## Risks / Trade-offs

- This is a breaking CLI change for a newly added command → Update README and parser tests in the same change.
- Positional receive names depend on users passing fully-qualified names → Keep validation and actionable errors for malformed receive names.
