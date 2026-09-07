## Why

Ledger Concordium app 5.6.4 can now export the complete deterministic material required to construct account credentials, including the extended commitment-randomness key introduced by LedgerHQ/app-concordium#40. The CLI still rejects Ledger-backed account creation, forcing users to use a seed-backed identity despite existing Ledger export and credential-signing protocol support.

## What Changes

- Add mutually exclusive `--seed <LABEL>` and `--ledger` source selectors to `identity new` and `account new`; `--ledger` resolves the connected device to exactly one enrolled Ledger owner.
- Enable `account new` for identities owned by an enrolled Ledger signer.
- Export transient account-creation and identity-recovery material through Ledger purpose-based protocols after explicit user approval, rejecting mismatched duplicate IDCredSec values.
- Construct the normal credential and zero-knowledge proofs on the host while deriving attribute randomness from the exported extended SLIP-10 node.
- Obtain the account public key and final credential-deployment signature from the matching Ledger device; never export the account signing key.
- Reuse the existing account persistence, node submission, and finalization lifecycle for Ledger-backed accounts.
- Fail before submission on unsupported app versions, malformed exports, device mismatch, user rejection, or signing failure.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `account-creation`: Allow account creation from a stored usable Ledger-owned identity without requiring a local seed and define explicit source selection.
- `identity-issuance`: Allow explicit connected-Ledger source selection alongside named seed selection.
- `ledger-identity-account-construction`: Define the approved export, host construction, and device signing behavior for Ledger-backed account creation.

## Impact

Affected areas include identity and account creation CLI/source resolution, account creation orchestration in `ccd-wallet`, credential construction and randomness derivation in `ccd-wallet-core`, purpose-based export and signing request use in `ccd-wallet-ledger`, Ledger/device tests, and account-creation documentation. No database schema, command taxonomy, or external API change is expected. Ledger Concordium app 5.6.4 or newer becomes required for this flow.
