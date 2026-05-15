## Context

Seed phrases can be added under stable labels, and active network state already uses the `wallet_state` table. The next step is making seeds convenient to reference by allowing one seed to be selected as the default active seed.

`seed show` should be an explicit secret-reveal command: it resolves a seed, prompts for that seed's password, unlocks only that seed, and displays the decrypted seed phrase.

## Goals / Non-Goals

**Goals:**
- Add `ccd-wallet seed use <LABEL>`.
- Persist active seed selection using `wallet_state` key `active_seed`.
- Validate that the selected seed exists before persisting it.
- Add `ccd-wallet seed show [LABEL]`.
- If no label is supplied to `seed show`, resolve and show the active seed.
- Prompt for the selected seed's password before revealing the seed phrase.
- Display the decrypted seed phrase only after successful password authentication.
- Avoid prompting for passwords for `seed use`.

**Non-Goals:**
- Exporting seed phrases to files.
- Long-lived seed unlock sessions.
- Listing all seeds.
- Deriving accounts from the active seed.
- Schema migration; `wallet_state` already supports arbitrary key/value state.

## Decisions

### D1: Store active seed as `wallet_state.active_seed`

**Decision**: Store the active seed label as key `active_seed` in the existing `wallet_state` table.

**Rationale**: This mirrors `active_network`, requires no schema change, and keeps mutable selection state separate from seed metadata.

### D2: Active seed references seed label, not seed ID

**Decision**: Persist the seed label as the active seed value.

**Rationale**: Labels are the user-facing handles used in CLI commands. Storing the label keeps error messages and debugging straightforward.

**Alternative considered**: Store seed UUID. Rejected for now because no seed rename command exists and labels are unique/stable enough for the current CLI. If renaming is added later, that change can update the active label in the same transaction.

### D3: `seed show` temporarily reveals the seed phrase after password authentication

**Decision**: `seed show [LABEL]` prompts for the selected seed's password, unlocks that seed with `store::seeds::unlock`, enters the terminal alternate screen, displays the decrypted seed phrase, then hides it when either the user presses any key or 30 seconds elapse — whichever happens first. The command clears the alternate screen before returning to the normal terminal.

**Rationale**: The user explicitly asked for `show` to reveal the phrase, but normal stdout would leave the secret in terminal scrollback. Alternate-screen display with an any-key/timeout hide behavior gives the user time to copy the phrase while reducing accidental persistence in the terminal.

Example flow:

```text
$ ccd-wallet seed show main_seed
Password for seed 'main_seed':

[alternate screen]
Seed phrase for 'main_seed':
abandon abandon ... about

Copy this now. Press any key to hide. It will hide automatically in 30 seconds.
[/alternate screen clears and exits]
```

Wrong passwords fail before printing any seed phrase bytes.

### D4: Missing label means active seed

**Decision**: `ccd-wallet seed show` resolves `wallet_state.active_seed` and shows that seed. If no active seed is configured, it exits with an actionable error.

**Rationale**: This matches the active network pattern and creates a convenient default for future commands.

## Risks / Trade-offs

- **Secret exposure in terminal**: `seed show` intentionally reveals the seed phrase. Mitigation: require an explicit command and password prompt, render in the alternate screen, clear on keypress or after 30 seconds, and document residual risks such as terminal logging, screenshots, tmux/screen behavior, and clipboard history.
- **Storing active seed by label**: A future rename command must update `active_seed`. Mitigation: there is no rename command yet; capture this when rename is proposed.
- **Stale active seed**: Manual DB edits could leave `active_seed` pointing to a missing label. Mitigation: `seed show` validates the label exists before prompting for a password and reports an actionable stale-state error.

## Migration Plan

No migration required. The `wallet_state` table already exists and can store the `active_seed` key.

## Open Questions

None.
