## 1. CLI Surface and Resolution

- [x] 1.1 Add `IdentitySubcommand::Show`, `IdentitySubcommand::Export`, and corresponding argument structs.
- [x] 1.2 Make `identity show` accept an optional positional `[LABEL]` and optional `--network <LABEL>` filter.
- [x] 1.3 Make `identity export` accept a positional `<LABEL>` plus optional inline `--out <FILE>` destination selection.
- [x] 1.4 Reuse the existing identity ambiguity convention from `identity rename`: accept an explicit label, prompt interactively when multiple matches exist, and fail when the command cannot prompt instead of guessing.
- [x] 1.5 Ensure neither `identity show` nor `identity export` exposes a dedicated non-prompting mode yet.

## 2. Authentication and Decryption

- [x] 2.1 Resolve the owning key source for the selected identity.
- [x] 2.2 Prompt for the owning key-source password using existing CLI password prompt conventions.
- [x] 2.3 Unlock the signer-owner vault and decrypt the identity private payload.
- [x] 2.4 Surface actionable errors for unknown identities, ambiguous labels, wrong passwords, and missing private payloads.

## 3. Sensitive Show Flow

- [x] 3.1 Implement `identity show` as a temporary sensitive reveal view similar to `seed show`.
- [x] 3.2 Render the complete selected identity in that reveal view, including metadata, `code_uri`, and the issued identity object when present.
- [x] 3.3 Keep the human reveal view free of raw internal identifiers such as signer-owner id and network genesis hash.
- [x] 3.4 Hide the sensitive reveal view when the user presses any key or after 30 seconds, whichever happens first.
- [x] 3.5 Render the issued identity object in the reveal view as deterministic flattened key/value lines using `.` for nested object paths and `[index]` for arrays.
- [x] 3.6 Render pending identities clearly when no issued identity object is present yet.

## 4. Identity Export

- [x] 4.1 Implement `identity export` as explicit JSON file output and never as sensitive JSON printed to stdout.
- [x] 4.2 Support inline `--out <FILE>` and otherwise prompt for a destination path when prompting is available; fail instead of defaulting to stdout when prompting is unavailable.
- [x] 4.3 Export both pending and completed identities using a stable wallet-owned JSON schema.
- [x] 4.4 Emit top-level `version`, `identity`, `network`, `keySource`, and `privatePayload` fields using RFC 3339 UTC timestamps and without internal database identifiers.

## 5. Tests and Documentation

- [x] 5.1 Add resolution tests covering unique labels, ambiguous labels, omitted show labels, network-filtered show selection, and interactive selection behavior shared with rename-style identity lookup.
- [x] 5.2 Add authentication and decryption tests covering correct password, wrong password, pending identities, and completed identities.
- [x] 5.3 Add sensitive reveal-view tests covering completed and pending identities.
- [x] 5.4 Add export tests covering explicit destination requirements, pending identity export, and completed identity export.
- [x] 5.5 Update `docs/commands.md` to list `identity show` and `identity export` under implemented identity commands.
- [x] 5.6 Run Rust formatting and the relevant Cargo test suite.
