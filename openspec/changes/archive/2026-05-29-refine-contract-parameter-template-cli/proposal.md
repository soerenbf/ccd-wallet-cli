## Why

The current `contract parameter-template` CLI shape works, but the init and receive target names feel overly flag-driven for a command whose primary job is inspection and scaffolding. Making the init name and receive name positional should make the command easier to discover, type, and remember.

## What Changes

- **BREAKING** Change `contract parameter-template init` to take the init name positionally instead of `--init-name`.
- **BREAKING** Change `contract parameter-template receive` to take the fully-qualified receive name positionally instead of `--receive`.
- Keep schema-source selection unchanged: init still requires `--module-ref`, and receive still resolves from exactly one of `--contract` or `--module-ref`.
- Update help text, examples, and tests to match the new positional interface.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `contract-instance-inspection`: refine the `contract parameter-template` command-line interface to use positional init/receive names.

## Impact

- Affects `crates/ccd-wallet/src/cli.rs` contract parameter-template argument parsing.
- Affects `crates/ccd-wallet/src/commands/contract/parameter_template.rs` argument handling.
- Requires README and test updates.
