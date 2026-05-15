## Why

Users can add named seed phrases, but there is no way to select a default seed for subsequent wallet commands or inspect which seed is currently selected. Adding active-seed state mirrors the existing active-network workflow and prepares the CLI for account derivation and transaction commands that should default to the active seed.

## What Changes

- Add `ccd-wallet seed use <LABEL>` to set the active seed after validating that the label exists.
- Add `ccd-wallet seed show [LABEL]` to display the actual seed phrase after prompting for that seed's password.
- If `seed show` is called without a label, resolve and show the active seed.
- Persist the active seed label in the SQLite `wallet_state` table using a new `active_seed` key.
- `seed show` MUST unlock only the selected seed and MUST fail without revealing the seed phrase if the password is incorrect.
- Update README examples to document active seed usage and password-protected seed phrase display.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `seed-command`: Add seed selection and seed metadata display commands.
- `wallet-state`: Add `active_seed` mutable state alongside `active_network`.

## Impact

- **CLI surface**: Adds `ccd-wallet seed use <LABEL>` and `ccd-wallet seed show [LABEL]`.
- **Storage**: Uses existing `wallet_state` table; no schema migration required.
- **Code**: Extends seed command routing, seed storage lookup helpers, and password prompt handling for seed unlock.
- **Tests**: Add tests for active seed persistence, missing active seed errors, unknown seed rejection, successful seed phrase display after password prompt, and wrong-password behavior.
