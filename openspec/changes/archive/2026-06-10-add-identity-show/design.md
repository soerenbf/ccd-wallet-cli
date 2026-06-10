## Context

The wallet already supports identity issuance, listing, and renaming, but it does not provide a direct way to inspect one stored identity in depth. That leaves a gap between the two existing identity views:

- `identity list` is intentionally metadata-only and works without unlocking private payloads.
- The encrypted identity private payload already contains the issuance `code_uri` and, once available, the full issued identity object.

Users therefore have no first-class CLI path to inspect pending issuance state, verify which key source owns a stored identity, or view the decrypted issued identity object for troubleshooting and recovery-oriented workflows.

This change is local-state inspection and export rather than on-chain inspection. The relevant data already exists in SQLite and is encrypted under the owning signer-owner password domain, so the new commands need to resolve the identity, authenticate against the owning key source, decrypt the stored private payload, and then either reveal it safely to a human or export it deliberately to a JSON file.

## Goals / Non-Goals

**Goals:**
- Add `ccd-wallet identity show <LABEL>` for detailed inspection of a stored identity.
- Add `ccd-wallet identity export <LABEL> [--out <FILE>]` for deliberate JSON export of a stored identity.
- Reuse the existing identity ambiguity convention from `identity rename` so label resolution feels consistent.
- Require authentication against the owning key-source password domain before revealing or exporting private identity data.
- Treat the complete human show flow as sensitive output and reveal it in a temporary terminal view.
- Keep machine-readable output off normal stdout and require an explicit file destination for export.
- Support both pending and completed identities.

**Non-Goals:**
- Adding a new non-prompting password input mechanism in this change.
- Redesigning `identity list` to reveal private data.
- Changing identity storage format, encryption model, or issuance protocol behavior.
- Adding mutation flows such as retry, resume, or delete to `identity show` or `identity export`.
- Defining a protocol-standard or portable identity interchange format beyond a wallet-owned JSON export.

## Decisions

### 1. The command surface splits inspection from export

The identity command space will separate human inspection from structured extraction:
- `identity show <LABEL>` is the human inspection command.
- `identity export <LABEL> --out <FILE>` is the structured extraction command.

**Rationale:** These are different intentions with different safety properties. `show` should optimize for reducing accidental persistence, while `export` should make persistence explicit and deliberate.

**Alternatives considered:**
- **One `identity show` command with `--json`:** rejected because printing sensitive JSON to stdout conflicts with the privacy posture of the human show flow.

### 2. `identity show` is a password-gated sensitive reveal command

`identity show` will always authenticate the user against the owning key source before revealing any selected identity details.

After successful authentication, the command will reveal the complete selected identity in a temporary terminal view similar to `seed show`. That reveal view should cover the full human-facing inspection content rather than splitting some of it into normal scrollback-visible output.

The reveal view will include:
- identity metadata: label, status, network, key source, provider id, identity index, creation time, and expiry when known;
- decrypted `code_uri` only for pending identities;
- the issued identity object when present.

The human view will not include raw internal identifiers such as signer-owner id or network genesis hash.

**Rationale:** The selected identity as a whole is sensitive enough that the cleanest boundary is to reveal all of it in the sensitive view, not just the identity object. Once an identity is completed, the stored `code_uri` is no longer useful and should be discarded so completed and recovered identities follow the same model.

### 3. The issued identity object uses flattened line-oriented key/value rendering in the reveal view

Within the sensitive reveal view, the issued identity object should be rendered as line-by-line key/value output rather than raw JSON.

The renderer should flatten nested values deterministically:
- recurse depth-first;
- visit object keys in sorted key order;
- visit array elements in index order;
- join nested object paths with `.`;
- render array positions as `[index]`;
- render scalar leaf values on a single line;
- render empty objects as `{}` and empty arrays as `[]` at their path.

