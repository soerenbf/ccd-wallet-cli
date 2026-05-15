## Why

The wallet can create and select several kinds of local entities, but it still lacks a consistent way to inspect what exists and to relabel entities after they have been created. As the number of configured seeds, networks, identities, and accounts grows, `list` and `rename` become necessary for day-to-day usability and for keeping the CLI's entity management model coherent.

## What Changes

- Add human-oriented `list` commands for `network`, `seed`, `identity`, and `account`.
- Add `rename` commands for `network`, `seed`, `identity`, and `account`.
- Make `identity list` and `account list` scope-aware using the same seed/network context model as other context-bearing commands, including explicit `--seed all` / `--network all` scope values.
- Add additional list filters where relevant, such as listing identities or accounts on a specific network and identity provider.
- Keep account addresses hidden in `account list` unless explicitly requested with a flag.
- Ensure rename commands support interactive source selection when the old label/name is omitted.
- Use a global fuzzy selector for `identity rename` and `account rename` when the source is omitted, with network and seed metadata included in the searchable text.
- Update active state when renaming the active network or active seed.

## Capabilities

### New Capabilities
- `entity-listing`: Human-oriented listing commands for networks, seeds, identities, and accounts, including context-aware scope selection and filter parameters for identity/account listings.
- `entity-rename`: Rename commands for networks, seeds, identities, and accounts, including interactive source selection and fuzzy searchable selection for identities/accounts.

### Modified Capabilities
- `config-storage`: Network rename moves the config entry to a new key while preserving the stored network data and keeping active-network state consistent.
- `seed-storage`: Seed rename updates only the plaintext label while preserving the stable seed id and keeping active-seed state consistent.
- `identity-storage`: Identity rows become queryable by scope and filter metadata and renamable without changing their underlying network/seed/provider/index identity.
- `account-storage`: Account rows become queryable by scope and filter metadata and renamable without changing their underlying derivation identity, and account listing can optionally reveal encrypted addresses when explicitly requested.

## Impact

- Affected code: `crates/ccd-wallet/src/cli.rs`, `crates/ccd-wallet/src/commands/{config/network,seed,identity,account,ui}.rs`, and the relevant store modules in `crates/ccd-wallet-core/src/store/`.
- Affected systems: JSON config handling for networks, SQLite storage/query APIs for seeds/identities/accounts, wallet-state active selection updates, and interactive CLI selection UX.
- User-facing behavior: new `list`/`rename` subcommands, context-bearing and filterable list output for identities/accounts, explicit `--seed all` / `--network all` scope values for list commands, and fuzzy searchable rename selection for identities/accounts.
