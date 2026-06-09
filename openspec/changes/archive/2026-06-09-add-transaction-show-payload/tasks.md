## 1. CLI and query plumbing

- [x] 1.1 Add a `--show-payload` flag to `TransactionShowArgs` and extend CLI parsing tests for the new option.
- [x] 1.2 Extend the transaction show query flow to fetch matching block items only when `--show-payload` is requested.
- [x] 1.3 Handle payload-unavailable cases for absent and received transactions without changing the default command behavior.

## 2. Submitted payload rendering

- [x] 2.1 Introduce a rendering model for optional submitted payload sections that stays separate from the existing summary-derived sections.
- [x] 2.2 Implement structured rendering for retrieved block item payloads and add a stable fallback for undecodable payloads.
- [x] 2.3 Ensure committed output can associate submitted payload rendering with each matching committed block section and that finalized output shows a single submitted payload section.

## 3. Verification and documentation

- [x] 3.1 Add unit tests for payload rendering and status-specific behavior across absent, received, committed, and finalized transactions.
- [x] 3.2 Add query-path tests or focused integration coverage for block-item lookup by transaction hash within committed/finalized block contents.
- [x] 3.3 Update any user-facing command documentation that should mention the new `--show-payload` transaction inspection option.
