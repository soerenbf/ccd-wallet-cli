## Why

Many command argument structs use `Option<T>` for values that are syntactically absent, but command handlers then have to decide ad hoc whether absence means prompt, use a default, infer from context, or error. This makes command handlers repeat prompt/default/non-interactive logic and makes it harder to use Concordium domain types consistently at command boundaries.

## What Changes

- Add a reusable prepared-input layer between clap argument parsing and command execution.
- Introduce shared input-resolution primitives for promptable values, defaultable values, resolved values, input mode, validation policy, and finalization policy.
- Add shared clap flag groups for common command options such as input mode, network/node context, and transaction submission waiting.
- Prefer Concordium SDK/domain types and small CLI domain newtypes at parse boundaries, including account addresses, contract addresses, CCD amounts, token amounts, token identifiers, lock identifiers, and labels.
- Refactor command families incrementally so execution code consumes prepared/resolved command inputs instead of interpreting raw `Option<T>` directly.
- Include the token composition REPL and submit flow in the same model, parsing REPL values into domain or unresolved-domain types as early as practical.
- Use `stake configure delegation` as the first vertical slice, followed by token mutation commands, token composition, contract submission commands, local label/default cleanup, and governance flows.

## Capabilities

### New Capabilities
- `command-input-resolution`: Shared command-input preparation and resolution model for CLI and REPL command implementations.

### Modified Capabilities
- `interactive-cli-prompts`: Clarify that supported prompt-first flows use the shared input-resolution model for required missing values and non-interactive errors.
- `token-composition`: Clarify that the token compose REPL participates in the shared input-resolution model and parses inputs into domain/unresolved-domain values before saving or submitting.

## Impact

- Affected Rust code: `crates/ccd-wallet/src/cli.rs`, command modules under `crates/ccd-wallet/src/commands/`, token composition REPL code, and shared smart-contract/token parsing helpers.
- Documentation/spec impact: no user-facing command taxonomy changes are intended, but prompt/default semantics and token composition input handling are clarified.
- Compatibility: no intentional CLI breaking changes; the refactor should preserve existing command flags, prompts, defaults, and non-interactive behavior unless an inconsistency is explicitly corrected in a scoped task.
- SDK alignment: when refactoring a command input, first look for an appropriate Concordium Rust SDK domain type before adding a local wrapper.
