# Protocol references and discrepancies

The Governance Ledger app protocol should be implemented against these references in priority order:

1. Governance Ledger app source code.
2. Governance Ledger end-to-end tests.
3. Markdown instruction documents.

The markdown instruction documents are valuable, but they can drift from the implemented app. During implementation, source and tests are treated as the primary source of truth.

Known discrepancy to keep in mind:

- The markdown documentation for level-2 authorizations mentions instruction `0x2C` for one authorization variant, while the Governance Ledger app source and tests use `0x2B` for level-2 authorization updates with level-1 keys. The crate follows source/test-backed instruction constants for the public authorizations methods.

The implementation keeps instruction constants explicit and tests representative APDU sequences with `MockTransport`.