The human reveal view should use an explicit allowlist and show only user-facing identity attributes from `value.attributeList`. Visible identity-object paths are limited to `value.attributeList.chosenAttributes.*`, `value.attributeList.createdAt`, `value.attributeList.maxAccounts`, and `value.attributeList.validTo`. The renderer should display only the final key segment in the human view while using the full path internally for filtering. This filtering is only for the human reveal view; `identity export` remains the exact structured representation and preserves the full identity object JSON.

Examples of the intended shape include `countryOfResidence: DK`, `firstName: Ledger`, `createdAt: 202606`, `maxAccounts: 200`, and `validTo: 202706`.

**Rationale:** This keeps the human inspection view readable, deterministic, and easy to scan while reserving exact structured JSON for the export path.

### 4. `identity export` writes JSON only to an explicit file destination

`identity export` will authenticate the user against the owning key source, decrypt the identity private payload, and write a wallet-owned JSON file to an explicit destination.

If `--out <FILE>` is supplied, the command uses that path directly. If `--out` is omitted and prompting is available, the command prompts for a destination path instead of defaulting to stdout. If prompting is unavailable and no output path is supplied, the command fails with an actionable error.

The command will not print the full decrypted identity JSON to stdout.

**Rationale:** Sensitive structured data should only be materialized when the user explicitly chooses persistence. This also aligns the identity command space with the existing `account export` pattern.

### 5. Exported JSON uses a wallet-owned inspection schema

`identity export` should emit a stable wallet-owned JSON schema containing the decrypted stored identity details. The exported JSON should be versioned from the start and should have this logical shape:
- top-level `version`;
- `identity` object with `label`, `status`, `provider`, `identityIndex`, `createdAt`, and `expiresAt`;
- `network` object with `label` and `genesisHash`;
- `keySource` object with `kind` and `label`;
- `privatePayload` object with `codeUri` and `identityObject`.

Timestamps in the exported JSON should use RFC 3339 UTC strings. For pending identities, `privatePayload.identityObject` should be `null`, `privatePayload.codeUri` should contain the stored pending URI, and `expiresAt` should be `null` when no expiry is known. For completed identities, `privatePayload.codeUri` should be `null` because the local wallet discards it after completion.

The exported JSON should not include internal database identifiers such as SQLite row ids or `signer_owner_id`.

**Rationale:** This makes export useful for tooling and support workflows without pretending to define a portable protocol-standard identity format.

### 6. Label resolution follows rename-style ambiguity handling

Both `identity show` and `identity export` will accept a label and use the same ambiguity convention as `identity rename`:
- if exactly one stored identity matches, use it directly;
- if multiple identities match and the command can prompt, open a fuzzy selector with label-first rows and disambiguating network/key-source metadata;
- if the command cannot prompt, fail instead of guessing.

**Rationale:** This keeps identity-oriented label resolution consistent across commands and avoids hidden scope defaults for a “find one exact entity” workflow.

### 7. No non-prompting authenticated flow in the first cut

This change will not add a dedicated non-interactive or otherwise non-prompting password-input path.

If `identity show` or `identity export` cannot prompt for ambiguity resolution, password entry, or export destination selection where needed, the command should fail with an actionable error.

**Rationale:** Password-input conventions for non-prompting authenticated flows should be designed deliberately across the CLI rather than invented ad hoc for one command.

## Risks / Trade-offs

- **`identity show` reveals very sensitive data once unlocked** → This is intentional for inspection; the command must remain explicitly password-gated and clearly positioned as a reveal command.
- **A temporary reveal view reduces but does not eliminate persistence risk** → This matches the existing seed reveal model and is still preferable to normal scrollback-visible output.
- **Line-oriented rendering may be less faithful than raw JSON for nested values** → Keep `identity export` as the exact structured representation.
- **`identity export` intentionally persists sensitive data to disk** → Require an explicit file destination so persistence is always a deliberate user action.
- **Ambiguous labels may be common across networks** → Reusing rename-style fuzzy disambiguation mitigates this without requiring extra flags in common cases.
- **Future users may want scripted inspection or export** → Deferring non-prompting password input now keeps this change focused and avoids locking in a weak convention.
