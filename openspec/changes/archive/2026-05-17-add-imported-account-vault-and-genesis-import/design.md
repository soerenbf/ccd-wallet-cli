## Context

The current account model assumes every wallet-managed account is derived from a stored seed and identity. Account rows require `seed_id`, `ip_identity`, `identity_index`, and `credential_counter`; encrypted account payloads are decrypted with the owning seed's DEK and currently contain the account address.

Genesis account JSON bundles from Concordium node test-run data contain fully materialized account secret material, including signing keys, credential data, account address, and encryption keys. These accounts are useful on localhost/private chains that may not have identity providers, but they are not tied to a local seed or identity. Browser-wallet account exports are a future import source with the same broad shape: externally supplied account secret material that should become a wallet-visible account capable of signing transactions.

## Goals / Non-Goals

**Goals:**
- Represent wallet accounts as source-aware entries backed by either seed derivation or imported secret material.
- Store imported account secret material in an imported accounts vault scoped by network genesis hash.
- Create the per-network imported vault implicitly on first import.
- Import one genesis account JSON file at a time.
- Require account labels for imports, prompt in interactive mode when absent, and suggest the filename stem.
- Enforce network-wide account label uniqueness across both derived and imported accounts.
- Keep imported account addresses encrypted and hidden by default.
- Provide a source-aware signing-material resolution shape so future transaction commands can sign with imported accounts.

**Non-Goals:**
- Directory/bulk import; a future `--dir` option can add that without changing the storage model.
- Browser-wallet export import UX; the storage shape should be compatible, but this change only implements genesis JSON import.
- Reworking identity issuance or seed recovery.
- Adding new transaction commands beyond the signing-source plumbing needed by this change.

## Decisions

### Imported accounts use one vault per network genesis hash

Imported accounts SHALL be protected by a vault keyed to `network_genesis_hash`, created on first import for that network. This matches the operational model for localhost/private chains: unlock imported account material once for the network and use any imported account on that chain.

Alternatives considered:
- **Per-account vaults**: simpler local isolation, but inconvenient for genesis account sets and repeated signing.
- **Per-import-session vaults**: preserves provenance but creates unnecessary fragmentation when all accounts belong to the same chain.
- **Seed-like synthetic owner**: reuses existing patterns but misrepresents imported accounts as mnemonic-backed.

### Account source is structural, not encoded in labels

Account rows SHALL carry a source kind such as `derived` or `imported`. Labels remain user-facing names and MUST be unique across all accounts on a network regardless of source. This avoids ambiguous signing resolution and avoids requiring label prefixes such as `genesis_` or `imp_`.

### Derived coordinates become source-specific metadata

The current derivation tuple remains required for derived accounts but not for imported accounts. Implementation may either make existing derivation columns nullable with source-kind constraints, or split source metadata into side tables. The chosen implementation MUST preserve existing derived-account uniqueness and account-label uniqueness.

A side-table model is preferred if the migration remains practical:

```text
accounts
├─ id
├─ network_genesis_hash
├─ label
├─ status
├─ source_kind
└─ timestamps

account_derived_sources
└─ account_id, seed_id, ip_identity, identity_index, credential_counter

account_imported_sources
└─ account_id, imported_vault_id, import_kind, source_metadata_json
```

This avoids null-heavy generic account rows and makes signing resolution explicit.

### Imported secret payload uses an internal format independent of genesis JSON

Genesis JSON is an input format, not the storage contract. The store SHALL convert it into an internal imported account secret payload containing the address and signing material required for transactions, with room for credential/encryption material needed by future account operations. Source metadata can preserve import kind and original filename for diagnostics.

### Address privacy remains consistent

Imported account addresses SHALL not be stored in plaintext account metadata. They SHALL be revealed only when the user explicitly requests address display, at which point the imported vault for that network is unlocked instead of a seed.

### Import command starts single-file only

The first CLI surface imports one genesis account file and one explicit/resolved network. Directory import is intentionally deferred; this keeps label prompting and collision handling simple and leaves bulk behavior to a future `--dir` option.

## Risks / Trade-offs

- **Schema migration complexity** → Keep the migration explicit and add store-level tests covering existing derived accounts, imported accounts, cascade behavior, and uniqueness constraints.
- **Signing API over-generalization** → Introduce only the source-aware resolver and imported signing payload shape needed for imported accounts; defer transaction command UX.
- **Genesis JSON shape drift** → Parse the known Concordium genesis account bundle format with actionable errors, and keep source parsing isolated from storage.
- **Vault password confusion** → Prompt copy should identify the vault as the imported accounts vault for the resolved network/genesis hash, distinct from seed passwords.
- **Future browser-wallet import mismatch** → Store an internal imported secret representation rather than raw genesis JSON, while allowing source metadata to record origin.

## Migration Plan

1. Add schema objects for source-aware accounts and imported vaults while preserving existing account rows as derived accounts.
2. Migrate existing rows into the derived-account source representation.
3. Preserve `UNIQUE(network_genesis_hash, label)` across all account rows.
4. Keep seed deletion cascading only derived accounts owned by that seed; imported accounts remain tied to their network partition and imported vault.
5. Keep network reset/prune deleting all accounts for the network partition, including imported accounts and their vault-backed payloads.

Rollback is not expected to be automatic after migrating local wallet data. Tests should validate fresh initialization and migration from the current schema.

## Open Questions

- Exact internal imported secret payload fields needed for the first signing use case beyond account signing keys and address.
- Whether imported vault passwords should be changeable in this change or deferred to later vault-management work.
