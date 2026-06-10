## Context

Protocol-level token and lock commands currently submit one account transaction per user command. The pinned SDK branch exposes lower-level MetaUpdate operation builders and lock composition helpers, including deterministic lock-id prediction for locks created in the next transaction. The CLI already has reusable helpers for network resolution, account signer resolution, account-reference prompts, token amount parsing, lock configuration parsing, confirmation, finalization waiting, and transaction event rendering.

The new composer adds an interactive planning layer above those primitives. Users build a plan on disk through a Reedline command loop, while missing fields and confirmations continue to use `cliclack` so prompt behavior remains consistent with the rest of the CLI.

## Goals / Non-Goals

**Goals:**
- Provide `ccd-wallet token compose <PLAN>` as an interactive way to build a token MetaUpdate composition without hand-writing TOML.
- Persist the plan after every successful `add` command so there are no in-memory-only drafts.
- Support all user-facing token and lock MetaUpdate operation families in a versioned TOML plan format.
- Provide `preview` for operation-list rendering and `submit` for resolving and submitting the plan as one MetaUpdate transaction.
- Support same-plan lock references with `@N`, and accept `@` interactively as shorthand for the most recent preceding lock creation.

**Non-Goals:**
- Providing a full-screen terminal UI.
- Preserving comments or hand formatting in plan files after CLI edits.
- Simulating complete transaction execution before submission.
- Adding browser/connect composition APIs in this change.
- Supporting in-memory-only composition sessions or implicit global active drafts.

## Decisions

### Use a Reedline-backed composer with cliclack field prompts

`token compose <PLAN>` will run a command loop backed by Reedline for line editing, history, completions, Ctrl-C handling, and in-session help. Command handlers will parse inline arguments where provided. When an operation command lacks required non-secret fields, the handler will collect them through existing `cliclack` prompt helpers.

Alternatives considered:
- Pipeline transformers were rejected because they require stdout to remain machine-only and make intermediary interactivity fragile.
- A full TUI was rejected as unnecessary complexity for v1.
- A pure cliclack wizard was rejected because users also want power-user command entry and in-session help.

### Make the plan path explicit and autosaved

The composer requires a plan path. If the file exists, it is loaded and continued; if it does not exist, a new versioned plan is created. After each successful `add`, the CLI atomically writes the full canonical plan to that path. Cancelled prompts, parse errors, and validation errors leave the file unchanged.

This avoids hidden active draft state and ensures `preview`/`submit` always operate on a concrete file.

### Use a versioned canonical TOML plan model

Plans will be represented internally as typed operation structs and serialized as TOML with `version = 1` and an ordered `operations` array. Saving may canonicalize formatting and shorthand references; preserving comments is out of scope.

The implementation should keep parsing/rendering/building separate from the Reedline UI so future non-interactive add commands or other UIs can reuse the same model.

### Resolve same-plan lock references during planning and submission

Lock-create operations are numbered by their order in the plan: `@1`, `@2`, and so on. Interactive input may use `@` to mean the most recent preceding lock creation; before saving, the CLI canonicalizes that shorthand to the explicit `@N` reference. Existing on-chain lock IDs remain valid lock references.

During submission, `@N` is resolved to the deterministic lock id for the Nth lock creation in the composed transaction. The submit path must reject references to non-existent lock creates and references that cannot be resolved from the plan order.

### Treat preview and submit as separate levels of resolution

`compose preview <PLAN>` renders the ordered operation list exactly as the plan describes it. It does not require sender or network context and does not display signed transaction payloads.

`compose submit <PLAN>` resolves sender, network, account labels, token amounts, token metadata needed for decimals, existing locks, and same-plan lock references. It then shows a final confirmation summary and submits one MetaUpdate transaction.

### Validate only transaction-external invariants

Composed transactions can contain state-changing operations that affect later operations in the same transaction, such as minting before funding a lock. The submit path must not preflight conditions that can be changed by earlier operations in the same composition, such as whether the sender currently has enough token balance to fund a lock or burn tokens. Those checks would reject valid composed transactions.

The submit path may validate transaction-external invariants that cannot be created by earlier operations in the same transaction, such as whether referenced existing accounts and tokens exist, whether the sender and network context resolve, whether the sender can cover CCD transaction costs, whether the plan syntax is valid, and whether same-plan `@N` references resolve. The node/protocol remains authoritative for final execution validity.

## Risks / Trade-offs

- **Reedline and cliclack terminal control could conflict** → Run them sequentially only: finish reading a line before invoking any cliclack prompt, then return to Reedline.
- **Autosave could corrupt plans on failure** → Write to a temporary file and rename atomically after serialization succeeds.
- **Plan schema could become hard to evolve** → Include `version = 1` and keep deserialization explicit so future migrations can be added.
- **Same-plan lock prediction depends on nonce freshness** → Resolve lock IDs immediately before signing/submission and present normal submission failure if another transaction consumes the nonce concurrently.
- **Full validation is hard for composed state transitions** → Document and implement best-effort validation only, with final confirmation before submission.
