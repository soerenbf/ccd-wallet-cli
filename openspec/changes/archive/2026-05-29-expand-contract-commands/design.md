## Context

The existing `contract deploy-module` command resolves wallet network and signer context, validates the request, prompts for explicit approval, submits the transaction, and waits for finalization by default. The connect server already supports contract init and update requests with similar low-level inputs, but that logic is currently tied to browser-session request handling. The SDK also exposes read-only contract invocation, instance metadata lookup, module source retrieval, and schema-driven JSON template generation through node queries plus embedded module schemas.

## Goals / Non-Goals

**Goals:**
- Provide direct CLI commands for contract init and update transactions with behavior consistent with `contract deploy-module`.
- Provide direct CLI commands for read-only invoke, instance info, parameter-template generation, and module source download.
- Reuse the existing wallet network/account resolution, node connection, approval, submission, and finalization patterns.
- Keep the contract interface predictable while supporting user-facing CCD decimal amounts, raw hex parameters, embedded-schema JSON parameters, explicit receive/init names, optional init/update energy with prompt fallback, and optional energy limits elsewhere.
- Use embedded module schemas from on-chain module sources for JSON parameter encoding where available.
- Leave room for future explicit external schema inputs and richer return-value rendering.

**Non-Goals:**
- Do not add explicit `--schema` file or base64 schema inputs in this change.
- Do not add browser JSON-RPC methods beyond the existing connect contract methods.
- Do not persist contract metadata locally.
- Do not require account unlocking or signing for read-only query commands.

## Decisions

### Share transaction core logic below command surfaces
Extract reusable init/update helpers into `smart_contracts` modules rather than duplicating transaction construction in both connect and CLI handlers. The command/connect layers should own UX differences, while shared helpers prepare payloads, perform simulations, submit transactions, and wait for finalization.

Alternative considered: duplicate init/update transaction code in `commands/contract`. That would be faster initially, but the existing `deploy-module` helper pattern already establishes a clear shared boundary and parity with connect is an explicit goal.

### Support hex parameters, embedded-schema JSON parameters, and parameter-template generation
`contract init`, `contract update`, and `contract invoke` will accept optional `--parameter-hex`, `--parameter-json`, and `--parameter-json-file` inputs, defaulting to an empty parameter when none is supplied. `--parameter-hex` treats the value as already serialized contract parameter bytes. `--parameter-json` accepts an inline JSON string. `--parameter-json-file` accepts a path to a JSON file. JSON-based inputs resolve the contract module source from the selected node, read the embedded module schema, and serialize the supplied JSON according to the init or receive parameter schema.

The CLI will also provide a separate `contract parameter-template` command that resolves the same embedded schema information and prints a JSON template for the selected init or receive parameter type. Default output should be pure pretty-printed JSON so users can redirect it into a file and fill it in.

The CLI will not support an explicit `--schema` flag in this change. Most target contracts are expected to have embedded schemas, and restricting JSON support to embedded schemas keeps the command surface smaller.

Alternative considered: require raw hex only initially. That is simpler and matches connect closely, but it makes the CLI unnecessarily awkward for users. Another alternative was folding template output into init/update/invoke flags; that saves a command, but it mixes inspection and execution concerns in a way that makes the UX harder to discover and script. Another alternative was adding `--schema`; that is useful for schema-less modules but can be added later without blocking embedded-schema support.

### Default read-only invoke context to the SDK's omitted invoker
`contract invoke` will not require `--invoker`. When omitted, the command will construct a contract context without an invoker so the node uses the SDK-defined synthetic zero-account invocation context. The command should not set `Some(AccountAddress([0; 32]))`, because explicit invokers must exist in block state.

Alternative considered: require a valid invoker account address. That is explicit, but it makes common no-argument view calls awkward and often forces users to find an account address for calls that do not depend on sender context.

### Make init and update energy optional with prompt fallback
`contract init` and `contract update` will allow `--energy` to be omitted in interactive mode. When omitted, the CLI will run simulation first when possible and prompt the user for an energy amount, defaulting the prompt to the simulated energy estimate. If simulation is unavailable or fails before an estimate is produced, the CLI should still prompt for energy with no simulated default.

In non-interactive mode, init and update must receive an explicit energy value.

Alternative considered: always require `--energy` for mutating commands. That is simpler, but it pushes an implementation detail onto users and ignores that the wallet can often estimate a sensible default.

### Make energy and parameter optional for invoke
Read-only invocation will default to node-selected energy and an empty parameter. Users can provide `--energy`, `--parameter-hex`, or `--parameter-json` when contracts require them.

Alternative considered: require energy and parameter for parity with update requests. That would make simple view entrypoints unnecessarily verbose.

### Accept decimal CCD amounts at the CLI boundary
Contract commands will accept amounts as user-facing decimal CCD strings, not microCCD integers. The command layer will parse those decimals into exact microCCD `Amount` values before constructing SDK payloads. This aligns with how users think about CCD while preserving exact chain units internally.

Alternative considered: expose microCCD values to mirror connect JSON-RPC fields. That is precise but unfriendly for CLI users and easy to misread.

### Keep parameter-template as a separate inspection command
Parameter-template generation should live under `contract parameter-template` rather than as a flag on init, update, or invoke. This keeps execution commands focused on execution and gives users a stable command for scaffolding JSON input files.

Recommended shape:
- `contract parameter-template init --module-ref <MODULE_REF> --init-name <INIT_NAME>`
- `contract parameter-template receive --contract <INDEX,SUBINDEX> --receive <CONTRACT.FUNCTION>`
- `contract parameter-template receive --module-ref <MODULE_REF> --receive <CONTRACT.FUNCTION>`

Alternative considered: add a `--show-parameter-template` flag to init/update/invoke. That is convenient in the short term, but it couples scaffolding behavior to execution-oriented commands and makes output shaping awkward.

### Separate mutating and read-only command behavior
Init/update commands require signer-capable wallet account resolution, explicit approval, transaction submission, and finalization waiting by default. Invoke/show/download-module/parameter-template commands only require network/node resolution and do not prompt for transaction approval or unlock signing material.

## Risks / Trade-offs

- JSON parameter encoding depends on embedded schemas being present on chain → Fail with an actionable error when `--parameter-json` or `--parameter-json-file` is requested but the module has no compatible embedded schema; keep `--parameter-hex` as an escape hatch.
- Multiple parameter input flags increase CLI surface area → Make them mutually exclusive and document the purpose of each flag clearly.
- Decimal amount parsing can be ambiguous if users supply too many fractional digits → Reject values with more than six decimal places and document that amounts are CCD decimals.
- Synthetic zero-account invoke defaults can surprise contracts that check sender/invoker → Document the default in CLI help and provide `--invoker` for explicit context.
- Energy prompting for init/update depends on simulation timing and UX flow → Keep the prompt behavior CLI-only, preserve explicit-energy behavior for connect, and ensure prompt defaults degrade cleanly when no estimate is available.
- Shared helper extraction may touch connect flows → Preserve existing connect behavior with tests around request parsing and approval results.
- Parameter-template generation for receive functions resolved by instance requires an extra instance-info lookup and module fetch → Keep direct module-reference resolution available for users who already know the module.
- Module download by instance requires an extra instance-info lookup → Keep direct module-reference download available for users who already know the reference.
