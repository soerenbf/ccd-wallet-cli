## Why

The CLI still mixes command-line-required arguments with ad hoc interactive input, which makes flows feel inconsistent and forces users to remember details that the tool could ask for. A prompt-first model with an explicit `--non-interactive` escape hatch would make guided usage much smoother while preserving scripting-friendly behavior.

## What Changes

- Introduce a prompt-first CLI model for user-facing flows that currently require command-line arguments.
- Make most user-facing command arguments optional in interactive mode and request missing values through `cliclack` prompts.
- Add a `--non-interactive` flag that disables prompt fallback and causes missing required values to produce actionable errors.
- Add a `--no-defaults` flag for flows that would otherwise silently use active seed/network selections; with this flag, the CLI forces an explicit picker choice and preselects the current active entity in that picker.
- Standardize interactive input handling on `cliclack` across identity issuance, seed management, and other prompt-driven flows.
- Reuse a shared compact context-summary pattern so derived values can be shown without re-echoing prompted selections.
- Use selectors instead of free-text entry for interactive active-entity selection flows such as `seed use` and `network use`.
- Skip picker prompts automatically when only one valid option exists.
- Keep underlying storage, network resolution, and issuance protocol behavior unchanged.

## Capabilities

### New Capabilities
- `interactive-cli-prompts`: A consistent prompt-first CLI interaction model with `cliclack` prompts and explicit `--non-interactive` opt-out.

### Modified Capabilities
- `seed-command`: Missing seed-command arguments are prompted interactively unless `--non-interactive` is supplied, and seed-related input uses `cliclack` consistently.
- `network-config-add`: Missing network registration inputs can be requested through `cliclack` prompts unless `--non-interactive` is supplied.
- `identity-issuance`: Missing identity issuance inputs can be requested through `cliclack` prompts unless `--non-interactive` is supplied, resolved context is displayed consistently, and `--no-defaults` forces explicit selection instead of silently using active seed/network state.
- `node-connectivity`: `--no-defaults` disables silent fallback to the active network and forces explicit network selection when no endpoint selector is supplied.

## Impact

- `crates/ccd-wallet/src/cli.rs` and command handlers will need argument/prompt restructuring.
- Interactive command flows will centralize around `cliclack`.
- Tests will need to cover prompted mode, `--non-interactive` failure behavior, `--no-defaults` active-entity selection behavior, and single-option picker elision.
- README examples/help text will need updates to explain prompt fallback, the new `--non-interactive` mode, and `--no-defaults` behavior.
