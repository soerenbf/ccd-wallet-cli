## Why

The encrypted seed storage layer exists, but users still have no CLI command to add a seed phrase to the wallet database. This is also a good moment to stabilize the CLI grammar by promoting network management from `ccd-wallet config network ...` to `ccd-wallet network ...` before the wallet gains more user-facing commands.

## What Changes

- Add a `ccd-wallet seed add <LABEL>` command that prompts interactively for a seed phrase and password, validates the seed phrase, and persists it through the encrypted seed store.
- Add hidden interactive prompts for sensitive inputs: seed phrase, password, and password confirmation. Sensitive values MUST NOT be accepted as CLI arguments.
- Validate seed phrases before storage. The wallet SHALL reject invalid mnemonic phrases and avoid writing them to the DB.
- Add a top-level `ccd-wallet network ...` command group with the existing `add` and `use` subcommands.
- **BREAKING**: Remove the `ccd-wallet config network ...` command path in favor of `ccd-wallet network ...`.
- Keep the underlying `config.json` network registry unchanged.

## Capabilities

### New Capabilities

- `seed-command`: User-facing seed commands, initially `ccd-wallet seed add <LABEL>` with interactive seed/password prompting and mnemonic validation.

### Modified Capabilities

- `network-config-add`: Change the network registration CLI path from `ccd-wallet config network add` to `ccd-wallet network add`.
- `active-network-selection`: Change the active-network CLI path from `ccd-wallet config network use` to `ccd-wallet network use`, with active network still persisted in `wallet_state`.
- `wallet-state`: Update user-facing examples/errors to reference `ccd-wallet network use <NAME>` instead of `ccd-wallet config network use <NAME>`.
- `config-storage`: Update config initialization scenarios to use the new top-level `network add` command.
- `node-connectivity`: Update active-network scenarios and error text to reference the SQLite wallet-state store and new `network use` command path.

## Impact

- **CLI surface**: Adds `seed` top-level command group; promotes `network` to a top-level command group; removes `config network` path.
- **Dependencies**: likely add `bip39` for mnemonic validation and `rpassword` (or equivalent) for hidden terminal prompts.
- **Code**: update `src/cli.rs`, `src/main.rs`, command routing, and add a seed command module that calls `store::seeds::add`.
- **Tests**: add command-level tests or unit tests for mnemonic validation, prompt handling abstraction, duplicate label handling, and network command routing.
