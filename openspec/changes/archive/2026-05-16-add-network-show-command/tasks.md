## 1. CLI surface and resolution

- [x] 1.1 Extend the network CLI surface with a `show` subcommand and arguments for label selection, `--node <ENDPOINT>`, `--no-defaults`, and `--non-interactive` as needed.
- [x] 1.2 Implement endpoint resolution for the three supported modes: active/default config mode, explicit-label config mode, and explicit `--node` node-only mode.
- [x] 1.3 Support explicit label plus `--node` override mode without silently deriving additional configured-network context from active state.

## 2. Query and matching logic

- [x] 2.1 Query consensus information from the resolved endpoint and extract the observed genesis hash together with the curated consensus fields chosen for display.
- [x] 2.2 Implement configured-network matching by observed genesis hash so node-only mode can report zero, one, or multiple matching aliases, using compact alias-plus-endpoint rows for multi-match rendering.
- [x] 2.3 Add config-vs-observed mismatch diagnostics for explicit-label mode when the queried node does not match the configured genesis hash.

## 3. Output, tests, and documentation

- [x] 3.1 Render config mode with `Network configuration` followed by `Consensus (<node endpoint>)`, and keep that rendering even when an explicit `--node` override is supplied together with a label.
- [x] 3.2 Render node-only mode with `Network match(es) (<genesis hash>)` followed by `Consensus (<node endpoint>)`, using compact alias-plus-endpoint rows for multiple matches and an explicit no-match summary otherwise.
- [x] 3.3 Add command-level tests covering active-network default behavior, node-only mode, multiple-match rendering, no-match behavior, and explicit-label-plus-override mismatch warnings.
- [x] 3.4 Update README and command documentation for `network show` modes and run formatting, linting, and relevant tests.
