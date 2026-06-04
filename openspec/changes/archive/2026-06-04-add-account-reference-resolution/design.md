## Context

`ccd-wallet` already has two distinct account-resolution patterns:

- sender-capable account selection resolves a finalized local account record, unlocks its signing material, and builds a signer wallet
- most non-sender transaction-detail fields still parse only raw `AccountAddress` values

That split is visible in the current code:

- `commands/account.rs` already resolves local account labels in network context and renders seed-aware account selectors
- `commands/token/shared.rs` still uses raw-address-only helpers for recipient/target/source fields
- `commands/contract/invoke.rs` still parses `--invoker` as a raw address only
- `account show` already demonstrates the intended precedence rule of `raw address first, then local label`

This change is cross-cutting because it touches shared command-resolution paths, interactive prompts, and transaction-detail inputs across multiple command families. It also crosses encryption domains: resolving a local account label may require decrypting a derived account payload through a seed password or an imported account payload through an imported-account-vault password.

## Goals / Non-Goals

**Goals:**
- Introduce one shared account-reference resolution path for non-sender command inputs.
- Accept either a raw account address or a finalized local account label in resolved network context.
- Reuse already-unlocked ownership domains within one command so same-seed or same-imported-vault lookups do not reprompt.
- Keep interactive account-reference prompts discoverable by showing local-account autocomplete suggestions with seed/imported ownership hints while still allowing pasted raw addresses.
- Preserve the current address-first precedence rule already used by `account show`.

**Non-Goals:**
- Change sender account selection semantics or replace the existing signer-account selector flow.
- Change wallet persistence, encryption primitives, or vault ownership rules.
- Support pending local accounts as account references for transaction details.
- Introduce fuzzy multi-column autocomplete widgets beyond what `cliclack` already supports.
- Retroactively rewrite every future address-like input; this design covers the currently implemented command surface plus a shared structure that future commands can reuse.

## Decisions

### 1. Introduce a shared `account reference` resolver centered on network-scoped local accounts
A new shared resolver will accept:

- an explicit CLI value, or
- a missing value that must be prompted interactively

and return a resolved `AccountAddress` plus metadata about how it was resolved.

Resolution order:
1. try parsing the value as a raw `AccountAddress`
2. if parsing fails, look up a finalized local account by label in the resolved network
3. if neither path succeeds, return an actionable error

The resolver is network-scoped because local account labels are unique per network, which avoids cross-seed ambiguity once network context is known.

Alternatives considered:
- label-first lookup: rejected because `account show` already uses address-first precedence and raw addresses must remain unambiguous
- command-local token-only helpers: rejected because `--invoker` and future address-like fields need the same behavior

### 2. Treat derived seeds and imported-account vaults as the same kind of reusable ownership domain
Unlock reuse will be modeled around the domain that owns the encrypted account payload, not around sender special cases.

That yields two reusable domain kinds:
- derived account domain: keyed by `seed_id`
- imported account domain: keyed by network-scoped imported vault identity

When the resolver needs a local account address:
- if the owning domain is already unlocked in the current command context, reuse the cached DEK without prompting again
- otherwise prompt for the appropriate seed or imported-vault password, unlock once, and cache the result for the remainder of the command

This generalizes the desired same-seed optimization and also covers repeated imported-account resolution naturally.

Alternatives considered:
- special-case only `same sender seed`: rejected because it does not cover imported accounts or multiple non-sender lookups cleanly
- always reprompt for non-sender labels: rejected because it weakens the UX improvement that motivates the change

### 3. Pass a short-lived local account unlock context through mutating command flows
Commands that already resolve a signer account will carry a short-lived unlock context alongside their existing signer/network/client context. Non-sender account-reference resolution will consume that same context.

The context will hold only in-memory unlocked material needed to decrypt local account payloads during the current command execution. It will not change persistence or share decrypted state across commands.

This keeps reuse explicit and command-scoped while avoiding global caches.

Alternatives considered:
- global process-wide unlock cache: rejected because it broadens secret lifetime and complicates invalidation
- re-unlock from scratch per field: rejected because it duplicates prompts and decryption work

### 4. Interactive prompts will use `cliclack` input autocomplete with decorated local-account suggestions
When an account-reference value is omitted in interactive mode, the CLI will prompt with a text input that supports autocomplete suggestions built from finalized local accounts on the resolved network.

Rendered suggestions will use the existing ownership style:
- derived: `[seed-label] account-label`
- imported: `[imported] account-label`

The prompt will still accept any pasted raw account address. To bridge `cliclack`'s string-based autocomplete model, the resolver will maintain an exact mapping from decorated suggestion strings back to the underlying local account record while still accepting plain label input and raw address input.

Alternatives considered:
- selector plus separate "enter raw address" branch: cleaner technically, but worse for the intended single-prompt paste-or-select experience
- plain-label autocomplete without ownership decoration: rejected because it hides the seed/imported context that makes the prompt trustworthy

### 5. Only finalized local accounts participate in account-reference resolution
Local account labels used as account references must resolve only to finalized accounts. Pending accounts will be excluded from autocomplete suggestions and label lookup results for transaction-detail fields.

This avoids offering references that cannot be relied on as stable on-chain recipients, sources, targets, or invokers.

Alternatives considered:
- allow pending accounts if a decrypted address exists: rejected because pending status still makes the UX and submission semantics confusing

### 6. Apply the shared resolver broadly across current non-sender account inputs
The shared resolver will be used anywhere the current CLI accepts a non-sender account address in implemented command surfaces, including:

- token transfer recipient
- token admin-role target
- token allow-list / deny-list targets
- token lock create recipients and grant account references
- token lock send source and recipient
- token lock return source
- contract invoke `--invoker`

This keeps the command surface internally consistent instead of making only some address-like fields label-aware.

Alternatives considered:
- start with singular prompted fields only: useful as a spike, but inconsistent for users and contrary to the requested general structure

## Risks / Trade-offs

- **[Autocomplete suggestions are string-based rather than structured items]** → Mitigation: keep a strict exact-string mapping for decorated suggestions and still support plain label lookup as a fallback.
- **[Unlock reuse increases in-memory secret lifetime within a command]** → Mitigation: scope the cache to one command execution only and keep it limited to already-needed decrypted domain keys.
- **[Cross-cutting helper changes could tangle sender and non-sender flows]** → Mitigation: keep sender-account selection intact and layer the new resolver beside it, sharing only the unlock context.
- **[Repeated explicit values like multiple `--target` or `--recipient` flags need consistent parsing]** → Mitigation: expose both singular and repeated shared helpers over the same underlying account-reference resolver.
- **[Imported-account prompts differ from derived-seed prompts]** → Mitigation: model both through domain ownership and centralize prompt text generation in the shared resolver.

## Migration Plan

- Add the new change artifacts and spec deltas.
- Introduce the shared account-reference resolution primitives and command-scoped unlock context.
- Switch existing token and contract address-like inputs from raw-address-only parsing to the shared resolver.
- Update any command help text and docs affected by the broadened input semantics.
- Rollback is additive and straightforward: revert the affected commands to raw-address-only parsing and remove the shared resolver/context if needed.

## Open Questions

All user-facing exploration questions needed for proposal scope are resolved for this change:

- raw address vs label precedence: raw address first
- imported accounts participate: yes
- pending accounts participate: no
- scope: all currently implemented non-sender account inputs
- prompt decoration: `[seed] label` and `[imported] label`

One implementation-level detail remains intentionally flexible during apply:
- whether the unlock cache is carried as a dedicated struct inside existing context objects or as a separate resolver context passed alongside them
