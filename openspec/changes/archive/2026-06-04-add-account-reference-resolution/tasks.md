## 1. Shared account-reference resolution

- [x] 1.1 Add a shared account-reference resolver that accepts raw addresses or finalized local account labels in resolved network context.
- [x] 1.2 Add a command-scoped unlock context that can cache unlocked derived-seed and imported-account-vault domains for later account-address resolution.
- [x] 1.3 Add singular and repeated helper APIs for resolving non-sender account references, including lock-grant account segments.
- [x] 1.4 Add tests for address-first precedence, finalized-label lookup, missing-label errors, and unlock reuse across repeated resolutions.

## 2. Interactive prompt integration

- [x] 2.1 Add a `cliclack` autocomplete input flow for prompted account references that suggests finalized local accounts on the resolved network.
- [x] 2.2 Render autocomplete suggestion strings with ownership decoration using `[seed] label` and `[imported] label` and map selected suggestions back to local account records.
- [x] 2.3 Add tests for prompted account-reference suggestion rendering and raw-address acceptance.

## 3. Token and contract command adoption

- [x] 3.1 Switch token recipient/target/source resolution helpers to the shared account-reference resolver for singular inputs.
- [x] 3.2 Switch repeated token recipient/target parsing and lock-grant account parsing to the shared account-reference resolver.
- [x] 3.3 Update `contract invoke --invoker` to accept a raw address or finalized local account label through the shared resolver.
- [x] 3.4 Add command-focused tests covering token transfer, list updates, lock create/send/return, and contract invoke with local account labels.

## 4. Verification and docs

- [x] 4.1 Update command help text and any relevant user-facing docs to say these inputs accept local account labels as well as raw addresses.
- [x] 4.2 Run the relevant Rust formatting, lint, and test commands for the touched command flows and address any failures.
