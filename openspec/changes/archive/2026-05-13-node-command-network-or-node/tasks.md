## 1. CLI Arguments

- [x] 1.1 Add `--network <NAME>` (`Option<String>`) to `NodeInfoArgs` with `conflicts_with = "node"`.
- [x] 1.2 Add `required_unless_present` on `--node` so clap enforces that at least one flag is supplied.

## 2. Endpoint Resolution

- [x] 2.1 Implement a `resolve_endpoint` helper that takes `Option<String>` (network name) and `Option<v2::Endpoint>` (explicit node), loads the config store when needed, and returns a `(v2::Endpoint, String)` label pair or an actionable error.
- [x] 2.2 Handle the "network not registered" error case with a message prompting the user to run `config network add`.

## 3. Node Info Handler

- [x] 3.1 Update the `info` handler in `src/commands/node.rs` to call `resolve_endpoint` and use the returned endpoint.

## 4. Validation

- [x] 4.1 `cargo fmt --check` and `cargo clippy` clean.
- [x] 4.2 Validate: `node info --node http://127.0.0.1:20001` still works.
- [x] 4.3 Validate: `node info --network local` resolves and connects (requires a registered network named `local`).
- [x] 4.4 Validate: `node info --network unknown` exits non-zero with a clear error.
- [x] 4.5 Validate: `node info --network local --node http://...` exits non-zero with a conflict error.
- [x] 4.6 Validate: `node info` with neither flag exits non-zero with a missing-argument error.
