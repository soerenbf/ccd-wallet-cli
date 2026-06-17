## Why

Governance update submission and detached proposal signing currently validate resolved updates, then sign or submit without a final human review/approval step. Adding that step makes testing and operator workflows safer without blocking users from intentionally signing or submitting experimental updates.

## What Changes

- Add an interactive review-and-approval gate before signing or submitting governance updates.
- Apply the gate to `ccd-wallet governance update`, `ccd-wallet governance proposal sign`, and `ccd-wallet governance proposal submit`.
- Render a governance update review with parsed payload details where the wallet can decode them, plus network, payload/update type, sequence, timing, signer/signature context, and blind-signing warnings where relevant.
- Use the existing cliclack yes/no confirmation prompt for approval so users can select Yes/No with the keyboard rather than typing `yes`; decline returns successfully without submitting.
- Preserve non-interactive behavior by skipping prompts and relying on existing validation when `--non-interactive` is supplied.
- Preserve existing payload input prompts, timing prompts, signer selection, validation, signing, submission, and finalization semantics except for the added pre-sign/pre-submit approval gate.

## Capabilities

### New Capabilities

### Modified Capabilities
- `governance-update-submission`: Governance update signing and submit paths gain an interactive transaction-style parsed-payload review and cliclack yes/no approval requirement before detached signing or node submission.

## Impact

- Affected Rust CLI code: `crates/ccd-wallet/src/commands/governance.rs`.
- Affected docs: `docs/commands.md` governance command descriptions may need a short mention of the review/approval gate.
- No new dependencies are expected.
- No persisted database, encryption, or wire-format changes are expected.
