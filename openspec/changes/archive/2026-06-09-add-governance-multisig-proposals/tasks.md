## 1. Command surface and detached file models

- [x] 1.1 Add a new `governance proposal` command family in `crates/ccd-wallet/src/cli.rs` with `create`, `sign`, and `submit` subcommands and flags for proposal/signature file paths, repeated `--signature` inputs, signature-directory input, local key selection, Ledger selection, network selection, and `--no-wait` where applicable.
- [x] 1.2 Add proposal-file and detached-signature-file Rust types in `crates/ccd-wallet/src/commands/governance.rs` (or a small supporting module) with serde-backed JSON parsing/printing, canonical pretty JSON writers, and validation helpers for the versioned file formats.
- [x] 1.3 Add shared helpers that convert between proposal JSON, `ResolvedGovernanceUpdatePayload`, `PreparedGovernanceUpdate`, detached signature wrappers, and `GovernanceSignerOutput` without changing the existing all-in-one signing semantics.

## 2. Detached proposal creation and revalidation

- [x] 2.1 Implement `governance proposal create` by reusing the existing governance payload parsing and chain-context resolution helpers to produce a frozen proposal header and payload, while requiring explicit effective time and timeout inputs without detached-proposal defaults.
- [x] 2.2 Implement proposal revalidation helpers that confirm proposal genesis hash, payload decoding, payload-size/header consistency, authorization-family derivation, sequence-queue derivation, and stale-sequence rejection against the live node.
- [x] 2.3 Add focused unit tests for proposal JSON round-tripping, payload-size validation, network mismatch rejection, and stale proposal rejection.

## 3. Detached signing for local keys and Ledger

- [x] 3.1 Implement `governance proposal sign` for local governance keys, including the existing interactive governance-key selection flow, live authorization lookup, verify-key-to-index resolution, proposal-hash signing, and detached signature-file output.
- [x] 3.2 Implement `governance proposal sign --ledger`, reusing the existing Governance Ledger path derivation and signing helpers while allowing detached signing for thresholds greater than one.
- [x] 3.3 Add tests for detached local signing and detached Ledger signing behavior, including unauthorized signer rejection, wrong-network rejection, stale proposal rejection, and single-entry signer-indexed signature output.

## 4. Detached submission assembly and validation

- [x] 4.1 Implement `governance proposal submit` to load one or more detached signature files from repeated `--signature` flags and optional signature-directory input, re-derive the expected signer index for each verify key from the live authorization context, verify signatures against the proposal signing hash, and assemble the final `UpdateInstruction`.
- [x] 4.2 Reuse the existing submission and finalization waiting flow for detached proposals, including threshold checks, duplicate-index rejection, `--no-wait`, and clear errors for below-threshold or stale proposals.
- [x] 4.3 Add tests covering successful detached submission assembly, mismatched verify-key/index rejection, below-threshold rejection, duplicate signer rejection, and no-wait/finalization behavior.

## 5. Documentation and command taxonomy

- [x] 5.1 Update `docs/commands.md` to document the new `governance proposal` command family and explain how it relates to the existing all-in-one `governance update` flow.
- [x] 5.2 Update the existing governance Ledger documentation text to describe detached multi-party signing support and to limit the current single-signer restriction to the all-in-one `governance update --ledger` flow.
- [x] 5.3 Run the relevant Rust test suite for governance command parsing and governance command behavior, then verify the OpenSpec artifacts and docs stay aligned with the implemented command surface.
