## 1. Storage and query foundations

- [x] 1.1 Add network config helpers for listing networks and renaming a network key while preserving entry data
- [x] 1.2 Add seed store helpers for listing seeds and renaming a seed label while preserving the stable seed id
- [x] 1.3 Add identity store helpers for scope-aware and filterable listing plus rename-by-label support
- [x] 1.4 Add account store helpers for scope-aware and filterable listing, rename-by-label support, and optional address decryption for display under a single-seed scope
- [x] 1.5 Update active seed/network state handling so renaming the active entity updates wallet-state consistently
- [x] 1.6 Add or update unit tests for config/store rename and list behavior, including collision handling

## 2. CLI command surface

- [x] 2.1 Extend CLI subcommand enums to add `list` and `rename` for network, seed, identity, and account
- [x] 2.2 Implement human-oriented `network list` / `seed list` and their rename flows, including interactive source selection when the old name is omitted
- [x] 2.3 Implement `identity list` / `account list` with context-aware seed/network scope resolution, explicit `--seed all` / `--network all`, resolved-context display, `--status` on both commands, and `--provider` on identity list only
- [x] 2.4 Add interactive `All seeds` and `All networks` selector rows for identity/account list scope selection where those pickers are used
- [x] 2.5 Implement `identity rename` / `account rename` with global fuzzy source selection when the old label is omitted, including searchable seed and network metadata and conditional status badges for non-happy-state rows
- [x] 2.6 Implement optional account-address reveal in `account list`, plus `account rename --show-addresses` with required explicit-or-selected seed scope

## 3. Output, UX, and documentation

- [x] 3.1 Add human-oriented output formatting for all four list commands and ensure active/default context is shown first for context-bearing list flows
- [x] 3.2 Add command-level tests covering fuzzy rename source selection, conditional status badges, `all` scope values, identity/account status filtering, identity-only provider filtering, and hidden-vs-revealed account addresses including rename-with-addresses seed scoping
- [x] 3.3 Update README and command documentation for the new `list` and `rename` commands, including `--seed all` / `--network all`, `--status`, identity-only `--provider`, fuzzy rename search behavior, and `account rename --show-addresses` seed requirements
- [x] 3.4 Run formatting, linting, and relevant test suites for the workspace and fix any issues found
