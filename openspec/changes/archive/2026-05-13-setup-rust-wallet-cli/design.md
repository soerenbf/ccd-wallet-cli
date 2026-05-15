## Context

The repository is currently empty, and this change establishes the first implementation foundation for a Concordium wallet CLI written in Rust. The immediate need is not full wallet functionality, but a project skeleton that can compile, expose a stable command-line surface, and prove that the application can connect to a Concordium node through the official Rust SDK.

This is an architectural change because it introduces the primary language toolchain, runtime model, dependency stack, and command layout that later wallet features will extend. The Concordium SDK is centered around an async client, so the project should adopt an async-first structure from the start rather than retrofit it later.

## Goals / Non-Goals

**Goals:**
- Establish a single Rust Cargo project that builds a `ccd-wallet` binary.
- Introduce the Concordium Rust SDK as the supported node integration layer.
- Provide a minimal read-only node command that validates connectivity and SDK usage.
- Define a small but maintainable application structure for commands, configuration, and shared error handling.
- Document enough local workflow for future implementation changes to build, lint, and run the project.

**Non-Goals:**
- Implement wallet creation, import, key storage, signing, or transaction submission.
- Design a plugin system, workspace split, or multi-binary architecture.
- Add persistent storage, database integration, or advanced configuration files.
- Support the full Concordium feature set in the initial setup change.

## Decisions

### Use a single binary Cargo package for the initial project
The project will start as one Cargo package that produces the `ccd-wallet` executable instead of a multi-crate workspace.

- **Why:** The repository has no existing code, and the first change should optimize for momentum and simplicity. A single package is enough for initial commands and keeps setup friction low.
- **Alternative considered:** Start with a workspace containing separate CLI and library crates. This was rejected for the first change because it adds structure before there is enough code pressure to justify it.

### Build the CLI around `clap` derive and an async `tokio` runtime
The command-line interface will use `clap` derive APIs, and `main` will run on Tokio.

- **Why:** `clap` is the standard Rust choice for structured command hierarchies and help output, and Tokio matches the async execution model used by the Concordium SDK.
- **Alternative considered:** Use synchronous command handlers or lighter argument parsers. This was rejected because node access already requires async, so a synchronous shell would only introduce glue code.

### Standardize on the Concordium Rust SDK for all node communication
All node connectivity in this change will go through `concordium-rust-sdk`, using its v2 client APIs.

- **Why:** The user explicitly wants the CLI to integrate through the Concordium Rust SDK, and using the official SDK reduces protocol drift while giving later changes a direct path toward wallet and transaction features.
- **Alternative considered:** Call node endpoints directly over lower-level gRPC or HTTP libraries. This was rejected because it would duplicate concerns the SDK already solves.

### Prove connectivity with a read-only node inspection command
The initial CLI will include a basic command such as `node info` that connects to a configured endpoint and prints node or consensus information.

- **Why:** A read-only command provides an observable proof that the scaffold works without introducing wallet state, keys, or transaction risk.
- **Alternative considered:** Limit the first change to project creation and dependency declaration only. This was rejected because it would not verify that the SDK integration actually works end to end.

### Keep configuration lightweight with flag/env resolution
Node endpoint selection will be resolved from command-line flags first, then an environment variable, then a development-friendly default.

- **Why:** This keeps the first version easy to use locally while establishing a predictable configuration pattern for later commands.
- **Alternative considered:** Introduce a configuration file format immediately. This was rejected because file-based config is not yet necessary for the first bootstrap change.

### Add structured diagnostics from the start
The project will include error-context handling and structured logging/tracing support.

- **Why:** Networked CLI tools fail in many ways, and early diagnostics support makes future implementation and debugging easier.
- **Alternative considered:** Use plain `println!` and bare errors initially. This was rejected because it creates avoidable cleanup work in later changes.

## Risks / Trade-offs

- **[SDK version compatibility]** The Concordium SDK may have specific node-version or Rust-version expectations. → **Mitigation:** Pin or intentionally select a compatible SDK release and document local prerequisites.
- **[Default endpoint assumptions]** A baked-in default endpoint may not match every developer environment. → **Mitigation:** Support explicit override via flag and environment variable and keep the default clearly documented.
- **[Bootstrap scope creep]** Initial setup work can easily expand into implementing real wallet features. → **Mitigation:** Keep this change limited to project scaffold, configuration baseline, and a read-only connectivity command.
- **[Premature architecture]** Even a modest structure can be too abstract for an empty repository. → **Mitigation:** Use only a minimal module split around commands and config, leaving deeper refactors for later changes when usage patterns are clearer.

## Migration Plan

- No production migration is required because this is a new repository.
- After implementation, validate the bootstrap by running the build, lint, and a read-only node command against a reachable Concordium node.
- Future changes can extend the established CLI structure instead of reworking project setup.

## Open Questions

- Should the initial read-only verification command print node info, consensus info, or both?
- Should a future change promote shared logic into a library crate once wallet operations are added?
