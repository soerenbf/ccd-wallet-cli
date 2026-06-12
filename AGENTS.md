# AGENTS.md

- This repo contains a Rust workspace (`Cargo.toml`, `crates/`) and a pnpm workspace (`package.json`, `packages/`). Keep changes scoped to the relevant ecosystem and avoid coupling Cargo and pnpm workflows unnecessarily.
- Public crate/package functionality should be documented to release quality.
- Public types should have a top-level description; structured fields should have field-level descriptions.
- Public functions/methods should have a description, parameter docs, return docs, error docs where relevant, and examples unless usage is self-explanatory.
- Source modules should have a short top-level docstring/comment describing the file’s responsibility.
- Use Rust doc comments for Rust public APIs and JSDoc for TypeScript public APIs.
- Browser-facing TypeScript packages should stay environment-flexible and avoid Node-specific runtime assumptions unless intentionally Node-only.
- Keep the command surface and `docs/commands.md` in sync. When changing clap command definitions, top-level command spaces, nested subcommand structure, or command taxonomy intent, update the code and `docs/commands.md` in the same change.
- Keep the database structure and encryption model in sync with `docs/db-structure.md` and `docs/encryption-model.md`. When changing persisted schema/layout, stored payload semantics, or encryption/key-management behavior, update the code and those docs in the same change.

## Connect API

- Keep connect-related code feature-oriented across both TypeScript and Rust.
  - In TypeScript connect packages, prefer `core/` for transport/protocol primitives and `features/` with one module per connect capability.
  - In Rust `commands::connect`, prefer one module per connect capability plus a small shared module for reused helpers.
  - When adding a new connect capability, mirror the capability split across the TypeScript client and Rust connect server where practical so the architecture stays easy to navigate.
- For connect API interaction logging, prefer `cliclack::log::{info, warning, success, error}` over `println!` / `eprintln!`. Reserve plain terminal printing for end-of-program output rather than interactive connect flow messages.
- For connect transaction submission and finalization progress, prefer `cliclack` spinners so interactive users can see that node-backed work is ongoing.

## CLI

- Prefer `cliclack` helpers for user prompts, confirmations, and input parsing where practical to keep the user experience consistent across commands. For non-interactive CLI commands, prefer structured error handling and machine-readable output over human-oriented printing.
- `--non-interactive` forces declaration of all required fields as command args and opts out of prompts and confirmations. It also implies `--json` output where applicable.
- `--json` outputs machine-readable JSON instead of human-oriented terminal printing.
- `--no-defaults` opts out of default value filling for any fields that have defaults.
- defaults are only filled in interactive mode, never in non-interactive mode, and the CLI should error if required fields with defaults are not provided in non-interactive mode.
- For CLI commands that mutate chain state, prefer a final confirmation step that summarizes the intended changes before submission. Use `cliclack` formatting to make the confirmation content scannable and clear.
