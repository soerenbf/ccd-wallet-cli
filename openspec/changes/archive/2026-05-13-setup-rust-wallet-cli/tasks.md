## 1. Project Scaffold

- [x] 1.1 Initialize the Cargo package and create the initial Rust source layout for the `ccd-wallet` binary.
- [x] 1.2 Add the Concordium Rust SDK plus the async runtime, CLI parsing, and diagnostics dependencies required by the bootstrap spec.
- [x] 1.3 Add baseline developer documentation describing how to build, lint, and run the CLI locally.

## 2. CLI Foundation

- [x] 2.1 Implement the root CLI structure and a `node` command group using the chosen command parser.
- [x] 2.2 Implement node endpoint resolution with command-line override, environment-variable fallback, and a documented default for local development.
- [x] 2.3 Add shared application startup concerns such as error-context handling and logging/tracing initialization.

## 3. Concordium Node Integration

- [x] 3.1 Implement a read-only node command that creates a Concordium SDK client and queries node information from the configured endpoint.
- [x] 3.2 Format successful node responses for terminal output and return actionable failures for invalid or unreachable endpoints.
- [x] 3.3 Validate the bootstrap by running the documented build/lint flow and exercising the read-only node command against a reachable node.
