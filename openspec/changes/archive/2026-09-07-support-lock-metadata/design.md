## Context

See `proposal.md` for motivation. The standalone lock-create path and the compose submission path both construct `LockConfig` through the shared token helper, which currently always sets `metadata` to `None`. The pinned SDK represents configuration and query metadata as optional raw CBOR, while exposing `LockMetadata` for typed encoding and decoding. That model has optional `name` and `description` fields plus an extensibility map.

The compose REPL stores versioned TOML operations and reconstructs SDK operations only at submission time. Existing version-1 lock-create operations therefore need an additive representation that remains deserializable when metadata fields are absent.

## Goals / Non-Goals

**Goals:**

- Keep standalone and compose metadata encoding semantics identical.
- Preserve the distinction between no metadata payload and a metadata payload containing one or both recognized fields.
- Keep lock inspection useful even when another client has authored metadata this CLI does not recognize.
- Preserve existing composition plan compatibility without a plan-version migration.

**Non-Goals:**

- Exposing the SDK metadata model's arbitrary `additional` map.
- Accepting raw CBOR, JSON metadata documents, metadata files, or generic key/value inputs.
- Updating immutable metadata on an existing lock.
- Displaying arbitrary additional metadata fields during lock inspection.

## Decisions

### Use the SDK field name at the command boundary

The public options and prompts will use `name` and `description`, matching `LockMetadata` directly. This avoids a translation layer such as `--title` mapping to `name` and keeps help, plans, reviews, and inspection terminology consistent.

### Centralize constrained metadata construction

A shared token helper will normalize empty optional text to absence, return no raw metadata when both fields are absent, and otherwise construct:

```text
LockMetadata
├── name: supplied name or none
├── description: supplied description or none
└── additional: empty
```

The helper will encode the typed value with the SDK's raw-CBOR encoder. Both standalone and compose lock creation will pass their resolved name and description through this helper before constructing `LockConfig`.

Passing raw CBOR from command-specific code was rejected because it would duplicate encoding behavior and make it easier for the two authoring paths to diverge. Accepting a general `LockMetadata` input was rejected because it would make arbitrary additional fields part of the CLI's effective authoring surface.

### Prompt only for omitted fields and keep prompts empty-allowed

Standalone interactive creation and compose REPL lock creation will prompt independently for each omitted field. A supplied `--name` suppresses only the name prompt, for example. Empty or whitespace-only prompt responses become absent values. Non-interactive standalone creation never prompts and does not require metadata.

An initial “add metadata?” confirmation was considered, but direct empty-allowed prompts better satisfy the requirement that both fields be available as optional prompt inputs and avoid a separate metadata mode.

### Store optional fields directly on version-1 compose lock operations

`PlanOperation::LockCreate` will gain optional `name` and `description` fields with absent-field Serde defaults and omission during serialization. A representative saved operation is:

```toml
[[operations]]
type = "lock-create"
name = "Treasury vesting"
description = "Quarterly contributor allocation"
# existing lock fields remain unchanged
```

This representation mirrors the REPL options and keeps metadata constrained to the two supported fields. A nested arbitrary metadata map was rejected because it suggests extensibility the CLI intentionally does not provide. The change is additive, so `PLAN_VERSION` remains 1 and existing plans deserialize with both values absent.

All code paths that reconstruct or transform `LockCreate` operations must preserve the new fields, including account-reference resolution, preview, validation, and operation submission.

### Make inspection decoding tolerant

`token lock show` will attempt to decode `LockInfo.metadata` with the SDK's typed decoder. It will render recognized name and description fields when decoding succeeds, report `none` when no payload exists, and report an unrecognized payload when decoding fails. A decoding failure will not fail the command or suppress recipients, controller, or funds.

Decoded additional fields will not be rendered. This keeps inspection aligned with the supported model surface while remaining compatible with locks created by more extensible clients.

Failing the entire show command on metadata decoding was rejected because metadata is optional presentation data and raw on-chain values from other clients must not make core lock state uninspectable.

### Include metadata in every pre-submission review

Standalone lock creation will include resolved name and description values in its confirmation summary. Compose already uses the plan preview as its pre-submission review, so extending lock-create preview rendering supplies the equivalent protection for composed transactions.

## Risks / Trade-offs

- **[Two new prompts lengthen interactive lock creation]** → Keep both prompts empty-allowed, skip each prompt when its corresponding option is supplied, and do not prompt in non-interactive mode.
- **[Raw metadata from other clients may not match the typed SDK model]** → Treat decode failure as display-only degradation and continue rendering the lock.
- **[Adding fields to `LockCreate` touches several exhaustive matches]** → Cover parsing, plan round trips, account-reference transformation, preview, and operation construction with focused tests.
- **[Users may expect arbitrary SDK additional fields to be supported]** → Document that authoring and display intentionally cover only name and description; reject or avoid advertising generic metadata options.

## Migration Plan

No persisted-data or database migration is required. Existing composition plans remain at version 1 and deserialize with absent metadata fields. Rolling back the CLI leaves metadata already created on-chain intact; older builds can still inspect the rest of those locks, although they will not display the metadata or preserve the new fields when rewriting a manually edited plan containing them.
