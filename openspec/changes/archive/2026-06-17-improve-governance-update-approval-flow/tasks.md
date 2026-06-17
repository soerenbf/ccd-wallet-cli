## 1. Review Rendering

- [x] 1.1 Add governance review rendering helpers in `crates/ccd-wallet/src/commands/governance.rs` for prepared updates, timing, parsed payload details, payload identity, sequence, network, and blind-payload warnings.
- [x] 1.2 Add signer-context rendering for local governance-vault signers and Ledger signing in `governance update` and `governance proposal sign`.
- [x] 1.3 Add detached-signature-context rendering for accepted signatures in `governance proposal submit`.

## 2. Approval Flow

- [x] 2.1 Add a reusable governance approval helper using the existing `cliclack::confirm` yes/no prompt with an initial No value.
- [x] 2.2 Insert the interactive approval gate in `governance update` after payload/timing/chain/signer resolution and before local or Ledger signing.
- [x] 2.3 Insert the interactive approval gate in `governance proposal sign` after proposal/signer validation and before local or Ledger signing.
- [x] 2.4 Insert the interactive approval gate in `governance proposal submit` after proposal/signature validation and before node submission.
- [x] 2.5 Ensure `--non-interactive` skips the approval prompt for governance update, detached signing, and detached submission paths.
- [x] 2.6 Ensure declined approval returns without signing or submitting and without reporting a failure.

## 3. Tests and Documentation

- [x] 3.1 Add or update Rust tests for review rendering, parsed payload detail output, confirmation flow behavior, and non-interactive prompt skipping where practical.
- [x] 3.2 Update `docs/commands.md` to document the governance signing/submission review/approval gate.
- [x] 3.3 Run targeted Rust validation for the CLI crate, including formatting and relevant tests.
