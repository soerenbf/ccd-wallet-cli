## 1. Shared prompt-first command model

- [x] 1.1 Identify the user-facing command flows in scope (`seed add`, `seed use`, `seed remove`, `network add`, `identity new`).
- [x] 1.2 Add a `--non-interactive` flag to the scoped command argument models.
- [x] 1.3 Make user-facing non-secret arguments optional in interactive mode for the scoped flows.
- [x] 1.4 Ensure `--non-interactive` disables prompt fallback and returns actionable missing-input errors.
- [x] 1.5 Add a `--no-defaults` flag to flows that would otherwise silently use active seed/network selections.
- [x] 1.6 Ensure `--no-defaults` forces explicit selection instead of silently using active state.
- [x] 1.7 Apply `--no-defaults` to active-default flows including `seed show`, `identity new`, and `node info`.

## 2. Seed and network prompt flows

- [x] 2.1 Replace seed-flow input handling with `cliclack` prompts where input is collected interactively.
- [x] 2.2 Prompt for missing seed labels in `seed add` and `seed remove`, and use selector-based active-seed selection for `seed use` when interactive mode is allowed.
- [x] 2.3 Prompt for missing network registration inputs in `network add` when interactive mode is allowed.
- [x] 2.4 Keep secret values interactive-only and do not reintroduce secret CLI flags.

## 3. Identity issuance context and prompt consistency

- [x] 3.1 Identify where `identity new` resolves the effective seed and network context.
- [x] 3.2 Add a short, consistent context display for the effective seed label.
- [x] 3.3 Add a short, consistent context display for the effective network label and node endpoint.
- [x] 3.4 Ensure the same context display appears in both interactive and non-interactive identity issuance flows.
- [x] 3.5 Ensure explicit overrides and active-state-derived values both show the effective context actually being used.
- [x] 3.5a Ensure `--no-defaults` forces explicit selection of active seed/network entities in identity issuance, with the active entity preselected in the picker.
- [x] 3.5b Use a compact aligned derived-context summary and avoid immediately re-echoing values that were just selected interactively.
- [x] 3.6 Replace any remaining non-`cliclack` input prompts inside the identity issuance flow.
- [x] 3.7 Replace manual callback paste input with a `cliclack`-based prompt.
- [x] 3.8 Keep callback parsing semantics unchanged while switching prompt framework.
- [x] 3.9 Ensure password, provider selection, and manual callback entry all use `cliclack` consistently.

## 4. Verification and documentation

- [x] 4.1 Add or update tests for interactive prompt fallback and `--non-interactive` failure behavior where practical.
- [x] 4.1a Add or update tests for `--no-defaults` active-entity selection behavior where practical.
- [x] 4.1b Add or update tests for active-entity pickers defaulting to the current active selection where practical.
- [x] 4.1c Add or update tests for skipping single-option selectors where practical.
- [x] 4.1d Add or update tests for selector-based active seed/network selection where practical.
- [x] 4.2 Add or update tests for identity issuance context-display behavior where practical.
- [x] 4.3 Add or update tests for manual callback prompt flow where practical.
- [x] 4.4 Update README to document prompt fallback, `--non-interactive`, `--no-defaults`, and the clearer identity issuance context display.
- [x] 4.5 Run `cargo fmt`.
- [x] 4.6 Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [x] 4.7 Run `cargo test --workspace`.
