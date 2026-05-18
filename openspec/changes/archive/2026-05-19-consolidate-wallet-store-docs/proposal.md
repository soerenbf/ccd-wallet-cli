## Why

The wallet store has reached a stable-enough shape that contributors need a clear description of the current SQLite schema and encryption model before more chain-facing work lands. Right now that understanding is split across multiple incremental migrations and storage modules, which makes the baseline harder to reason about and harder to document accurately.

## What Changes

- Consolidate the current development-only wallet SQLite schema into a single baseline migration that represents the full current store shape.
- Treat older pre-consolidation development databases as unsupported and fail with an actionable recreate-the-database error instead of preserving in-place migration history.
- Add `docs/db-structure.md` describing the current wallet store tables, relationships, ownership boundaries, and cascade behavior.
- Add `docs/encryption-model.md` describing the password domains, envelope-encryption flow, plaintext-vs-encrypted storage split, and AAD binding strategy.
- Include Mermaid diagrams in both documents so the schema and encryption model are easy to inspect visually.

## Capabilities

### New Capabilities
<!-- None. -->

### Modified Capabilities
- `sqlite-store`: consolidate the migration set into a single baseline schema and explicitly reject older pre-reset development databases.

## Impact

- Affects `crates/ccd-wallet-core/src/store/migrations.rs` and the SQL files under `crates/ccd-wallet-core/src/store/migrations/`.
- May require migration-related test updates to reflect the new baseline and unsupported older development schemas.
- Adds contributor-facing documentation under `docs/` for the local wallet database structure and encryption model.
- Does not change the intended wallet storage model for seeds, identities, accounts, imported account vaults, or governance key vaults; it documents and consolidates the existing model.
