## Context

The repository already has a growing CLI surface in `crates/ccd-wallet/src/cli.rs`, but it does not yet have a single document that explains the intended command taxonomy across implemented and planned spaces. The next round of transaction work will add protocol-version-11 token features from the pinned Concordium Rust SDK branch, including `MetaUpdate`, token admin-role updates, token metadata updates, and protocol-level lock operations. Those protocol payload names are useful implementation details, but they are not good primary user-facing command names.

The immediate deliverable is documentation rather than command implementation: a canonical `docs/commands.md` file and an `AGENTS.md` rule that keeps command code and command documentation synchronized. Because future implementation work will build on this taxonomy, the design needs to distinguish user-facing command groups from protocol-level payload types, separate currently implemented commands from planned commands, and avoid reviving deprecated legacy baker transaction families that are not relevant on recent protocol versions.

## Goals / Non-Goals

**Goals:**
- Establish `docs/commands.md` as the canonical reference for the wallet CLI command taxonomy.
- Document the intended top-level command spaces and the expected nested grouping for the next major areas, especially `token` and `stake`.
- Keep the validator side of the staking taxonomy focused on modern `ConfigureBaker`-style flows and exclude deprecated legacy baker transaction families from the documented surface.
- Keep protocol payload terminology such as `TokenUpdate` and `MetaUpdate` out of the primary command path unless a future user-facing need emerges.
- Define a synchronization rule so changes to command code or command taxonomy update `docs/commands.md` together.
- Leave room in the documented taxonomy for future token-operation composition without prematurely freezing one concrete composition syntax.

**Non-Goals:**
- Implementing new command handlers or changing the live CLI in this change.
- Finalizing a pipe-based or file-based composition UX for token operations.
- Exhaustively documenting every flag for every existing command.
- Defining protocol semantics beyond what is needed to explain command grouping and naming.

## Decisions

### 1. Use `docs/commands.md` as the canonical command taxonomy document
The project will add a dedicated command-taxonomy document in `docs/commands.md` rather than spreading command-shape decisions across proposals, PR descriptions, and inline code comments.

**Rationale:**
- The repo already uses `docs/` for design-oriented reference documents.
- A stable documentation location makes it easier to review command-surface changes before or alongside implementation.
- Future contributors need one source of truth for planned and implemented command groups.

**Alternatives considered:**
- Put the taxonomy in `README.md`: rejected because it would compete with onboarding material and get too long.
- Rely on clap help output as the source of truth: rejected because it only reflects implemented commands and does not capture intended future structure.

### 2. Keep `token` as the user-facing namespace for token, metadata, role, and lock operations
The taxonomy document will place protocol-level token send, mint/burn, list management, pause/unpause, admin-role changes, metadata updates, and lock operations under the `token` command space, using nested groups such as `token metadata update`, `token roles grant`, and `token lock create`.

**Rationale:**
- The SDK now models token and lock work as a connected family, with `MetaUpdate` acting as a superset envelope over token operations.
- Users think in terms of “working with a token”, not in terms of choosing between `TokenUpdate` and `MetaUpdate` payload kinds.
- This keeps room for future token-operation composition without introducing `metaupdate` as a command-space concept.

**Alternatives considered:**
- Add a `metaupdate` or `token metaupdate` command space: rejected because it exposes protocol transport terminology instead of the user’s task.
- Split locks into a top-level `lock` space: rejected because locks are scoped to protocol-level token workflows and are modeled that way in the SDK.

### 3. Document `stake` as an umbrella over validator and delegation flows
The taxonomy document will describe staking as a grouped space with separate validator and delegation branches rather than a flat list of stake actions.

**Rationale:**
- `ConfigureBaker` and `ConfigureDelegation` are distinct capabilities with different concepts and options.
- A grouped `stake validator ...` / `stake delegation ...` structure keeps related flows together while avoiding an overloaded flat `stake` surface.
- Recent protocol versions use `ConfigureBaker` for validator management, so documenting legacy baker transaction families would create an outdated and misleading taxonomy.

**Alternatives considered:**
- Separate top-level `validator` and `delegation` spaces: viable, but less aligned with the desire to think in terms of a broad staking area.
- Flat `stake` subcommands: rejected because validator-specific controls like metadata, commissions, and suspension do not read naturally as generic “stake” actions.
- Include deprecated `AddBaker` / `RemoveBaker` / related legacy baker flows in the validator branch: rejected because those transactions are deprecated and not part of the intended modern CLI surface.

### 4. Explicitly separate implemented commands from planned commands in the document
`docs/commands.md` should mark whether a command space or branch is implemented today or planned for later work.

**Rationale:**
- The document is meant to guide future implementation, not just describe current code.
- Without status labeling, the document could mislead contributors or users about what already exists.

**Alternatives considered:**
- Document only implemented commands: rejected because it would not help guide the current taxonomy proposal.
- Document planned commands without status labels: rejected because it would invite drift and confusion.

### 5. Add an `AGENTS.md` rule that command code and command docs must move together
The repository guidance will explicitly require `docs/commands.md` and command-surface code to be updated in the same change whenever the CLI structure changes.

**Rationale:**
- Command drift is the main failure mode for this kind of document.
- Putting the rule in `AGENTS.md` makes it part of normal contributor workflow and review expectations.

**Alternatives considered:**
- Keep the rule only in `docs/commands.md`: rejected because contribution guidance belongs in the repository rules as well.
- Rely on reviewer memory: rejected because it is inconsistent and does not scale.

## Risks / Trade-offs

- **[Documentation drift]** The command tree may diverge from the code over time. → **Mitigation:** add an explicit `AGENTS.md` sync rule and make `docs/commands.md` the review reference for command-surface changes.
- **[Premature taxonomy lock-in]** Future protocol or UX discoveries may force changes to the documented structure. → **Mitigation:** mark planned sections clearly and keep the document focused on command-space boundaries, not rigid flag-level syntax.
- **[Legacy naming leakage]** Contributors may reintroduce deprecated baker transaction names into the staking taxonomy out of familiarity with older APIs. → **Mitigation:** state explicitly that the validator branch is modeled on modern `ConfigureBaker`-based flows only.
- **[Confusion between user-facing names and protocol payloads]** Contributors may still try to expose `TokenUpdate` / `MetaUpdate` in the command path. → **Mitigation:** document the distinction directly and explain that payload kind is an implementation detail.
- **[Composition syntax churn]** A future pipe-based composition model may evolve after more experimentation. → **Mitigation:** document only the namespace reservation and intent for composition, not a final syntax contract.

## Migration Plan

1. Add `docs/commands.md` with the canonical command taxonomy and status labels for implemented versus planned areas.
2. Update `AGENTS.md` to require synchronized changes between command-surface code and `docs/commands.md`.
3. Use the document as the reference input for later command implementation proposals and code changes.
4. If the documented taxonomy changes during implementation, update the document in the same change before merge.

## Open Questions

- Should the future composition workflow be documented under `token op ...` / `token submit`, or should the document only reserve composition as a future concern without naming commands yet?
- Should the taxonomy document include aliases or legacy compatibility names if the implementation later adopts them?
- How much of the current implemented command tree should be copied into `docs/commands.md` versus summarized at the command-space level?