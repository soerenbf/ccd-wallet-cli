## Context

The wallet already knows how to reconstruct usable account signing material for both account sources:
- derived accounts can be rebuilt from the owning seed plus derivation coordinates
- imported accounts can be rebuilt from the encrypted imported-account payload

That source-aware reconstruction is already used internally for signing flows, but there is no CLI command that materializes the result as a standalone JSON signer file. The Concordium Rust SDK already accepts a minimal JSON account shape through `WalletAccount::from_json_file`, so this change is primarily about CLI UX, source-aware material extraction, and safe file writing rather than new cryptographic or persistence primitives.

The change touches multiple existing conventions:
- account command parsing and routing
- account selection and network disambiguation
- seed-password vs imported-vault-password unlock flows
- human-oriented CLI messaging around sensitive operations

A key constraint is that derived accounts do not currently store full genesis-style credential bundles locally. The initial export format must therefore be something the wallet can generate for both derived and imported accounts from material it already has or can deterministically reconstruct.

## Goals / Non-Goals

**Goals:**
- Add an `account export` command under the existing account command group.
- Export a selected account in a JSON shape accepted by `concordium_rust_sdk::types::WalletAccount::from_json_file`.
- Support both derived and imported accounts through a single source-aware export flow.
- Require the correct secret-unlock path before revealing and writing signing material.
- Keep file output explicit so plaintext signing material is only written to a user-chosen destination.

**Non-Goals:**
- Export full genesis-style account bundles for derived accounts.
- Add browser-wallet wrapper export, encrypted export, QR export, or multiple output formats in this change.
- Change the SQLite schema or persist additional signing metadata for derived accounts.
- Introduce bulk export or directory export.

## Decisions

### Export the minimal SDK-compatible JSON shape
The command will emit the smallest format accepted by `WalletAccount::from_json_file`:
- `address`
- `accountKeys`

The file will not include genesis-only fields such as `credentials`, `encryptionPublicKey`, `encryptionSecretKey`, or `aci`, and it will not wrap the payload in the browser-wallet export envelope.

**Rationale:** this is the only format that cleanly works for both imported and derived accounts with the wallet's current data model. It directly satisfies the stated interoperability target while avoiding invented partial-genesis exports.

**Alternatives considered:**
- Full genesis-style export: possible for imported accounts, but not generally for derived accounts because the required credential bundle is not stored locally.
- Browser-wallet wrapper export: also SDK-compatible, but adds format-specific metadata without solving an immediate user need.

### Reconstruct export material through account source kind
The export flow will resolve a target account record, then build exportable material according to `source_kind`:
- derived: unlock the owning seed, decrypt the stored account address, derive the signing key, and synthesize `accountKeys`
- imported: unlock the imported accounts vault, decrypt the imported payload, and reuse the stored `accountKeys`

The command should share the same source-aware account-material logic used by existing signer-building paths as much as practical.

**Rationale:** this keeps export behavior aligned with the wallet's existing account model instead of creating a separate storage or derivation path.

**Alternatives considered:**
- Persist precomputed export JSON in the database: unnecessary plaintext-material expansion and added schema complexity.
- Support only imported-account export initially: simpler, but weaker UX and inconsistent with the wallet's unified account abstraction.

### Keep destination selection explicit and file-based
The command will write signer JSON to a user-chosen file path rather than silently inventing a default export location. In non-interactive use, the destination must be supplied explicitly. Interactive prompting may help fill it in, but the command remains file-oriented.

**Rationale:** exporting plaintext signing material is sensitive. Requiring an explicit destination reduces surprise and avoids accumulating unmanaged secret files in default directories.

**Alternatives considered:**
- Auto-generate a filename in the current directory: convenient, but too implicit for a secret-bearing export.
- Print signer JSON to stdout by default: script-friendly, but easy to leak into shell history, terminals, logs, or pipes unintentionally.

### Preserve existing account-selection and label-disambiguation behavior
`account export` will reuse the wallet's established account-selection model rather than introducing an export-specific account identifier. Labels remain human-facing, and network context remains the primary way to disambiguate records where necessary.

**Rationale:** export should feel like another account operation, not a separate addressing model.

**Alternatives considered:**
- Require internal account IDs: precise, but user-hostile.
- Require globally unique labels across all networks: inconsistent with existing wallet behavior.

## Risks / Trade-offs

- **[Plaintext secret file creation]** Export writes hot signing material to disk. → **Mitigation:** require explicit destination selection, prompt through the correct unlock flow, and document the security implications clearly.
- **[Format expectation mismatch]** Users may expect exported files to be re-importable as full genesis bundles. → **Mitigation:** document that v1 exports target SDK signer compatibility, not full genesis round-tripping.
- **[Logic duplication risk]** Export and transaction-signing paths may each rebuild similar `WalletAccount` data. → **Mitigation:** factor shared source-aware account-material helpers where practical during implementation.
- **[Account ambiguity across networks]** The same label can exist on multiple networks. → **Mitigation:** reuse existing network/account selection flows and fail with actionable ambiguity errors when needed.

## Migration Plan

- Add the new CLI subcommand and handler without changing existing account commands.
- Reuse existing account lookup and unlock machinery; no schema or stored-data migration is required.
- Update documentation to describe the supported export format and security expectations.
- Rollback is straightforward because the change only adds command behavior and produces external files on user request.

## Open Questions

- None for the initial proposal. Richer export formats can be proposed later as a follow-up change once the minimal SDK-compatible export exists.
