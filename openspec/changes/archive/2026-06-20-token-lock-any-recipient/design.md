## Context

The pinned Concordium Rust SDK changed lock recipients from a plain account list to a tagged `LockRecipients` value with `Any` and `Limited(Vec<...>)` variants. `ccd-wallet` currently assumes every lock stores a concrete recipient list across direct token-lock commands, lock rendering, and token composition plan parsing and validation. That mismatch now causes compilation failures and leaves no user-facing way to author or inspect any-recipient locks.

The affected behavior spans multiple command paths:
- direct `token lock create`, `token lock send`, and `token lock show`
- shared lock rendering and lock-config construction helpers
- token compose parsing, validation, prompting, plan persistence, and preview/help text

The project already has stable user-facing command taxonomy and saved compose plans, so the change must fit existing flows without introducing ambiguous account-reference syntax.

## Goals / Non-Goals

**Goals:**
- Represent protocol lock recipients as either any-recipient or limited-recipient throughout the CLI.
- Add an unambiguous direct CLI surface for any-recipient lock creation.
- Preserve existing limited-recipient compose plans while adding a clear serialized representation for any-recipient plans.
- Keep interactive flows aligned with existing cliclack-based prompting patterns.
- Update validation and rendering so any-recipient locks behave correctly in show, confirmation, and compose lock-send flows.

**Non-Goals:**
- Changing the overall `token` command taxonomy.
- Introducing new lock recipient variants beyond the SDK's `Any` and `Limited` cases.
- Broadly redesigning compose plan syntax outside the lock-recipient field.
- Changing lock fund, return, or cancel semantics.

## Decisions

### Use an explicit `--any-recipient` flag for direct CLI lock creation
The direct `token lock create` command will accept an explicit `--any-recipient` flag that is mutually exclusive with repeated `--recipient` values.

Rationale:
- Avoids overloading a string such as `any`, which could collide with valid local account labels.
- Maps clearly to the SDK enum shape.
- Keeps limited-recipient syntax unchanged for existing users.

Alternatives considered:
- Treating `--recipient any` as special text. Rejected because it is ambiguous with local account labels and plan text values.
- Adding a more general `--recipient-mode` enum. Rejected as unnecessary surface area for a two-variant protocol type.

### Use `recipients = "any"` in compose plans
Saved compose plans will support `recipients = "any"` for any-recipient locks while retaining existing `recipients = ["..."]` arrays for limited-recipient locks.

Rationale:
- Matches the conceptual field name already used in plans.
- Preserves backward compatibility for existing array-based plans.
- Keeps the serialized model compact and easy to read.

Alternatives considered:
- Adding a parallel `recipient_mode` field. Rejected because it duplicates state and complicates migration for a simple two-form value.
- Encoding any-recipient as an empty array. Rejected because empty limited-recipient locks have different semantics from explicit any-recipient locks in the protocol model.

### Prompt for recipient mode in interactive lock creation
When interactive lock creation lacks both explicit recipients and `--any-recipient`, the CLI will first prompt for recipient mode and only ask for account recipients when the user selects limited recipients.

Rationale:
- Mirrors the real protocol choice before collecting variant-specific data.
- Avoids awkward prompt cancellation and backtracking.
- Keeps interactive behavior consistent between direct commands and compose flows.

Alternatives considered:
- Prompting for recipients first and offering a special sentinel input. Rejected because it mixes variant selection with account entry and weakens autocomplete behavior.

### Treat any-recipient lock sends as free-form recipient entry
For lock-send flows, limited-recipient locks will keep the existing recipient-selection and membership-validation behavior. Any-recipient locks will instead accept any explicit recipient and, when missing interactively, will prompt with free-form account-reference input rather than a fixed selector.

Rationale:
- There is no finite configured recipient set to select from for any-recipient locks.
- Preserves strict validation for limited-recipient locks.
- Reuses existing shared account-reference prompting for free-form recipient entry.

Alternatives considered:
- Skipping interactive prompting for any-recipient sends and requiring explicit `--recipient`. Rejected because it would make any-recipient flows less ergonomic than current compose behavior.

### Render `Any` as `any eligible account`
Human-readable lock rendering and confirmation summaries will describe the any-recipient variant as `any eligible account`.

Rationale:
- Matches the protocol intent more clearly than a bare `any` label.
- Reads well in both `token lock show` output and confirmation summaries.

## Risks / Trade-offs

- **[Plan format dual shape]** Supporting both string and array forms for `recipients` adds parser branching. → Mitigation: keep the branching localized to compose plan serialization and validation helpers, and add round-trip tests for both forms.
- **[Interactive flow divergence]** Any-recipient lock-send prompts differ from limited-recipient lock-send selectors. → Mitigation: document the distinction in compose help text and test both prompt paths.
- **[CLI surface growth]** Adding `--any-recipient` slightly expands the lock-create API. → Mitigation: make it mutually exclusive with `--recipient` and keep naming explicit and discoverable in help output.

## Migration Plan

- Existing limited-recipient compose plans remain valid without modification because array-valued `recipients` keep their current meaning.
- New any-recipient plans serialize as `recipients = "any"`.
- Rollback is straightforward before release because no persisted wallet-state migration is involved; reverting the code leaves existing array-based plans untouched, while any string-based plans would simply require re-authoring under the reverted build.

## Open Questions

- None at proposal time.
