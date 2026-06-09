## Context

Ledger key sources are stored as signer owners with `owner_kind = 'ledger'`. Enrollment writes signer-owner metadata, a local signer-owner vault, and a Ledger owner-details row containing public enrollment metadata. Recovered Ledger-owned identities and derived accounts reference the signer-owner id, and the database schema already cascades deletion from signer owners to the vault, Ledger details, identities, accounts, and their private payload rows.

The seed command family already has a destructive local cleanup flow in `seed delete`: resolve a label without silently using the active default, count owned identities/accounts, require typed confirmation, delete the owner, and clear active wallet state when needed. `ledger remove` should follow that user-safety model while using Ledger-specific terminology.

## Goals / Non-Goals

**Goals:**

- Add a Ledger-specific local removal command at `ccd-wallet ledger remove <LABEL>`.
- Restrict the target to enrolled Ledger key sources; seed key sources with the same label resolution path must not be removable through this command.
- Require explicit user confirmation by typing the target label before deleting local state.
- Reuse existing signer-owner deletion and cascade semantics instead of adding new storage behavior.
- Clear `active_key_source` when it points at the removed Ledger key source.
- Keep `docs/commands.md` in sync with the implemented command surface.

**Non-Goals:**

- Do not communicate with or modify the physical Ledger device during removal.
- Do not require the Ledger key-source local password for removal.
- Do not add a new database table, migration, or cascade mechanism.
- Do not remove seed key sources or imported accounts through `ledger remove`.
- Do not rename or alter the existing `seed delete` command.

## Decisions

1. **Expose the command as `ledger remove` rather than `ledger delete`.**
   - Rationale: removal is local unenrollment from this wallet, not deletion of anything from the physical Ledger device. The verb reduces the risk that users think the hardware wallet will be changed.
   - Alternative considered: `ledger delete` for symmetry with `seed delete`. This was rejected because Ledger device semantics are different and the user explicitly requested `ledger remove`.

2. **Use typed label confirmation for destructive removal.**
   - Rationale: deleting the signer owner cascades to Ledger-owned identities and derived accounts. Mirroring seed deletion gives users a familiar guardrail and avoids accidental cleanup from a mistyped command or selector selection.
   - Alternative considered: a yes/no confirmation. This is weaker for destructive operations that can delete multiple child records.

3. **Do not require a connected Ledger device or local password.**
   - Rationale: the command removes local wallet metadata and does not sign, derive, decrypt payloads, or prove device possession. Requiring a device would make cleanup impossible when the user no longer has the Ledger available.
   - Alternative considered: verify the connected Ledger before removal. This would improve possession assurance but conflicts with stale/lost-device cleanup.

4. **Reuse `signer_owners::delete_by_id` and existing cascades.**
   - Rationale: signer-owner storage already defines deletion as cascading to the vault, owner-kind detail row, identities, derived accounts, and encrypted payloads. Reusing it avoids duplicating storage logic and keeps behavior consistent across signer-owner kinds.
   - Alternative considered: manually delete Ledger detail rows and child records. This is more error-prone and would duplicate schema-level behavior.

5. **Resolve omitted labels with a Ledger-only selector and no active default deletion.**
   - Rationale: destructive commands should not silently target the active key source. Interactive omission can open a selector, while `--non-interactive` must require an explicit label.
   - Alternative considered: default to `active_key_source` when it is a Ledger owner. This was rejected to match seed deletion safety semantics.

## Risks / Trade-offs

- **Users may expect removal to require device verification** → Command text and docs will state that removal affects local wallet state only and does not modify the Ledger device.
- **Accidental removal of recovered local state** → The command will display identity/account counts and require exact typed label confirmation before deletion.
- **Active key source may become stale** → The command will clear `active_key_source` when it matches the removed label.
- **Command symmetry with `seed delete` is imperfect** → The Ledger-specific verb better communicates local unenrollment and matches the requested command.

## Migration Plan

No database migration is needed. Rollback is limited to removing the CLI command and documentation/spec changes; existing persisted data remains compatible because the command uses current signer-owner deletion semantics.

## Open Questions

None.
