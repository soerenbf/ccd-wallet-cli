## Why

`identity new` already recognizes Ledger-backed key sources, but it intentionally stops before any local state is written because the end-to-end issuance flow has not been defined. We now want to support Ledger-owned identities by using an explicit export security model, so operators can bootstrap Ledger-backed identity ownership without silently falling back to seed-based derivation or ambiguous hardware-wallet semantics.

## What Changes

- Add Ledger support to `identity new` for enrolled Ledger key sources running a supported Concordium Ledger app protocol.
- Add a Ledger app inspection command (for example `ledger show`) that displays the connected app name and app version when the device supports version reporting.
- Use an explicit export-based security model for Ledger identity issuance: the CLI warns that issuance secrets will be exported into host process memory temporarily and requires clear user approval before continuing.
- Treat the checked-out app 5.4.1 tag (`flex_1.6.0_5.4.1_sdk_v26.0.2`) as authoritative for the installed device and document its `INS=0x37` legacy new-path export protocol.
- Re-enable Ledger-backed identity issuance assuming a Concordium Ledger app `5.5.0` or newer is connected, using its purpose-based identity credential creation export to retrieve every recovery-critical issuance secret.
- Store pending and completed identity private payloads under the Ledger signer owner's local vault DEK rather than the seed password domain.
- Introduce non-interactive guardrails so Ledger identity issuance cannot perform secret export implicitly.
- Preserve the existing safety boundary: if export is declined, unsupported, app version/protocol is too old, or device ownership does not match, no pending identity row is written.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `ledger-identity-account-construction`: Use the Ledger construction layer to perform purpose-based app `5.5.0+` identity credential creation export and reject legacy/raw export responses.
- `identity-issuance`: Allow `identity new` on Ledger key sources after explicit export approval when the connected app supports the `5.5.0+` purpose-based export protocol.
- `ledger-signer-owner`: Clarify that Ledger-owned identity issuance may temporarily export issuance secrets after explicit approval while continuing to store identity payloads under the Ledger owner's local vault.
- `command-taxonomy`: Add a Ledger app inspection command to the documented command surface so users can verify the connected app name/version before Ledger-gated flows.

## Impact

- Affected code: `crates/ccd-wallet/src/commands/identity/new.rs`, `crates/ccd-wallet/src/commands/ledger_construction.rs`, `crates/ccd-wallet/src/commands/ledger.rs`, `crates/ccd-wallet-identity-provider`, `crates/ccd-wallet-ledger`, command taxonomy docs, and Ledger-related tests/mocks.
- Affected APIs: adds a lower-level identity-request construction API that accepts pre-derived issuance material instead of only `ConcordiumHdWallet`; adds low-level Ledger app-version support; refines/splits the new-path private-key export API so app 5.4.1 legacy-new-path semantics are not conflated with later purpose-based semantics.
- Dependencies: reuses and corrects the `ccd-wallet-ledger` crate export commands and owner-verification flow.
- Systems affected: Ledger owner vault usage, identity issuance UX, and non-interactive CLI behavior for Ledger-backed issuance.
- Systems unchanged: wallet database schema, network configuration, browser callback transport, and seed-backed identity issuance behavior.
