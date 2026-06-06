## Context

`identity new` resolves Ledger-backed key sources, but the wallet must avoid silently falling back to seed derivation or host-only randomness. Seed-backed issuance unlocks a seed, derives identity issuance material through `ConcordiumHdWallet`, builds the request JSON in `ccd-wallet-identity-provider`, and then runs the browser/callback/polling flow.

The installed Ledger app source at `/Users/sorenbz/Developer/Concordium/app-concordium` tag `flex_1.6.0_5.4.1_sdk_v26.0.2` was inspected and documented in `ledger-app-5.4.1-analysis.md`. That tag cannot provide deterministic signature blinding randomness. Repository history shows the required purpose-based export appears in app version `5.5.0`.

The relevant boundaries are:

- `crates/ccd-wallet/src/commands/identity/new.rs` owns CLI orchestration and local storage sequencing.
- `crates/ccd-wallet/src/commands/ledger_construction.rs` owns wallet policy around Ledger identity/account construction.
- `crates/ccd-wallet-ledger` exposes raw APDU-close Ledger commands.
- `crates/ccd-wallet-identity-provider` can build an identity request from prepared issuance material.
- Ledger-owned identities use the Ledger owner's local vault for encrypted private payload storage.

## Goals / Non-Goals

**Goals:**

- Accurately model app `5.4.1` `INS=0x37` as legacy new-path export in `ccd-wallet-ledger`.
- Re-enable Ledger-backed `identity new` assuming app `5.5.0+` purpose-based identity credential creation export is available.
- Export IDCredSec, PRFKey, and signature blinding randomness from the Ledger after explicit approval.
- Reject legacy/raw 32-byte or 64-byte app `5.4.1` responses as incomplete identity issuance material.
- Preserve explicit export approval UX and non-interactive guardrails.
- Preserve seed-backed identity issuance behavior.
- Keep low-level Ledger APIs APDU-close and return raw bytes.

**Non-Goals:**

- Generating signature blinding randomness on the host for Ledger-backed identity issuance.
- Supporting Ledger-backed identity issuance on app `5.4.1` or older.
- Changing the wallet database schema.
- Adding generic secret-export support for unrelated flows.
- Changing browser callback transport behavior.

## Decisions

### 1. App 5.4.1 remains legacy new-path export

**Decision:** `ccd-wallet-ledger` SHALL continue to model app `5.4.1` `INS=0x37` as legacy new-path export:

- `P1=0x00`: PRF key, decrypt-credentials display wording
- `P1=0x01`: PRF key, recovery display wording
- `P1=0x02`: PRF key followed by IDCredSec, create-credentials display wording
- `P2=0x01`: ed25519 seed output
- `P2=0x02`: BLS key output
- `CDATA=idp_index[uint32] || identity_index[uint32]`
- response: raw 32-byte or 64-byte concatenated values

**Rationale:** The checked-out app `5.4.1` tag implements this behavior and cannot provide signature blinding randomness.

### 2. Ledger identity issuance assumes app 5.5.0+ purpose-based export

**Decision:** `ledger_construction::construct_identity_issuance` SHALL use the app `5.5.0+` purpose-based identity credential creation export after explicit approval:

```text
CLA  = 0xE0
INS  = 0x37
P1   = 0x00  // identity credential creation
P2   = 0x00 for mainnet, 0x01 for testnet
DATA = idp_index[uint32 big-endian] || identity_index[uint32 big-endian]
RESP = [32]IDCredSec || [32]PRFKey || [32]signature_blinding_randomness
```

The construction layer SHALL parse exactly three repeated `[length=32][key]` fields in that order and translate them into `IdentityIssuanceMaterial`.

**Rationale:** App `5.5.0` is the first release line found to contain `NEW_SIGNATURE_BLINDING_RANDOMNESS` and the purpose-based identity credential creation flow. That flow provides all recovery-critical issuance material from deterministic Ledger derivation paths.

### 3. Legacy/raw responses are rejected

**Decision:** The wallet SHALL reject raw 32-byte and 64-byte responses from legacy new-path export as incomplete identity issuance material.

**Rationale:** Those responses correspond to app `5.4.1` or older semantics and lack deterministic signature blinding randomness. Accepting them would either fail request construction or tempt host-generated randomness, which breaks Ledger-backed recovery.

### 4. Export approval remains mandatory

**Decision:** The CLI SHALL require explicit export approval before the purpose-based identity export command is sent. In non-interactive mode, `--allow-ledger-secret-export` remains required.

**Rationale:** This is an export-based flow, not an on-device signing flow. Users and automation must opt into that security model explicitly.

### 5. Do not use app-version APDU as an issuance preflight

**Decision:** The user-facing issuance path SHALL not depend on calling `INS=0x40` before export. It SHALL send the purpose-based export only after explicit approval and then treat unsupported/invalid APDU statuses or malformed responses as actionable errors.

**Rationale:** Earlier device testing suggested extra preflight APDUs can destabilize some flows. The actual compatibility gate is whether the approved export command succeeds with the 5.5.0+ response format.

## Risks / Trade-offs

- **Users on app 5.4.1 cannot complete Ledger-backed `identity new`** → Mitigation: they receive an update-oriented error or malformed-response error before provider contact/storage.
- **Exported secrets temporarily exist in host memory** → Mitigation: dedicated confirmation language, explicit non-interactive opt-in, zeroizing/transient handling where practical, and no persistence.
- **Protocol naming confusion** → Mitigation: keep legacy new-path and purpose-based export APIs separate and document version behavior.
- **Physical app 5.5.0+ not yet tested in this session** → Mitigation: implement against checked source/tags and mock-transport tests; real-device validation is the next step.

## Migration Plan

1. Record installed app 5.4.1 and app 5.5.0 protocol findings in `ledger-app-5.4.1-analysis.md`.
2. Update OpenSpec proposal/design/specs/tasks to require app `5.5.0+` purpose-based export for Ledger identity issuance.
3. Ensure `ccd-wallet-ledger` models legacy new-path and purpose-based export separately.
4. Re-enable `ledger_construction::construct_identity_issuance` using purpose-based `P1=0x00` with network `P2`.
5. Parse exactly three length-prefixed 32-byte keys: IDCredSec, PRFKey, and signature blinding randomness.
6. Reject raw legacy responses and unsupported statuses before provider contact/storage.
7. Re-run formatting, tests, clippy, and OpenSpec validation.

## Open Questions

- Does Ledger Live currently offer app `5.5.0+` for the user's device model?
- Does the physical app `5.5.0+` export path behave exactly like the checked source on the user's firmware/device?
- Should a future change add an explicit user-facing minimum-version check before export, or is export-response gating sufficient?
