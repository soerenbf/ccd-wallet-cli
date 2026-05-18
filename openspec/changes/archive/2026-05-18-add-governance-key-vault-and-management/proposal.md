## Why

Chain governance work needs local management of update-signing keypairs that are distinct from both seed-derived accounts and imported account signers. The wallet should be able to import, inspect, and remove governance keys in a network-scoped vault while treating live chain parameters—not static genesis snapshots—as the source of truth for which keys are authorized for which governance purposes.

## What Changes

- Add a separate governance key vault scoped by network genesis hash, with its own password domain and lifecycle.
- Add `ccd-wallet governance keys import <file>` for importing a single governance keypair JSON file.
- Add `ccd-wallet governance keys import --dir <dir>` for importing a directory of governance keypair files while ignoring aggregate governance snapshot files.
- Add `ccd-wallet governance keys list`, which preflights whether a governance vault exists for the resolved network, unlocks the governance vault only when present, decrypts stored key material, queries live chain parameters, and shows which stored keys are currently authorized using a tag-first, capability-oriented list format.
- Add `ccd-wallet governance keys remove <verify-key>` and `ccd-wallet governance keys remove --all` for deleting imported governance key material.
- Identify governance keys by public key instead of user-defined labels.
- Store only encrypted raw governance keypair JSON payloads and vault metadata locally; do not store governance authorization snapshots, plaintext public keys, or derived governance-level metadata in the database.
- Prepare the model for a future `ccd-wallet governance update <type> ...` flow that will derive the required signers from live chain state at signing time.

## Capabilities

### New Capabilities
- `governance-key-vault`: Store governance signing keypairs in a separate network-scoped encrypted vault.
- `governance-key-management`: Import, list, and remove governance keypairs while matching them against live chain authorization state.

### Modified Capabilities
- `network-reset-delete`: Resetting a network partition also removes governance vault data and imported governance keys for that network.
- `node-connectivity`: Governance key inspection uses live chain queries for chain parameters and update-related authorization state.

## Impact

- SQLite schema and migrations for governance vaults and encrypted governance key payloads.
- New CLI command surface under `governance keys`.
- Node query integration for live chain parameters and future update-sequence-number-driven governance signing flows.
- Reset/delete behavior for network partitions so governance vault data follows the same network-scoped cleanup model.
- Governance key listing UX is refined so missing-vault cases fail before password entry and authorized keys are grouped in operator-centric order (`level 2`, `level 1`, `root`, then `not authorized`) with aligned key columns.
- Account list output is refreshed to use a bracket-first format similar to governance keys, tagging rows with the owning seed label or `imported` before the account label and optional address.
- `account list` defaults to all accounts on the resolved network instead of the active seed, while `--seed` remains available as an explicit filter and becomes required for `--show-addresses`.
- No governance update submission command is included in this first change, but the data model should support it cleanly later.
