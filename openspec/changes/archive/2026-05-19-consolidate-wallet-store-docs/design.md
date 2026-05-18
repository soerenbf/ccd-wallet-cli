## Context

The wallet database now spans seeds, identities, derived accounts, imported-account vaults, governance-key vaults, and wallet-local state, but the migration history is still expressed as a sequence of development-era schema additions. That makes the current baseline harder to understand because the authoritative schema is spread across multiple SQL files plus follow-up table rewrites, and it makes contributor documentation awkward because the docs would either have to describe migration history or restate the final schema independently.

This change is intentionally a development reset rather than a user-data migration project. The existing sqlite-store specification already allows older pre-reset development databases to be unsupported, which gives us room to replace the incremental migration chain with a single baseline schema that represents the current store model. Once that baseline exists, the documentation can describe the schema and encryption model as first-class design artifacts instead of reverse-engineering them from the migration sequence.

## Goals / Non-Goals

**Goals:**
- Define one baseline SQL schema file that creates the full current wallet store in a fresh database.
- Simplify migration runner logic so fresh databases initialise from the consolidated schema and already-current databases open without extra work.
- Reject older development databases with an actionable message telling contributors to recreate the local database.
- Add stable contributor docs for the database structure and encryption model, including Mermaid diagrams.
- Keep the documented schema aligned with the actual current store implementation in `seeds`, `identities`, `accounts`, and `governance` modules.

**Non-Goals:**
- Preserving in-place upgrade support for earlier development databases.
- Changing the logical storage model for seeds, identities, accounts, imported accounts, or governance keys.
- Introducing new runtime encryption primitives or changing password semantics.
- Documenting chain interaction flows beyond the storage boundaries needed to explain persisted data.

## Decisions

### Use a single consolidated baseline migration for the current schema
The migration set will be reduced to one SQL file that creates the full current schema in one pass and records the consolidated schema version. This reset baseline will return the schema version to `1`, treating the previous multi-step history as discarded development-only evolution rather than a long-term contract. This makes the database baseline explicit and gives future schema changes a clean point from which to add truly incremental migrations.

Alternatives considered:
- Keep the existing four migrations and document the final schema separately. Rejected because it preserves unnecessary development history and leaves contributors inferring the baseline from multiple steps.
- Add a new "consolidation" migration on top of the old chain. Rejected because it still treats transitional history as part of the long-term baseline and weakens the value of the reset.

### Treat pre-consolidation development databases as unsupported
Instead of translating versions 1-4 forward, the migration runner will fail early when it finds an older development schema version. The error should tell the user that the local development database must be recreated.

Alternatives considered:
- Continue supporting old versions in place. Rejected because the proposal explicitly frames consolidation as a development reset and the extra compatibility logic would outlive its value.
- Silently delete and recreate old databases. Rejected because destructive implicit recovery is risky and harder to trust.

### Document the store by current-state concerns, not by migration history
`docs/db-structure.md` should describe the tables, relationships, ownership boundaries, uniqueness rules, and cascade behavior of the current schema. `docs/encryption-model.md` should describe encryption domains, KEK/DEK envelope flow, plaintext-versus-ciphertext boundaries, AAD binding, and the detailed classification of encrypted data at rest. Both docs should use Mermaid diagrams so contributors can quickly orient themselves.

Alternatives considered:
- Put the explanation into code comments only. Rejected because the intended audience is contributors navigating the overall store model, not just readers of one module.
- Write one combined document. Rejected because schema structure and encryption model are related but distinct concepts that will be easier to maintain as separate references.

### Keep the documentation grounded in implemented storage modules
The docs should be derived from the actual store code and spec contracts rather than speculative future chain behavior. That means the documentation should explain the current persistent model and clearly treat future chain interaction work as out of scope.

Alternatives considered:
- Expand the docs to anticipated future chain flows. Rejected because it would go stale quickly and blur the difference between implemented storage and planned behavior.

## Risks / Trade-offs

- **Older local dev databases stop working** → Return a clear recreate-the-database error and treat this as an intentional development reset.
- **Baseline schema and docs drift apart over time** → Generate the docs from the consolidated current schema and reference the relevant store modules so future changes have a clear place to update.
- **Consolidation accidentally drops a current constraint or foreign key** → Validate the new baseline with migration tests that assert presence of all current tables, indexes, and cascade relationships.
- **Docs duplicate OpenSpec language awkwardly** → Use the docs for contributor-oriented explanation and keep normative behavior in specs.

## Migration Plan

1. Replace the incremental SQL chain with one baseline schema file representing the full current store.
2. Update `migrations.rs` to initialise fresh databases from that baseline and reject older pre-consolidation versions with an actionable error.
3. Update migration tests to assert the consolidated schema and unsupported-old-schema behavior.
4. Add `docs/db-structure.md` and `docs/encryption-model.md`, each with Mermaid diagrams and explicit scope notes.
5. Verify that the documentation matches the tables, constraints, and crypto flows used by the current storage modules.

Rollback for this change is straightforward during development: restore the prior migration files and migration runner logic. No production data migration or partial rollout support is planned.

## Open Questions

None at the moment.
