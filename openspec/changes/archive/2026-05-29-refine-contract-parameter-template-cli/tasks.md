## 1. CLI Surface

- [x] 1.1 Change `contract parameter-template init` to accept the init name as a positional argument instead of `--init-name`.
- [x] 1.2 Change `contract parameter-template receive` to accept the fully-qualified receive name as a positional argument instead of `--receive`.
- [x] 1.3 Update clap help text and parser tests for the refined positional interface.

## 2. Command Handling

- [x] 2.1 Update `commands::contract::parameter_template` to read positional init/receive names from the new argument structs.
- [x] 2.2 Preserve existing schema-source validation for `--module-ref` and `--contract` after the CLI change.

## 3. Documentation and Verification

- [x] 3.1 Update README examples for `contract parameter-template` to use positional init/receive names.
- [x] 3.2 Run `cargo fmt`.
- [x] 3.3 Run targeted `cargo test -p ccd-wallet` coverage for the updated parser and command paths.
- [x] 3.4 Run `OPENSPEC_TELEMETRY=0 openspec validate refine-contract-parameter-template-cli --strict`.
