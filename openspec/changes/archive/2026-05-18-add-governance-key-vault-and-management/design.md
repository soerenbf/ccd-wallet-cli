## Context

Concordium chain governance is signed by update keys that are separate from account signers. In node test-run directories these appear as individual keypair JSON files such as `root-key-0.json`, `level1-key-3.json`, and `level2-key-12.json`, alongside an aggregate `governance-keys.json` snapshot. The aggregate file is useful at genesis time but is not authoritative in a live chain because governance keys and access structures can be rotated later.

The wallet therefore needs a local place to store governance keypairs, but it should derive governance levels and current authorization from live chain state rather than persisting a possibly stale snapshot. This is also the right foundation for a future `ccd-wallet governance update <type> ...` flow, where signer selection must be driven by current chain parameters and current update sequence numbers.

## Goals / Non-Goals

**Goals:**
- Add a separate governance key vault scoped by network genesis hash with its own password domain.
- Import governance keypair JSON files one at a time and via `--dir` bulk import.
- Store only encrypted raw governance key JSON payloads plus minimal vault metadata.
- Prevent governance key inspection without unlocking the governance vault.
- Implement `governance keys list` by decrypting local key material and matching it against live chain parameters to derive levels and authorization.
- Identify governance keys by public key rather than user-facing labels.
- Support `governance keys remove <verify-key>` and `governance keys remove --all`.
- Ensure `network reset` removes governance vault data for the affected network partition.

**Non-Goals:**
- Implement `governance update <type> ...` submission in this change.
- Persist aggregate governance authorization snapshots from `governance-keys.json`.
- Display governance key metadata without the vault password.
- Support arbitrary non-Concordium governance key formats beyond the current imported keypair JSON shape.

## Decisions

### Live chain parameters are the source of truth

The wallet SHALL not trust `governance-keys.json` as local authority metadata. Governance level and update-type authorization SHALL be derived on demand from live chain parameters queried from the selected node. This keeps list/remove/update behavior correct when governance keys have been rotated after genesis.

Alternatives considered:
- **Persist aggregate snapshot metadata locally**: simpler list implementation, but wrong in live environments once keys rotate.
- **Persist both snapshot and live state**: adds reconciliation complexity without improving correctness.

### Governance key storage is a separate per-network vault

Governance keys SHALL live in a separate encrypted vault scoped by `network_genesis_hash`, not inside the imported account vault. This keeps account signing and governance signing as distinct secret domains while still aligning both with network partition cleanup.

### Store encrypted raw key JSON only

The store SHALL encrypt the raw imported governance keypair JSON contents, including both public and private key material. The database SHALL not keep plaintext verify keys, derived governance levels, or live authorization metadata. This makes the governance vault opaque without the password and keeps the persistent model format-oriented rather than inference-oriented.

This implies that import duplicate detection, listing, and targeted removal by verify key all require vault unlock.

### Public key is the governance-key identity

Governance keys SHALL not have user labels. The wallet SHALL treat the public key (`verifyKey`) as the stable identity for matching and removal. This avoids parallel local naming for keys whose real authority and role are determined by the chain.

Because public keys are encrypted at rest, interactive removal SHALL happen only after governance vault unlock, when the CLI can decrypt stored key payloads and present a selector over the available public keys.

### Directory import is a convenience wrapper over single-file import

`--dir` SHALL scan a directory for recognized governance keypair files such as `root-key-*.json`, `level1-key-*.json`, and `level2-key-*.json`, while ignoring aggregate files such as `governance-keys.json`. The directory flow should unlock or create the governance vault once and then reuse it for all imported files.

### List is an unlock-and-query command, not a metadata listing

Because no governance key metadata is stored in plaintext, `governance keys list` SHALL:
1. resolve the target network,
2. fail actionably before prompting if no governance vault exists for that network,
3. unlock the governance vault,
4. decrypt and parse local key JSON payloads,
5. query live chain parameters,
6. match local public keys against live root/level1/level2 authorization structures,
7. render the derived result.

