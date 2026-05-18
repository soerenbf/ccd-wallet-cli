## Why

The wallet can now manage governance keys, but it still cannot use them to create, sign, and submit chain governance updates. The next step is to let operators drive governance updates directly from the CLI while preserving explicit signer choice, payload verification where possible, and a forward-compatible blind-signing path for future serialized update types.

## What Changes

- Add `ccd-wallet governance update` for signing and submitting governance updates from either JSON payload files or serialized hex payloads.
- Support Create PLT JSON payloads whose `initializationParameters` are supplied as JSON, converting only that field to Concordium CBOR before signing.
- Support interactive payload entry when a payload is not supplied on the command line: pasted JSON for `--json` mode and pasted hex for `--serialized` mode.
- Deserialize known payloads to determine update type, required authorization family, threshold, sequence-number queue, and human-readable verification output.
- Support explicit signer selection through repeatable `--key <VERIFY_KEY>` flags and interactive fuzzy multi-select when keys are omitted.
- Preselect authorized governance keys up to the required threshold when the authorization structure is known from chain state.
- Support blind signing of unknown serialized payloads with strong warnings, explicit signer choice, and explicit sequence-number override.
- Support an optional `--sign-as <AUTH_FAMILY>` helper for unknown serialized payloads so the CLI can derive thresholds, eligible keys, and default sequence numbers when the operator knows the authorization family even if the payload itself is not yet understood by the wallet.
- Prompt for effective time when it is omitted, defaulting to `0`, and prompt for timeout when it is omitted, defaulting to five minutes before a scheduled effective time (or five minutes from now when effective time is `0`). Both inputs support relative durations, RFC3339 datetimes, and unix seconds.
- Wait for finalization by default after submission, with `--no-wait` to return after successful submission.

## Capabilities

### New Capabilities
- `governance-update-submission`: Create, sign, verify, submit, and optionally wait for finalization of governance updates from JSON or serialized payloads, including blind-sign support for unknown serialized payloads.

### Modified Capabilities
- `governance-key-management`: Governance update flows reuse stored governance keys for explicit signer selection, including interactive fuzzy multi-select and authorization-aware key presentation.
- `node-connectivity`: Governance update submission depends on live chain queries for authorization structures, key-index resolution, and next update sequence numbers when those values are not explicitly overridden.

## Impact

- New CLI surface under `crates/ccd-wallet/src/commands/governance.rs` and corresponding clap definitions in `crates/ccd-wallet/src/cli.rs`.
- Reuse and extension of governance-key listing/removal presentation logic for update signer selection.
- Additional Concordium node query usage for chain parameters, key-index lookup, update-sequence-number lookup, submission, and optional finalization waiting.
- New parsing and prompting logic for governance payload inputs, effective-time / timeout inputs, blind-sign warnings, and submission result handling.
