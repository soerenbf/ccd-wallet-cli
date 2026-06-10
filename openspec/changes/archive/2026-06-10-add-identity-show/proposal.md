## Why

Users can list identities and rename them, but they cannot inspect a single stored identity in depth. That leaves no direct way to verify which key source owns an identity, inspect issuance state for pending identities, or view the decrypted issued identity object for troubleshooting and recovery-oriented workflows.

The wallet already stores enough plaintext metadata for `identity list`, and it already stores the private issuance payload encrypted under the owning signer-owner password domain. What is missing is a dedicated inspection command that resolves an identity, authenticates the user against the owning key source, decrypts the private payload, and reveals it in a temporary sensitive view, plus an explicit export command for structured JSON output.

## What Changes

- Add `ccd-wallet identity show <LABEL>` to inspect a stored identity in a temporary sensitive reveal view.
- Add `ccd-wallet identity export <LABEL> [--out <FILE>]` to export a stored identity as JSON to an explicit file destination.
- Resolve ambiguous labels using the same convention as `identity rename` where practical:
  - exact label lookup first;
  - if multiple matches exist and the command can prompt, ask the user to choose the identity;
  - if the command cannot prompt, fail instead of guessing.
- Prompt for the owning key source's password before revealing or exporting private identity data.
- Make `identity show` reveal the complete selected identity in a temporary sensitive view similar to `seed show`, including metadata, `code_uri`, and the issued identity object when present.
- Keep the human show view free of raw internal identifiers such as signer-owner id and network genesis hash.
- Make `identity export` write a stable wallet-owned JSON file rather than printing sensitive JSON to stdout.
- Let `identity export` accept `--out <FILE>` inline and otherwise prompt for a destination when prompting is available.
- Do not add a non-prompting authenticated mode for identity inspection or export yet.
- Update command taxonomy documentation to include `identity show` and `identity export`.

## Capabilities

### New Capabilities
- `identity-inspection`: Resolve, authenticate, decrypt, and reveal stored identity details in a temporary sensitive view.
- `identity-export`: Resolve, authenticate, decrypt, and export stored identity details as a JSON file.

### Modified Capabilities
- `command-taxonomy`: Add `identity show` and `identity export` to the implemented identity command surface.

## Impact

- Rust CLI command definitions in `crates/ccd-wallet/src/cli.rs`.
- Identity command dispatch and rendering/export logic in `crates/ccd-wallet/src/commands/identity/`.
- Key-source password prompt and vault-unlock flow reuse.
- Sensitive temporary reveal-view handling for `identity show`.
- JSON file export for `identity export`.
- `docs/commands.md` command taxonomy documentation.
