# Design

## Context

See proposal.md for motivation. Cargo.lock pins SDK commit `ecde0d177f10edf78d4b959c70a314826096dd9c` on `release/p11`. Its Unreleased changelog and source are the API baseline; older changelog bullets still mention MetaUpdate but do not describe the current API.

Affected flows converge in `commands/token/shared.rs`, `lock.rs`, and `compose.rs`; command definitions live in `cli.rs` and dispatch in `commands/token/mod.rs`. Transaction rendering fixtures still construct MetaUpdate events. `crates/ccd-wallet-ledger/src/sdk.rs` also constructs TokenUpdate payloads and must be checked against the new payload representation.

## Goals / Non-Goals

**Goals:** Migrate existing flows in place, preserve validation and signing behavior, and retain saved-plan semantics with minimal compatibility code.

**Non-Goals:** No generic transaction abstraction, dual SDK support, legacy MetaUpdate wire fallback, new lock configuration variants, database migration, or unrelated P11 features such as genesis construction. No new dependencies or TypeScript work unless implementation reveals a directly affected public interface.

## Decisions

### Use the SDK's scoped and unscoped split directly

Standalone single-token commands use `token_operations` and scoped TokenUpdate construction. Composer always uses `operations::Operations` and `construct::operations` for one unscoped TokenUpdate, even for a one-token plan. Lock workflows use the current lock client helpers, including `ReleaseTokens` and `release_funds`. Preserve order and same-plan lock-ID resolution.

Alternative: choose scoped versus unscoped dynamically for each composition. Rejected: adds branching without user benefit and complicates mixed token/lock plans.

Audit all construction and signing call sites, including the Ledger SDK adapter, rather than fixing only compiler errors in composer. Keep byte serialization delegated to SDK APIs and preserve the existing signing route.

### Rename release end to end; compatibility lives only in saved-plan parsing

Rename CLI argument types, dispatch, prompts, completion, prepared operations, validation, grant capability parsing/rendering, and plan variants to release. New CLI and REPL input reject `return`; help and errors identify `release`.

Keep version 1. Add deserialization aliases for the actual existing serialized lock-return tag and normalize `return` capabilities while loading saved structured grants. Do not widen shared CLI capability parsing to accept legacy names. Save canonical release tags and capabilities through the existing atomic save path. Loading and previewing never rewrite files.

Alternative: drop legacy plan support or bump the plan version. Rejected: spelling changes do not justify invalidating plans or adding a migration framework. Compatibility is backward-reading only; older wallet builds are not promised to read newly saved release spellings.

### Adapt configuration and metadata at shared boundaries

Build `LockConfig::SimpleV0(LockConfigSimpleV0 { ... })` in the existing shared constructor. Read lock fields through `LockInfo.config` and its tagged configuration in both rendering and selectors/validation; preserve recipients, grants, expiry, tokens, keep-alive, memo, and constrained lock metadata behavior.

Build `TokenMetadataUrlDetails` for update inputs using URL and optional checksum. Keep token state inspection's richer `MetadataUrl` representation separate; do not strip queried additional fields merely because update inputs became narrower.

Alternative: flatten SDK configuration into a new local model. Rejected: duplicates the SDK and increases conversion surface.

### Render unified events with per-event compatibility

Replace MetaUpdate handling and fixtures with TokenUpdate `Upward<OperationEvent>` events. Retain known transfer and lock lifecycle one-line summaries, lock context annotations, and event order. Unknown events receive explicit fallback entries preserving available raw details; they must not suppress known sibling events. Reuse existing fallback formatting where available rather than inventing another event schema.

Alternative: pretty-print the entire event array. Rejected: regresses existing concise output and makes mixed events less readable.

## Risks / Trade-offs

- Updated wire transactions require a compatible node → document P11 compatibility; propagate node rejection without a legacy fallback.
- Saved tags and grant strings can be missed by a symbol rename → regression fixtures must exercise actual old version-1 TOML for both operation and capability spellings.
- Configuration moves affect selectors as well as rendering → inspect every lock configuration consumer and cover limited/any-recipient paths and metadata-free locks.
- Event changes can hide future protocol data → test unknown events between known events.
- Ledger payload conversion can retain an outdated TokenUpdate shape → run all-feature workspace checks and adapter serialization tests; never hand-encode replacement transaction bytes.

## Migration Plan

1. Keep the user's dependency update unchanged and establish migration scope against the pinned SDK.
2. Migrate shared types, lock flows, composer, signing integrations, and rendering together; add regression checks as each path changes.
3. Update `docs/commands.md` and help to canonical release and current TokenUpdate terminology in the same change.
4. Run formatting, all-feature workspace checks/tests, and clippy. No local data migration is needed.

Rollback requires restoring a mutually compatible wallet and SDK through the user's normal Git workflow. Original saved plans remain readable by old builds only while they retain their old spellings; plans saved with canonical release spellings may require manual conversion for an old wallet.
