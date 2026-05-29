## 1. CLI Surface

- [x] 1.1 Add `contract init`, `contract update`, `contract invoke`, `contract show`, `contract parameter-template`, and `contract download-module` subcommands and argument structs in `cli.rs`.
- [x] 1.2 Add CLI parsing tests for the new contract subcommands, including optional invoke parameter, `--parameter-json`, `--parameter-json-file`, decimal amount, optional init/update energy, and invoker arguments.
- [x] 1.3 Add shared parsers for contract addresses, module references, raw hex parameters, inline JSON parameter values, JSON parameter file paths, decimal CCD amounts, and optional block selectors as needed.

## 2. Shared Contract Transaction Helpers

- [x] 2.1 Extract contract init preparation, simulation, submission, and finalization helpers into `smart_contracts::init`.
- [x] 2.2 Extract contract update preparation, simulation, submission, and finalization helpers into `smart_contracts::update`.
- [x] 2.3 Update connect init/update handlers to use the shared helpers while preserving existing JSON-RPC behavior and error mapping.
- [x] 2.4 Add unit tests for invalid init/update names, parameter hex decoding, embedded-schema JSON parameter encoding from `--parameter-json` and `--parameter-json-file`, decimal amount parsing, energy prompt default derivation from simulation, payload preparation, and invoke default construction where practical.

## 3. Mutating Contract Commands

- [x] 3.1 Implement `commands::contract::init` with network/account resolution, decimal amount parsing, optional `--parameter-hex`, `--parameter-json`, or `--parameter-json-file` inputs, optional energy prompting, optional simulation, review prompt, explicit approval, submission, and default finalization waiting.
- [x] 3.2 Implement `commands::contract::update` with network/account resolution, decimal amount parsing, optional `--parameter-hex`, `--parameter-json`, or `--parameter-json-file` inputs, optional energy prompting, optional simulation, review prompt, explicit approval, submission, and default finalization waiting.
- [x] 3.3 Use simulation results to prefill interactive init/update energy prompts when available, and require explicit energy in non-interactive mode.
- [x] 3.4 Reuse transaction summary rendering for finalized init/update outcomes and honor `--no-wait` after successful submission.
- [x] 3.5 Ensure user-declined init/update flows exit without submitting transactions.

## 4. Read-only Contract Commands

- [x] 4.1 Implement `smart_contracts::invoke` helper logic for building and executing node query invocation contexts.
- [x] 4.2 Implement `contract invoke` without account unlocking, with omitted invoker by default, empty parameter by default, zero CCD amount by default, and node-selected energy by default.
- [x] 4.3 Implement optional `--invoker`, `--parameter-hex`, `--parameter-json`, `--parameter-json-file`, decimal `--amount`, `--energy`, block selection, and `--json` output for invoke.
- [x] 4.4 Implement `contract show` using instance-info node queries with human-readable and JSON output.
- [x] 4.5 Implement parameter-template helpers that resolve embedded schemas by module reference or by contract instance and render pretty-printed JSON templates from init/receive parameter types.
- [x] 4.6 Implement `contract parameter-template init` and `contract parameter-template receive` with pure JSON output and mutually exclusive `--contract` / `--module-ref` resolution in receive mode.
- [x] 4.7 Implement `contract download-module` by module reference, including output-file overwrite protection.
- [x] 4.8 Implement `contract download-module --contract` by resolving the instance source module before downloading source bytes.

## 5. Documentation and Verification

- [x] 5.1 Update README contract command documentation with init, update, invoke, show, parameter-template, and download-module examples, including decimal amounts plus `--parameter-json` and `--parameter-json-file` with embedded schemas.
- [x] 5.2 Add or update tests for contract command dispatch and non-mutating commands not requiring account unlocking, including parameter-template resolution rules.
- [x] 5.3 Run `cargo fmt`.
- [x] 5.4 Run relevant `cargo test` targets for `ccd-wallet` and `ccd-wallet-connect`.
- [x] 5.5 Run `OPENSPEC_TELEMETRY=0 openspec validate expand-contract-commands --strict` and address any validation issues.
