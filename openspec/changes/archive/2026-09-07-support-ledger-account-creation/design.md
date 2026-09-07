## Context

See `proposal.md` for motivation and delta specs for required behavior. Current `account new` branches only far enough to reject Ledger owners, then assumes a seed record and `ConcordiumHdWallet`. `build_credential_deployment` likewise couples credential inputs, account signing-key derivation, and attribute-randomness derivation to that wallet.

The Ledger client already models purpose-based account-creation export, account public-key retrieval, and staged credential-deployment signing. App 5.6.4 changes the account-creation export's third field from a raw Ed25519 private key to a 64-byte extended SLIP-10 node, enabling deterministic child derivation without exporting the account signing key.

## Goals / Non-Goals

**Goals:**
- Share canonical credential construction between seed and Ledger sources.
- Keep Ledger-exported secrets transient, zeroize wallet-owned containers, and minimize unavoidable lower-level copies.
- Preserve current account persistence, submission, and finalization behavior.
- Keep low-level APDU sequencing inside `ccd-wallet-ledger` and source-aware orchestration inside `ccd-wallet`.

**Non-Goals:**
- Storing exported Ledger secrets for offline account creation or recovery.
- Supporting Ledger app versions older than 5.6.4 for account creation.
- Changing account database layout or account transaction signing.
- Generalizing credential construction for hypothetical signer types.

## Decisions

### Split credential construction by required cryptographic inputs

Extract a narrow credential-construction core that accepts ID credential data, credential public-key data, and an implementation of attribute-randomness retrieval. Keep thin seed and Ledger adapters rather than introducing a general signer framework.

Seed construction continues to derive all inputs from `ConcordiumHdWallet`. Ledger construction supplies exported PRFKey/IDCredSec, a device public key, and randomness derived from the exported extended node. This avoids fabricating a mnemonic wallet or widening `ConcordiumHdWallet` into a multi-source abstraction.

Alternative: duplicate `create_credential` orchestration for Ledger. Rejected because canonical proof and message assembly would then diverge in security-sensitive code.

### Treat the exported randomness value as an extended SLIP-10 node

Represent the 64-byte value as a zeroizing private-key/chain-code pair and add the smallest derivation operation needed for hardened attribute-tag children. Derivation semantics must match the existing wallet's SLIP-10 implementation and Ledger's path: account export already selects `.../5'/account_index'`, so host derivation appends only `attribute_tag'`.

Alternative: use only the first 32 bytes. Rejected because it reproduces the defect fixed by LedgerHQ/app-concordium#40 and cannot derive protocol-compatible children.

### Keep chain context outside the Ledger construction bridge

Account command orchestration fetches the identity object, identity-provider information, anonymity-revoker information, and global chain context, then passes them with the connected Ledger app and derivation coordinates into the Ledger construction bridge. The bridge owns transient exports, unsigned credential construction, private conversion from SDK credential structures to existing low-level Ledger signing request types, signature application, and returning the final SDK `AccountCredentialMessage`.

The SDK-to-Ledger conversion remains private to `ccd-wallet`; `ccd-wallet-ledger` continues to expose protocol request types and APDU choreography without depending on higher-level SDK credential construction.

Alternative: make the low-level Ledger crate construct SDK credentials or fetch chain context. Rejected because it would couple transport code to wallet orchestration and broaden its public API.

### Construct proofs on host and sign final deployment on device

Use account-creation export material together with signature blinding randomness from the purpose-based identity-recovery export to construct the normal credential and proofs. Compare the IDCredSec returned by both exports and fail before proof construction if they differ. Build the existing staged `CredentialDeploymentSigningRequest` from that canonical result, ask Ledger for the credential signature, then assemble the final SDK message.

Account private signing keys never leave Ledger. Export APIs are not treated as transaction-signing APIs.

Alternative: export or synthesize the account signing key. Rejected because Ledger explicitly prohibits signing-key export and fallback signing would violate owner semantics.

### Branch account orchestration before seed lookup

After common network, owner, identity, label, and counter resolution, branch by signer-owner kind:

```text
common context
     |
     +-- seed ---- unlock seed ---- seed credential builder --+
     |                                                        |
     +-- Ledger -- approve export -- device credential builder +
                                                              |
                           pending record -> submit -> finalize
```

The Ledger branch opens and verifies the device, performs export/construction/signing, and never queries the seed table. Both branches return the same SDK credential message and converge before submission.

Alternative: preserve the current early Ledger probe and continue into seed code. Rejected because it cannot work for a Ledger-only owner and obscures lifecycle boundaries.

### Keep seed and Ledger source selectors distinct

Expose mutually exclusive `--seed <LABEL>` and `--ledger` options on `identity new` and `account new`. `--seed` preserves backward-compatible named key-source behavior, including explicit Ledger owner labels. `--ledger` opens the connected device, derives its canonical identity, and matches it to exactly one enrolled Ledger owner before resolving identities or constructing requests. Neither option preserves existing default and interactive resolution.

For `account new --ledger`, identity selection is scoped to the matched Ledger owner. For `identity new --ledger`, the matched owner supplies the issuance derivation coordinates. Non-interactive use fails unless device matching is unique. `--allow-ledger-secret-export` remains orthogonal authorization rather than a source selector.

Alternative: rename `--seed` to generic `--key-source` or reject Ledger labels passed through `--seed`. Rejected to preserve existing scripts and compatibility; `--ledger` supplies the clearer device-driven path.

### Reuse existing explicit-export policy

Add the existing `--allow-ledger-secret-export` convention to `account new`. Interactive mode shows the same class of warning and confirmation used by identity issuance; non-interactive mode requires the option. Update `docs/commands.md` with all command options.

Alternative: rely only on device confirmation. Rejected because existing wallet policy requires host-side explicit approval and clear disclosure that secrets enter host memory.

### Persist pending state only after credential construction succeeds

Construct and sign first, then insert the pending account immediately before submission, matching safe failure semantics while retaining the existing submitted/finalized transitions. If insertion succeeds but submission fails, preserve current cleanup or reconciliation behavior rather than adding a new Ledger-specific lifecycle.

## Risks / Trade-offs

- [Incorrect extended-node derivation produces invalid proofs] → Add vectors comparing root-plus-child derivation with full seed-path derivation and known Ledger output.
- [SDK credential representation is difficult to stage before its signature exists] → Reuse SDK lower-level credential construction primitives and isolate final device signature insertion.
- [Secret remnants may survive in transport, allocator, cryptographic-library, driver, or OS-managed buffers] → Zeroize wallet-owned containers, use a secret-specific in-place APDU response path, avoid application-level clones, minimize lifetime, and document that lower-level clearing remains best effort.
- [Device/app incompatibility appears as opaque APDU status] → Map unsupported instruction/status responses to a 5.6.4 requirement and distinguish malformed successful responses.
- [Two exports could derive inconsistent identity material] → Require identical IDCredSec values before using either export.
- [Pending account row is left after pre-submission device failure] → Complete export, proof generation, and signing before pending insertion.

## Migration Plan

1. Ship CLI support guarded by successful 5.6.4-format export rather than stored migration state.
2. Existing seed and Ledger records require no database migration.
3. Users update the Concordium Ledger app to 5.6.4 or newer before using `account new` with a Ledger key source.
4. Rollback consists of reverting CLI support; existing created accounts remain valid and continue using normal Ledger transaction signing.
