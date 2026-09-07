## 1. Extended Randomness Derivation

- [x] 1.1 Add a zeroizing extended SLIP-10 node representation and hardened child derivation for exported account randomness; verify unit vectors match full seed-path derivation for multiple attribute tags.
- [x] 1.2 Parse Ledger account-creation exports as `32/32/64` length-prefixed fields with exact framing checks; verify valid, truncated, wrong-length, and trailing-byte unit cases.

## 2. Source-Aware Credential Construction

- [x] 2.1 Refactor credential construction around explicit ID credential data, credential public keys, and attribute-randomness retrieval while preserving the seed adapter; verify existing seed account-creation tests pass unchanged.
- [x] 2.2 Add Ledger credential construction using account-creation PRFKey/IDCredSec/extended randomness, identity-recovery signature blinding randomness, and the device account public key; reject mismatched exported IDCredSec values and verify focused construction tests pass.
- [x] 2.3 Extend the private `ccd-wallet` Ledger construction input with caller-fetched identity/IP/AR/global context and the connected app, privately convert the unsigned SDK credential into the existing staged credential-deployment signing request, apply the device signature to the SDK message, and verify APDU sequence plus final message signature in mock-transport tests without broadening the public `ccd-wallet-ledger` API.
- [x] 2.4 Keep exported and derived Ledger secrets transient, zeroize wallet-owned containers, minimize unavoidable lower-level copies, and document boundary limitations; verify focused secret-handling tests and `cargo clippy` report no regressions.

## 3. Account Command Integration

- [x] 3.1 Add mutually exclusive `--seed <LABEL>` and `--ledger` selectors to `identity new` and `account new`; verify clap parsing, conflict rejection, omitted-source behavior, unique device-to-owner matching, and owner-scoped identity selection.
- [x] 3.2 Add `--allow-ledger-secret-export` to `account new` as authorization independent of source selection; verify interactive and non-interactive approval tests.
- [x] 3.3 Replace the Ledger rejection stub with device connection, owner matching, account-creation export, credential construction, and device signing; verify unsupported app, malformed response, mismatch, rejection, and signing-error tests fail before submission.
- [x] 3.4 Split `account new` before seed lookup so seed and Ledger branches produce one credential message and share persistence/submission/finalization; verify Ledger flows never request a seed password and seed flows retain current behavior.
- [x] 3.5 Preserve safe pending-account lifecycle boundaries around Ledger failures and submission; verify tests cover pre-insert device failure, submission failure, `--no-wait`, and successful finalization.

## 4. Documentation and Validation

- [x] 4.1 Update `docs/commands.md` and Ledger crate documentation with the new option, app 5.6.4 minimum, exported-field format, and security warning; verify documented command surface matches clap help.
- [x] 4.2 Run `cargo fmt --check`, targeted Ledger/core/CLI tests, `cargo clippy --all-targets --all-features`, and workspace tests; verify all commands pass.
- [x] 4.3 Validate account creation on a physical matching Ledger running Concordium app 5.6.4 on testnet, confirming export, device approval, node submission, and final account persistence without storing exported secrets. Validation evidence: user completed the interactive flow on a Nano S+ with app 5.6.4; both exports, credential signing, testnet submission, and default finalization completed successfully.
