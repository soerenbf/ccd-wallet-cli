# AGENTS.md

- This repo contains a Rust workspace (`Cargo.toml`, `crates/`) and a pnpm workspace (`package.json`, `packages/`). Keep changes scoped to the relevant ecosystem and avoid coupling Cargo and pnpm workflows unnecessarily.
- Public crate/package functionality should be documented to release quality.
- Public types should have a top-level description; structured fields should have field-level descriptions.
- Public functions/methods should have a description, parameter docs, return docs, error docs where relevant, and examples unless usage is self-explanatory.
- Source modules should have a short top-level docstring/comment describing the file’s responsibility.
- Use Rust doc comments for Rust public APIs and JSDoc for TypeScript public APIs.
- Browser-facing TypeScript packages should stay environment-flexible and avoid Node-specific runtime assumptions unless intentionally Node-only.