`governance keys list` SHOULD follow the CLI's normal context-bearing ergonomics and use the active network by default when one exists, while still allowing explicit network selection and prompted fallback.

This also allows the command to show useful states such as “stored locally but not currently authorized on-chain”.

### List output is tag-first and operator-oriented

Governance key rows SHALL be rendered in a tag-first format so operators can scan by authorization class before public key identity. Each row SHALL begin with one of `[level 2]`, `[level 1]`, `[root]`, or `[not authorized]` followed directly by the displayed verify key without extra alignment padding.

By default, `governance keys list` SHALL abbreviate verify keys to a compact form such as `1234...5678` for readability. The command SHALL also support an explicit `--show-full` option that renders the full verify key instead.

Rows SHALL be ordered for operational frequency rather than strict governance hierarchy:
1. `level 2`
2. `level 1`
3. `root`
4. `not authorized`

Authorized `root` and `level 1` keys SHALL render a short governance-key update summary:
- `root` → `update governance keys (root, level 1, level 2)`
- `level 1` → `update governance keys (level 1, level 2)`

Authorized `level 2` keys SHALL render a concise comma-separated summary of the update families they are currently authorized to sign, such as `protocol`, `create plt`, or `pool`.

Stored keys that are no longer authorized on-chain SHALL still be shown, but without a capability suffix.

### Interactive remove reuses list semantics with compact key display

Interactive `governance keys remove` SHALL reuse the same live-derived authorization semantics as `governance keys list` so operators can tell what they are deleting. The interactive picker SHALL:
- unlock the governance vault,
- query live chain parameters,
- match stored keys to current authorizations,
- render tag-first rows using the same authorization summaries as listing,
- abbreviate verify keys in the displayed rows to a compact form such as `1234...5678`, and
- support fuzzy multiselect so more than one governance key can be removed in one interaction.

### Account list rows can adopt the same bracket-first visual grammar

`account list` is also a human-oriented operator-facing command, so its rows MAY use the same bracket-first visual grammar as governance keys where that improves scanability. In this format, each account row begins with a bracketed ownership tag:
- `[<seed label>]` for seed-derived accounts
- `[imported]` for imported accounts

The bracketed ownership tag SHALL be followed by the account label and, when address display is enabled, the decrypted address in parentheses. Existing disambiguating metadata such as network, provider, identity index, and credential counter SHOULD remain available in a compact suffix so account rows remain useful both in normal listing and fuzzy selectors.

For scope resolution, `account list` SHALL default to all accounts on the resolved network rather than narrowing silently to the active seed. An explicit `--seed <LABEL>` filter remains available when the operator wants to narrow the result set. Because address display requires decrypting seed-scoped payloads, `account list --show-addresses` SHALL require an explicit `--seed <LABEL>` instead of silently using the active seed or prompting for one.

## Risks / Trade-offs

- **No plaintext index means more commands require a password** → This is intentional; keep prompts clear and accept the extra unlock step.
- **Duplicate detection requires decrypting existing keys** → Reuse the unlocked-vault scan during import; add tests for repeated imports and verify-key collisions.
- **Live chain query dependency for list** → Surface actionable node/network errors; this is preferable to presenting stale local metadata.
- **Directory import may encounter mixed files** → Ignore known aggregate snapshot files and fail actionably on malformed keypair files.
- **Future signing needs richer matching logic** → Keep the current data model minimal, but structure the in-memory matching code so `governance update` can reuse it later.

## Migration Plan

1. Add schema objects for governance vault metadata and encrypted governance key payloads.
2. Keep governance vault cleanup tied to `network reset` for the relevant `network_genesis_hash`.
3. Leave existing account/identity/seed vault data unchanged.
4. Add fresh-database and migration tests to ensure the new tables coexist with current wallet data.

Rollback is not expected to be automatic for local wallet data after schema migration. The change should be validated with migration tests and fresh initialization tests.

## Open Questions

- Whether explicit verify-key removal should remain required in non-interactive mode or whether a future short-hash form should be supported for easier scripting.
