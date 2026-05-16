## 1. Recovery foundations

- [x] 1.1 Add wallet-proxy/provider metadata plumbing for recovery-capable identity providers, including `recoveryStart` handling.
- [x] 1.2 Extend the identity-provider client crate with a recovery request flow that returns recovered identity objects or recoverable misses.
- [x] 1.3 Add core recovery orchestration that derives identity recovery requests from a stored seed and scans selected providers with bounded parallelism.
- [x] 1.4 Add account-discovery orchestration that derives credential registration IDs for recovered identities and queries the node for matching accounts with bounded parallelism.
- [x] 1.5 Add shared recovery state aggregation for concurrent workers, cancellation, and partial-failure collection.

## 2. Store import helpers

- [x] 2.1 Add identity-store helpers for idempotent import/update of recovered completed identities by derivation tuple.
- [x] 2.2 Add account-store helpers for idempotent import/update of recovered confirmed accounts by derivation tuple.
- [x] 2.3 Add deterministic fallback labeling helpers for newly imported recovered identities and accounts.

## 3. CLI command flows

- [x] 3.1 Extend the CLI argument surface with `seed sync`, repeated `--provider <VALUE>`, `--provider all`, and `seed add --restore <NETWORK>`.
- [x] 3.2 Implement recovery scope resolution for seed/network defaults, explicit provider filters, interactive prompts, and non-interactive errors.
- [x] 3.3 Implement interactive provider multiselect together with explicit-provider behavior and `all` exclusivity validation.
- [x] 3.4 Implement the add-and-restore convenience flow so successful seed creation can immediately invoke recovery.

## 4. Recovery UX and reporting

- [x] 4.1 Add cliclack-based aggregate recovery progress UI with provider-level progress, worker-state counts, active-work snapshots, and live discovery counters.
- [x] 4.2 Add final recovery summaries that distinguish recovered identities/accounts from skipped or failed providers.
- [x] 4.3 Ensure context-bearing recovery commands print resolved seed/network context before prompting for provider selection.

## 5. Validation

- [x] 5.1 Add tests for recovery URL handling, provider skipping, and recoverable provider/account misses.
- [x] 5.2 Add tests for idempotent recovered identity/account imports and generated-label behavior.
- [x] 5.5 Run `cargo fmt`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace`.
