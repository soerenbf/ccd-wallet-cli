## Context

`ccd-wallet` has grown several command families with similar input semantics: clap parses `Option<T>`, then command handlers decide whether absence means prompt, default, infer, or error. The same flags (`--non-interactive`, `--no-defaults`, `--network`, `--node`, `--no-wait`) are repeated across many structs, and command handlers frequently pass raw booleans through helper stacks.

This change introduces a prepared-input layer that keeps clap parsing simple while making command execution explicit about input semantics. The design must preserve the existing CLI surface, prompt behavior, and non-interactive determinism while enabling incremental refactors.

## Goals / Non-Goals

**Goals:**

- Preserve ergonomic clap parsing and avoid custom clap wrapper boilerplate.
- Introduce shared prepared-input primitives for promptable, defaultable, and resolved values.
- Introduce shared clap flag groups for input mode, network/node selection, and submission waiting.
- Prefer Concordium SDK/domain types and small label/newtype wrappers as early as practical.
- Make command handlers resolve prepared inputs in dependency order, including async prompts/defaults.
- Include both normal CLI commands and the token compose REPL.
- Start with `stake configure delegation` as the first vertical slice, then expand to token, contract, local entity, and governance flows.

**Non-Goals:**

- No intentional user-facing command taxonomy changes.
- No change to persisted wallet database schema or encryption model.
- No attempt to make clap parse directly into promptable/defaultable wrappers.
- No broad behavioral cleanup unrelated to input-resolution semantics.

## Decisions

### Keep clap args simple and add prepared command inputs

Clap-facing structs SHALL remain ergonomic and may continue to use `Option<T>` where the CLI argument is absent or supplied. Each refactored command then converts into a prepared type using semantic wrappers such as `Promptable<T>` and `Defaultable<T>` when resolution policy must be enforced. Prepared command inputs that are required for the command's operation should generally become `Promptable<T>`, even when the prompt can use existing state as displayed context or as an interactive default.

Alternative considered: make clap parse directly into `Promptable<T>`. This was rejected because clap would require per-field default/display/parser boilerplate and still could not perform database-backed defaults, prompts, or async node-backed resolution.

### Resolve by calling methods with sync or async providers

`Promptable<T>` and `Defaultable<T>` SHALL not store prompt closures. Instead, they SHALL expose methods such as `resolve_with(...)`, `resolve_with_async(...)`, and default-aware equivalents. This forces command code to explicitly provide a prompt/default provider while keeping lifetimes and async handling simple.

Alternative considered: store boxed prompt closures or boxed futures inside the wrapper. This was rejected because it would add lifetime noise and hide resolution dependencies that are clearer in the command resolver.

### Model absence semantics explicitly

Prepared input types SHALL distinguish these cases:

- `Promptable<T>`: missing means prompt in interactive mode or error in non-interactive mode.
- `Defaultable<T>`: missing means use an allowed interactive default/inference, otherwise prompt or error according to `InputMode`.
- `Option<T>`: missing means the value is genuinely optional for the command.

For stateful commands such as `stake configure delegation`, omitted target/capital/restake values should be represented as `Promptable<T>`. When current chain state exists, the prompt should show that state in its label or context, for example `Capital (current: 1000.12 CCD)`, and may use it as the prompt default when defaults are allowed. In non-interactive mode, omitted required values still produce actionable errors. After resolving a complete desired delegation configuration, the command diffs it against current state and includes only changed fields in the final transaction payload.

### Centralize common modes and policies

The shared input module SHALL define `InputMode`, prompt/default policies, `Resolved<T>` with source metadata, `FinalizationPolicy`, and `ValidationPolicy`. `InputMode` SHALL encode that defaults are not filled in non-interactive mode.

### Add shared clap flag groups

Repeated clap flags SHALL be grouped behind shared structs where doing so preserves the public command surface:

- input mode: `--non-interactive`, `--no-defaults`
- network/node selection: `--network`, `--node`
- submission waiting: `--no-wait`

These groups reduce duplicated command structs while keeping existing flag names and help text consistent.

### Prefer domain types early

Raw strings SHALL be replaced incrementally with Concordium Rust SDK domain types or small wrappers when the command syntax has a clear domain meaning. Each refactor should first search the SDK for an existing domain type before introducing a local wrapper. Examples include `AccountAddress`, `ContractAddress`, `TokenId`, `LockId`, `TokenAmount` where decimals are known, `CcdAmount`, and label newtypes such as `AccountLabel`, `NetworkName`, and `KeySourceLabel`.

For read-only account references, use a distinct input type that can represent either a raw account address or a local account label. For signing senders, use a local account label type rather than accepting raw addresses.

### Include token compose REPL in scope

The token compose REPL has its own mini-parser and prompt fallback implementation. It SHALL be refactored to use the same prepared-input concepts where practical. REPL inputs SHOULD be parsed into domain or unresolved-domain types before validation and saving, while preserving symbolic plan concepts such as `@sender` and local lock references.

### Refactor vertically and incrementally

The first implementation slice SHALL refactor `stake configure delegation` to prove the model across promptable account selection, promptable transaction inputs with current-state prompt defaults, defaultable network context, validation policy, submission policy, and async node validation. Subsequent slices SHALL reuse the same primitives across token mutations, token composition, contract submissions, local entity flows, and governance.

## Risks / Trade-offs

- **Risk: Prepared types add another layer of indirection.** → Keep prepared/resolved structs close to each command module and avoid generic abstraction beyond the shared primitives.
- **Risk: Refactors accidentally alter prompt/default behavior.** → Preserve existing behavior with focused tests for each vertical slice, especially non-interactive errors and active-default behavior.
- **Risk: Shared flag groups change clap help or conflict behavior.** → Introduce groups carefully and compare command help/parse behavior where commands have special conflicts.
- **Risk: Domain parsing too early rejects symbolic compose values.** → Use unresolved-domain types for plan-specific symbols such as `@sender` and lock references instead of forcing final chain types prematurely.
- **Risk: Async resolution gets over-abstracted.** → Prefer explicit command resolver methods and only add traits if repeated code justifies them.

## Migration Plan

1. Add the shared input primitives and shared clap flag groups without changing command behavior.
2. Add domain/newtype parsers that can coexist with current raw string paths.
3. Refactor `stake configure delegation` as a vertical slice and add/adjust tests.
4. Refactor token mutation shared helpers and direct token commands.
5. Refactor token compose submit and REPL operation parsing.
6. Refactor contract submission commands.
7. Refactor local entity/default/delete flows and governance flows.
8. Remove obsolete ad hoc prompt/default helpers once no longer used.

Rollback is straightforward per slice: keep behavior-preserving tests and avoid coupling unrelated command families in one change step.

## Open Questions

- Exact module names may be adjusted during implementation; the default location is `crates/ccd-wallet/src/commands/input.rs` plus small domain modules if needed.
- The final list of domain label newtypes may be refined as command families are refactored.
