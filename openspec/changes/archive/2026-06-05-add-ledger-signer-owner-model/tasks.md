## 1. Store Schema and Core Owner Types

- [x] 1.1 Replace the baseline SQLite schema with signer-owner tables: `signer_owners`, `signer_owner_vaults`, `seed_owner_secrets`, and `ledger_owner_details`.
- [x] 1.2 Update identity and account tables to reference `signer_owner_id` for signer-owned rows and derived-account uniqueness.
- [x] 1.3 Rename or replace derived account private payload storage so derived payloads are clearly separate from imported account payloads.
- [x] 1.4 Add SQLite constraints, partial indexes, foreign keys, and cascades for signer-owner ownership, imported account ownership, labels, and derivation tuples.
- [x] 1.5 Update migration/schema tests to verify the clean-slate signer-owner schema, uniqueness rules, and cascade behavior.

## 2. Signer Owner Store APIs

- [x] 2.1 Add store types for signer owners, signer owner kinds, signer owner vaults, seed owner details, Ledger owner details, and unlocked signer-owner contexts.
- [x] 2.2 Implement signer-owner create/list/find/rename/delete operations with global signer-owner label uniqueness.
- [x] 2.3 Implement signer-owner vault creation, unlock, and password-change behavior using the existing Argon2id and ChaCha20-Poly1305 primitives.
- [x] 2.4 Implement seed owner secret encryption/decryption under the signer-owner DEK.
- [x] 2.5 Implement Ledger owner detail insertion and lookup by canonical public key and signer owner id.
- [x] 2.6 Add unit tests for signer-owner vault unlock, wrong-password rejection, independent owner domains, owner-kind detail invariants, and deletion cascades.

## 3. Seed Command and Seed Storage Adaptation

- [x] 3.1 Update seed add/import/generate flows to create seed-kind signer owners, signer-owner vaults, and seed owner secret payloads.
- [x] 3.2 Update seed unlock flows to return signer-owner unlock contexts plus decrypted seed secret material.
- [x] 3.3 Update seed list/rename/delete flows to operate on seed-kind signer owners while preserving seed command UX.
- [x] 3.4 Replace active-seed state usage where necessary with active signer-owner state internally, while preserving or introducing `active key source` as the user-facing concept.
- [x] 3.5 Update seed command tests to cover signer-owner-backed seed behavior and label conflicts with Ledger owners.

## 4. Ledger Signer Owner Enrollment

- [x] 4.1 Define the canonical Ledger owner enrollment path and helper for deriving the display fingerprint from the canonical public key.
- [x] 4.2 Add a separate Ledger setup/enrollment CLI flow using `ccd-wallet-ledger` public-key retrieval and local signer-owner vault creation without folding Ledger setup into the seed command family.
- [x] 4.3 Add Ledger owner recognition logic that matches a connected Ledger by canonical public key rather than transport identity.
- [x] 4.4 Reject duplicate Ledger enrollment when the canonical public key is already stored.
- [x] 4.5 Add tests or mock-transport coverage for enrollment, duplicate detection, owner matching, and owner mismatch errors.

## 5. Identity Storage and Identity Issuance

- [x] 5.1 Update identity store records, insert/find/list/rename/next-index APIs, and private payload AAD from `seed_id` to `signer_owner_id`.
- [x] 5.2 Update identity private payload encryption/decryption to use signer-owner DEKs.
- [x] 5.3 Update seed-backed identity issuance to use signer-owner APIs while preserving current behavior.
- [x] 5.4 Add a Ledger identity construction module that maps identity issuance inputs to supported Ledger app operations and returns an unsupported-flow error before storage mutation when the flow cannot be represented.
- [x] 5.5 Wire Ledger-backed identity issuance through the construction module, connected-Ledger owner matching, and Ledger signer-owner DEK storage for pending/done identity payloads.
- [x] 5.6 Update pending identity lazy confirmation and recovered identity import flows to use signer-owner tuples.
- [x] 5.7 Add tests for seed-owned and Ledger-owned identity uniqueness, indexing, payload encryption, listing, and owner deletion cascades.

## 6. Account Storage, Creation, and Signing Sources

- [x] 6.1 Update account store records and queries so derived accounts reference `signer_owner_id` and imported accounts remain imported-vault backed.
- [x] 6.2 Update derived account private payload encryption/decryption and AAD to bind to signer-owner context.
- [x] 6.3 Update account creation for seed-backed identities to use signer-owner APIs while preserving current behavior.
- [x] 6.4 Add a Ledger account construction module that prepares credential deployment signing requests from wallet/account inputs and fails safely for unsupported Ledger credential flows.
- [x] 6.5 Wire Ledger-derived account creation through the construction module, connected-Ledger owner matching, and Ledger signer-owner DEK storage for account address payloads.
- [x] 6.6 Update account signing-source resolution to return seed-backed derived, Ledger-backed derived, or imported signing sources.
- [x] 6.7 Add Ledger account transaction signing routing for transaction, token, contract, and connect flows with fail-safe user rejection, device mismatch, and unsupported-command behavior.
- [x] 6.8 Add tests for derived tuple uniqueness, cross-source label uniqueness, payload encryption, signing-source routing, construction-layer request staging, and failure-safe Ledger signing behavior.

## 7. Listing, Selection, Reset, and UX Integration

- [x] 7.1 Update identity and account list scope resolution from seed scope to internal signer-owner scope while exposing `key source` as the user-facing scope term where appropriate.
- [x] 7.2 Render seed-backed and Ledger-backed identity/account rows with clear key-source context without decrypting private payloads.
- [x] 7.3 Update address reveal flows to unlock the appropriate signer-owner or imported vault domains only when explicitly requested.
- [x] 7.4 Update fuzzy selectors and rename flows to include seed and Ledger owner context and preserve network-wide label uniqueness.
- [x] 7.5 Update network reset to delete network-scoped identities/accounts and network-scoped imported/governance vaults while preserving signer owners and signer-owner vaults.
- [x] 7.6 Add tests for owner-scoped listing, imported account inclusion, address reveal prompts, rename selection, and network reset preservation of signer owners.

## 8. Documentation and Validation

- [x] 8.1 Update `docs/db-structure.md` to describe signer-owner tables, identity/account ownership, imported vault separation, uniqueness rules, and cascade behavior.
- [x] 8.2 Update `docs/encryption-model.md` to describe signer-owner password domains, seed owner secrets, Ledger owner details, derived account payload encryption, and AAD binding.
- [x] 8.3 Update `docs/commands.md` for any command-surface changes introduced for Ledger enrollment, key-source selection, and the separate Ledger setup flow.
- [x] 8.4 Update README or command examples to explain seed-backed versus Ledger-backed key sources at a user-facing level.
- [x] 8.5 Run `cargo fmt`, targeted `cargo test`, and relevant workspace validation for the Rust changes.
- [x] 8.6 Run `OPENSPEC_TELEMETRY=0 openspec validate add-ledger-signer-owner-model --strict` and fix any spec issues before implementation is considered complete.
