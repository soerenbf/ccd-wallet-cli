## Context

The CLI has evolved command by command. Some flows are primarily argument-driven, some prompt for sensitive values, and some use `cliclack` while others still use lower-level terminal input. This creates two related problems:

1. users do not get a consistent guided experience
2. commands remain more rigid than necessary because values that could be prompted still have to be passed on the command line

The user wants the CLI to bias toward interactive prompting, while still supporting a strict mode where missing command-line inputs cause an error.

## Goals / Non-Goals

**Goals:**
- Introduce a coherent prompt-first model across user-facing flows.
- Use `cliclack` consistently for interactive input collection.
- Allow most user-facing arguments to be omitted in interactive mode.
- Add `--non-interactive` to force error-on-missing-input behavior.
- Add `--no-defaults` to disable silent active-state defaults and require explicit selection of active seed/network entities.
- Preserve automation and scripting support.

**Non-Goals:**
- No attempt to make secret values available through command-line flags.
- No changes to storage semantics, network semantics, or issuance protocol semantics.
- No change to read-only informational commands that do not need user input.
- No requirement that every single subcommand become interactive immediately if it has no meaningful prompted UX.

## Decisions

### Prompt-first, not prompt-only

Commands should prefer prompting for missing user-facing values, but `--non-interactive` should restore strict CLI behavior.

Rationale:
- Best fit for human use.
- Preserves deterministic scripting behavior.

Alternative considered: always prompt and remove strict CLI behavior. Rejected because it harms automation.

### `--non-interactive` remains the strict missing-input override

Use a shared `--non-interactive` flag on relevant commands to disable prompt fallback for missing values.

Rationale:
- Simple mental model.
- Easy to document and test.
- Encourages consistent behavior across commands.

### `--no-defaults` disables silent active-state defaults

Use a shared `--no-defaults` flag on commands that would otherwise silently use active seed or active network state. With this flag, the command must ask the user to choose explicitly from a picker, and the active entity should be preselected in that picker when one exists.

Rationale:
- Keeps default behavior convenient while making the implicit choice visible when requested.
- Gives users a safety mode that still remains interactive, unlike `--non-interactive`.

Alternative considered: make `--non-interactive` also disable defaults. Rejected because it mixes two concerns: disabling prompts vs requiring explicit selection.

### Keep secrets interactive-only

Prompt fallback applies to user-facing metadata like labels, names, provider selection, and endpoints. Secret values such as passwords and seed phrases remain interactive-only and are still never accepted on the command line.

Rationale:
- Preserves existing security posture.
- Avoids conflating convenience prompting with relaxed secret handling.

### Use `cliclack` for all interactive command input in scoped flows

Within the flows touched by this change, interactive input collection should use `cliclack` consistently.

Rationale:
- Unified visual language.
- Fewer mixed prompt implementations.

### Effective-context display remains part of identity issuance

For identity issuance specifically, the seed/network context display should remain and fit into the broader prompt-first model.

Rationale:
- Missing-value prompting and explicit context display complement each other.

### Derived context should be shown once, compactly

When a command resolves context from active defaults, explicit CLI overrides, or inferred metadata, it should display that resolved context as a compact aligned block. When the user just chose the same value in an interactive picker during the current run, the CLI should not immediately restate it.

Rationale:
- Keeps important derived context visible.
- Avoids redundant noise right after a picker interaction.

### Existing-entity selection should prefer selectors

When a flow asks the user to choose from already configured seeds or networks, the CLI should use an interactive selector instead of asking the user to retype an existing label.

Rationale:
- Reduces typing for known finite choices.
- Avoids typo-prone repetition when the tool already knows the available options.

### Single-option pickers should be skipped

If a picker would present exactly one valid option, the CLI should select it automatically instead of rendering a one-item selector.

Rationale:
- Removes unnecessary interaction.
- Keeps the prompt-first model efficient when configuration is minimal.

## Risks / Trade-offs

- **Risk:** Making arguments optional may blur which values are actually required for a command to succeed.  
  **Mitigation:** Keep help text explicit and make prompted questions direct and minimal.

- **Risk:** `cliclack` prompt handling may make some tests more involved.  
  **Mitigation:** Keep parsing/validation logic separate from prompt plumbing where possible.

- **Risk:** Some commands may have ambiguous prompt order when multiple values are missing.  
  **Mitigation:** Prompt in a deterministic order that matches the command’s conceptual flow.

- **Risk:** Prompt-first UX can surprise automation users if accidentally triggered.  
  **Mitigation:** `--non-interactive` provides an explicit strict mode, and commands should produce actionable errors in that mode.

- **Risk:** `--no-defaults` may appear redundant next to prompt fallback.  
  **Mitigation:** Document the distinction clearly: prompt fallback asks for missing values, while `--no-defaults` prevents silent use of active selections.

## Migration Plan

- No schema migration.
- Update command parsing/help, prompt flow, tests, and docs.

## Open Questions

- Whether `--non-interactive` should be per-command only or also supported as a future global flag. For this change, command-level support is sufficient.
