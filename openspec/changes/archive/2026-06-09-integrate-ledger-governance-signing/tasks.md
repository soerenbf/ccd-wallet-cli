## 1. CLI surface and dependencies

- [x] 1.1 Add the `ccd-wallet-ledger-governance` dependency to the wallet CLI crate with the feature set needed for governance SDK conversions.
- [x] 1.2 Extend `GovernanceUpdateArgs` with `--ledger` and a `--ledger-key-index <N>` override argument, including clap conflicts for unsupported local-key combinations and any unsupported multi-signer selectors.
- [x] 1.3 Add CLI parsing tests for Ledger mode, Ledger key-index override, rejected mixed local/Ledger signer selection, and rejected unsupported multi-signer Ledger input.

## 2. Prepared update and signer output foundation

- [x] 2.1 Extract governance update preparation into an internal value containing resolved payload, encoded payload, update header, timing, sequence number, and chain authorization context.
- [x] 2.2 Introduce an internal signer-output representation for indexed raw governance update signatures.
- [x] 2.3 Adapt the existing local governance key vault signing path to consume the prepared update and produce signer outputs without changing current behavior.
- [x] 2.4 Refactor signed update assembly so it builds the final update instruction from prepared update data plus signer outputs.
- [x] 2.5 Add tests that local governance signing still enforces signer authorization, thresholds, blind-sign behavior, timing, and signed update assembly as before.

## 3. Ledger signer resolution

- [x] 3.1 Determine the Governance Ledger purpose from the prepared update authorization family (`0` root, `1` level 1, `2` level 2).
- [x] 3.2 Construct `ccd-wallet-ledger-governance::DerivationPath` values from the derived governance purpose plus a default key index of `0` or an explicit `--ledger-key-index <N>` override.
- [x] 3.3 Implement interactive prompting for the Ledger key index when `--ledger` is used without an explicit override in interactive mode, defaulting the prompt to `0`.
- [x] 3.4 Fetch the Ledger governance public key for the selected derived path before update signing where practical.
- [x] 3.5 Map the Ledger public key to the current on-chain governance key index and reject unauthorized signers before submission.
- [x] 3.6 Reject all-in-one Ledger mode when the update-family threshold exceeds one and report the supported signer count versus the required threshold.

## 4. Ledger update signing backend

- [x] 4.1 Reject Ledger signing for blind/unknown serialized governance payloads before opening the device signing flow.
- [x] 4.2 Convert each supported prepared typed governance update payload into the corresponding `ccd-wallet-ledger-governance` request type, including update header bytes and update type tags.
- [x] 4.3 Implement Ledger signing dispatch for fixed-shape governance update families.
- [x] 4.4 Implement Ledger signing dispatch for staged/chunked governance update families such as protocol update, add anonymity revoker, add identity provider, and create PLT.
- [x] 4.5 Implement Ledger signing dispatch for root-key, level-1-key, and level-2 authorization update flows.
- [x] 4.6 Collect the raw Ledger signature for the single selected derived path and convert it into an indexed signer output.
- [x] 4.7 Map Ledger user-decline and transport/protocol failures into actionable CLI errors.

## 5. Submission flow integration and UX

- [x] 5.1 Route `governance update --ledger` through the Ledger signer backend while preserving existing network resolution, payload parsing, timing prompts, sequence-number resolution, submission, and finalization behavior.
- [x] 5.2 Use `cliclack` progress messages or spinners for Ledger public-key lookup, device signing, node submission, and finalization.
- [x] 5.3 Verify that `--no-wait` works the same for Ledger-signed updates as for locally signed updates.
- [x] 5.4 Add integration-style tests or focused unit tests for successful Ledger signer mapping, threshold-above-one rejection, unauthorized Ledger key rejection, blind-sign rejection, and user-decline errors using mockable Ledger boundaries.

## 6. Documentation and verification

- [x] 6.1 Update `docs/commands.md` to document `governance update --ledger`, derived governance purpose plus `--ledger-key-index <N>`, exclusivity from local governance key vault signing, single-signer scope, and lack of blind Ledger signing.
- [x] 6.2 Review command help text and user-facing errors for consistent terminology: local governance key vault, Governance Ledger app, governance purpose, Ledger key index, and exclusive signing mode.
- [x] 6.3 Run Rust formatting for the affected crates.
- [x] 6.4 Run targeted wallet CLI tests and the Governance Ledger crate tests.
- [x] 6.5 Run relevant workspace checks to ensure existing governance local-signing behavior remains intact.
