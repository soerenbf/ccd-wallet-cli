## ADDED Requirements

### Requirement: Prompt-first flows use shared prepared input resolution
Supported prompt-first command flows that are refactored under the shared command-input model SHALL represent required missing inputs as promptable prepared values before execution. Resolving a promptable prepared value SHALL require the command flow to provide the prompt, selector, or domain-specific resolver that supplies the value in interactive mode.

#### Scenario: Refactored prompt-first flow resolves through prepared input
- **WHEN** a supported refactored command runs interactively without a required non-secret argument
- **THEN** the command represents the missing argument as a promptable prepared value
- **AND** resolves it through a command-specific `cliclack` prompt, selector, or shared domain resolver before execution continues

#### Scenario: Refactored non-interactive flow reports prepared input error
- **WHEN** a supported refactored command runs with `--non-interactive`
- **AND** a required non-secret argument is missing
- **THEN** resolving the promptable prepared value fails before command execution performs the operation
- **AND** the error explains which command-line value must be supplied

### Requirement: Defaultable flows preserve destructive-command safety
Supported refactored flows SHALL distinguish defaultable values from promptable values so that commands can opt into active defaults only where existing command semantics allow them. Destructive remove, delete, and reset flows SHALL NOT silently use active defaults merely because a defaultable helper exists.

#### Scenario: Non-destructive flow may use active default interactively
- **WHEN** a supported refactored non-destructive command has an omitted context value such as network or key source
- **AND** the command's existing semantics allow an active default
- **AND** the command runs interactively without `--no-defaults`
- **THEN** the command may resolve the value from the active default through the shared defaultable input model

#### Scenario: Destructive flow selects or prompts instead of silent active default
- **WHEN** a supported refactored destructive command such as delete, remove, or reset is missing its target
- **THEN** the command SHALL use an explicit selector or prompt according to its existing semantics
- **AND** SHALL NOT silently choose the active target solely through the shared defaultable input model
