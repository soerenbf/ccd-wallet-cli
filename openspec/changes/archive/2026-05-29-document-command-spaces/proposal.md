## Why

The CLI is growing beyond the initial account, contract, and governance surfaces, and the remaining command spaces need a documented structure before more transaction features are added. Protocol version 11 introduces `MetaUpdate` and protocol-level lock operations, so the project needs a canonical command taxonomy that keeps the user-facing `token` space coherent while also documenting how command code and command documentation stay in sync.

## What Changes

- Add a canonical command taxonomy document at `docs/commands.md` describing the intended top-level CLI spaces and the current or planned subcommand hierarchy.
- Document that protocol-level token operations, metadata updates, admin-role changes, and lock operations all belong under the `token` command space, using nested grouping where helpful instead of exposing protocol payload names directly in the CLI.
- Document the recommended `stake` structure as an umbrella over validator and delegation flows rather than a flat list of stake operations.
- Exclude deprecated pre-`ConfigureBaker` baker/validator transaction families from the documented validator command space so the taxonomy reflects recent protocol versions only.
- Document how future token-transaction composition should build on `token` subcommands without introducing `metaupdate` as a user-facing command path.
- Update `AGENTS.md` with a repository rule that command implementation changes and `docs/commands.md` must be kept in sync.

## Capabilities

### New Capabilities
- `command-taxonomy`: Define the canonical command-space documentation, including required coverage in `docs/commands.md`, exclusion of deprecated legacy baker transaction families, and synchronization expectations for command code and command docs.

### Modified Capabilities
- None.

## Impact

- Affects CLI planning and future command implementation work in `crates/ccd-wallet/src/cli.rs` and `crates/ccd-wallet/src/commands/`.
- Adds a new canonical documentation file at `docs/commands.md`.
- Updates repository guidance in `AGENTS.md` so command-surface changes and documentation remain aligned.
- Provides a stable reference for future `token` composition work built on top of protocol version 11 `MetaUpdate` and lock functionality.
