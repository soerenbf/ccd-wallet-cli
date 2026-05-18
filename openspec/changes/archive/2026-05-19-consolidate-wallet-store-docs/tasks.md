## 1. Consolidate the schema baseline

- [x] 1.1 Reset the consolidated baseline schema version to `1` and encode that versioning in the migration runner and schema SQL.
- [x] 1.2 Replace the current multi-file development migration chain with a single SQL schema file that creates the full current wallet store.
- [x] 1.3 Preserve the current tables, indexes, uniqueness constraints, and foreign-key cascades in the consolidated baseline schema.

## 2. Update migration execution and validation

- [x] 2.1 Simplify `crates/ccd-wallet-core/src/store/migrations.rs` to apply the consolidated baseline schema for fresh databases.
- [x] 2.2 Reject pre-consolidation development databases with an actionable recreate-the-database error instead of attempting in-place legacy migration.
- [x] 2.3 Update migration tests to verify the consolidated schema contents and the unsupported-older-database behavior.

## 3. Document the current store model

- [x] 3.1 Add `docs/db-structure.md` covering the current tables, relationships, ownership boundaries, uniqueness rules, and cascade behavior without duplicating detailed encrypted-at-rest classification.
- [x] 3.2 Add `docs/encryption-model.md` covering password domains, KEK/DEK envelope flow, plaintext-versus-encrypted data boundaries, encrypted-at-rest classification, and AAD binding.
- [x] 3.3 Add Mermaid diagrams to both documents and cross-check them against the implemented store modules and consolidated schema.
