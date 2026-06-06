## 1. Identity request construction APIs

- [x] 1.1 Add a lower-level `ccd-wallet-identity-provider` request builder that accepts already-derived identity issuance material instead of only `ConcordiumHdWallet`.
- [x] 1.2 Keep the existing seed-based request builder as a wrapper over the new lower-level API and preserve current seed-backed behavior.
- [x] 1.3 Add tests covering seed-backed and material-based request construction parity.

## 2. Ledger identity construction flow

- [x] 2.1 Extend `ledger_construction::construct_identity_issuance` to verify the connected Ledger owner and orchestrate the required export sequence for identity issuance material.
- [x] 2.2 Add request/response translation helpers for the exported Ledger issuance material without persisting exported secrets.
- [x] 2.3 Add mock-transport tests covering successful export, user rejection, owner mismatch, and no-storage-before-success behavior.

## 3. CLI identity issuance integration

- [x] 3.1 Update `identity new` key-source handling so Ledger-backed issuance uses the Ledger construction layer and seed-backed issuance keeps the current path.
- [x] 3.2 Add the explicit interactive warning/confirmation step for Ledger secret export with wording that distinguishes export from on-device signing.
- [x] 3.3 Add the non-interactive `--allow-ledger-secret-export` flag and actionable errors when it is missing.
- [x] 3.4 Ensure Ledger-backed pending and completed identity payloads are encrypted under the Ledger signer-owner vault DEK.

## 4. End-to-end behavior and documentation sync

- [x] 4.1 Add or update tests covering successful Ledger-backed `identity new`, declined export, provider error cleanup, and timeout behavior.
- [x] 4.2 Update `docs/commands.md` if the CLI surface changes to include a new non-interactive Ledger export flag or revised key-source wording.
- [x] 4.3 Run relevant Rust formatting, linting, and test commands for the touched crates and verify the OpenSpec change is implementation-ready.

## 5. Ledger app protocol correction and version-gated re-enable

- [x] 5.1 Add low-level Ledger app-version support for `INS 0x40` and expose the app name/version through a Ledger app inspection command (for example `ledger show`).
- [x] 5.2 Update `docs/commands.md` for the new Ledger app inspection command.
- [x] 5.3 Split or clarify `ccd-wallet-ledger` private-key export-new APIs so legacy-new-path semantics are distinct from later purpose-based semantics.
- [x] 5.4 Preserve fail-safe Ledger identity issuance behavior after physical-device testing showed installed app 5.4.1 does not provide all recovery-critical issuance material.
- [x] 5.5 Keep purpose-based export parsing isolated for later app branches without using it as app 5.4.1 behavior.
- [x] 5.6 Add tests for app-version parsing, export APDU shapes, malformed export responses, and no pending identity storage before failures.
- [x] 5.7 Run relevant Rust formatting, linting, and tests for `ccd-wallet-ledger`, `ccd-wallet`, and identity-provider integration paths.

## 6. Installed Ledger app 5.4.1 source alignment

- [x] 6.1 Inspect `/Users/sorenbz/Developer/Concordium/app-concordium` at the checked-out tag corresponding to the installed app.
- [x] 6.2 Document the app 5.4.1 `INS=0x37` handler, accepted P1/P2 values, payloads, UI wording, and response formats in `ledger-app-5.4.1-analysis.md`.
- [x] 6.3 Update OpenSpec proposal/design/specs so app 5.4.1 is modeled as legacy new-path export, not purpose-based identity issuance.
- [x] 6.4 Update `ccd-wallet-ledger` comments/docs so legacy new-path export is not described as pre-5.4-only and purpose-based export is not described as app 5.4.1 behavior.
- [x] 6.5 Keep Ledger-backed `identity new` fail-safe until deterministic signature blinding randomness is Ledger-backed; do not use host-generated replacement randomness.
- [x] 6.6 Run formatting, focused Rust tests/clippy, and strict OpenSpec validation after the source-alignment updates.

## 7. App 5.5.0+ purpose-based Ledger identity issuance

- [x] 7.1 Update proposal/design/specs to require app 5.5.0+ purpose-based identity credential creation export for Ledger-backed identity issuance.
- [x] 7.2 Re-enable `ledger_construction::construct_identity_issuance` using `INS=0x37`, `P1=0x00`, network-designated `P2`, and `idp || identity` payload.
- [x] 7.3 Parse exactly three length-prefixed 32-byte fields ordered IDCredSec, PRFKey, and signature blinding randomness.
- [x] 7.4 Pass mainnet/testnet export network designation from `identity new` into the Ledger construction layer.
- [x] 7.5 Reject unsupported statuses and raw legacy 32/64-byte responses before provider contact/storage.
- [x] 7.6 Update Ledger crate docs/comments to identify purpose-based export as app 5.5.0+ behavior.
- [x] 7.7 Run formatting, focused Rust tests/clippy, and strict OpenSpec validation after the 5.5.0+ implementation.
- [x] 7.8 Verify the connected Ledger canonical public key matches the selected signer owner before identity issuance export/storage.
