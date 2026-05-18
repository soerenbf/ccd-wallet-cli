## 1. CLI Surface and Input Parsing

- [x] 1.1 Add `governance update` clap arguments for `--json`, `--serialized`, repeatable `--key`, optional `--sign-as`, optional `--sequence-number`, effective time, timeout, and `--no-wait`.
- [x] 1.2 Implement governance update payload ingestion for JSON files, serialized hex, and interactive paste fallback for both modes.
- [x] 1.3 Implement parsing helpers for effective time and timeout inputs covering relative durations, RFC3339 timestamps, and unix seconds.
- [x] 1.4 Implement interactive prompting defaults for omitted effective time (`0`) and omitted timeout (effective time - 5 minutes, or now + 5 minutes when effective time is `0`).
- [x] 1.5 Add validation for future timeout and timeout-versus-effective-time invariants before submission.

## 2. Payload Resolution and Signing Context

- [x] 2.1 Implement deserialization of known governance update payloads and mapping from decoded payload types to authorization families and default sequence-number queues.
- [x] 2.2 Implement blind-sign handling for unknown serialized payloads, including explicit warning flows and support for manual signing with explicit keys and explicit sequence number.
- [x] 2.3 Implement optional `--sign-as` handling for blind payloads so the CLI can derive threshold, eligible keys, and default sequence number behavior when the operator supplies an authorization-family hint.
- [x] 2.4 Implement live chain queries for key-index resolution, authorization structures, and next update sequence numbers when they are needed and not explicitly overridden.

## 3. Signer Selection UX and Submission

- [x] 3.1 Reuse governance-key-management rendering helpers for governance update signer presentation, including compact verify keys, tag-first rows, and capability-aware summaries.
- [x] 3.2 Implement interactive fuzzy multiselect signer prompts with threshold-aware preselection when the authorization structure is known.
- [x] 3.3 Implement update-instruction construction, signing with selected governance keys, submission to the node, and transaction-hash reporting.
- [x] 3.4 Implement default wait-for-finalization behavior plus `--no-wait` early return.

## 4. Tests and Documentation

- [x] 4.1 Add tests for JSON and serialized payload ingestion, blind-sign warnings, `--sign-as` helper behavior, and explicit sequence-number override.
- [x] 4.2 Add tests for timing-input parsing, prompting defaults, and validation, including relative durations, RFC3339, unix seconds, and timeout invariants.
- [x] 4.3 Add tests for signer-selection ordering, threshold-aware preselection, key-index resolution, and finalization waiting vs `--no-wait`.
- [x] 4.4 Update README and command documentation with governance update examples for decoded JSON payloads, serialized payloads, blind signing, explicit key selection, and effective-time/timeout formats.
