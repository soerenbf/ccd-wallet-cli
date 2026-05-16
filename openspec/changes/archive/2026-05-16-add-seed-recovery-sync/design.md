## Context

The wallet can already store encrypted seeds, issue identities, and create accounts, but it cannot yet reconstruct existing wallet state from a previously used seed. Concordium recovery is not a single server-side sync endpoint: identities must be discovered by querying identity providers with seed-derived recovery requests, and accounts must then be discovered by deriving credential registration IDs and asking the node whether those credentials exist on-chain.

The browser wallet provides a strong reference model for this flow. It fetches provider metadata from wallet-proxy `/v2/ip_info`, uses each provider's `recoveryStart` URL to submit a serialized `idRecoveryRequest`, and, for each recovered identity, derives credential IDs and looks up accounts by credential registration ID against the node. This change should reuse that architecture while fitting the CLI's seed/network-scoped storage model and existing cliclack-based UX.

A further UX constraint is that recovery can be long-running while the total number of identities and accounts is unknown up front. We therefore need progress feedback that is informative without pretending to know a final item count. Because provider recovery and account discovery are good candidates for bounded parallelization, the progress model must also remain comprehensible when multiple providers and identities are in flight at once.

## Goals / Non-Goals

**Goals:**
- Add `seed sync` as the primary recovery command for a stored seed on a selected network.
- Add `seed add --restore <NETWORK>` as a convenience flow that stores a seed, then immediately runs recovery.
- Recover identities using wallet-proxy provider metadata and provider `recoveryStart` endpoints.
- Recover accounts by deriving credential registration IDs from the seed and querying the node by credential ID.
- Import recovered identities/accounts idempotently into local storage.
- Provide interactive provider selection and strong recovery progress visibility.
- Support actionable non-interactive usage by requiring explicit scope where prompts would otherwise be needed.

**Non-Goals:**
- Designing an unbounded or user-tunable gap-search strategy in this first version.
- Reconstructing arbitrary historical wallet metadata from remote systems beyond identities and wallet-managed accounts.
- Recovering accounts for seeds or networks other than the explicitly selected recovery scope.
- Introducing JSON output for the initial recovery UX.

## Decisions

### 1. Use wallet-proxy `/v2/ip_info` as the source of recovery URLs
The CLI will fetch identity providers from wallet-proxy and require provider metadata to carry `recoveryStart`, matching the browser wallet model.

**Why:** This avoids trying to derive recovery URLs from `ip_info` or provider description URLs, keeps parity with existing Concordium wallet behavior, and centralizes provider configuration in wallet-proxy.

**Alternative considered:** Construct recovery URLs from `ipDescription.url` or other `IpInfo` fields. Rejected because the browser wallet does not do this and some providers already rely on explicit `recoveryStart` metadata.

### 2. Treat recovery as a two-phase pipeline: identities first, then accounts
For each selected provider, the CLI will:
1. probe identity indexes starting at 0;
2. when a recovery response succeeds, import/update the identity locally;
3. derive credential IDs for that identity and query the node for matching accounts;
4. import any newly found accounts.

The phases are logically ordered, but execution may overlap: providers may run in parallel, and account discovery for a recovered identity may begin while other providers are still probing identities.

**Why:** This matches the protocol model and browser wallet behavior. Accounts cannot be discovered independently of identities because credential derivation depends on the recovered identity tuple.

**Alternative considered:** Separate `seed sync-identities` and `seed sync-accounts` commands. Rejected for v1 because users think in terms of restoring wallet state, not individual protocol phases.

### 3. Keep the initial scan strategy simple and bounded by implementation defaults
The initial version will start scanning from identity index 0 for each selected provider and from the next locally unused credential counter for each recovered identity. The stopping policy will be implementation-defined and conservative, but not surfaced as user-configurable gap controls yet.

**Why:** The user explicitly wants to defer gap design. Deferring CLI flags and spec complexity keeps the first recovery feature understandable while preserving room for later tuning.

**Alternative considered:** Add `--gap`, `--max-identity-index`, and related knobs now. Rejected to avoid prematurely locking in search semantics.

### 4. Make recovery imports idempotent and tuple-driven
Recovered identities and accounts will be matched on their existing derivation tuples:
- identity: `(network_genesis_hash, seed_id, ip_identity, identity_index)`
- account: `(network_genesis_hash, seed_id, ip_identity, identity_index, credential_counter)`

If a tuple already exists, recovery updates or reuses the local row instead of creating a duplicate. Newly discovered rows get generated labels.

**Why:** Tuple identity is stable across installations and user-facing labels are not. This preserves rename semantics and allows repeated sync runs.

**Alternative considered:** Match primarily by label. Rejected because labels are wallet-local and may differ or be absent.

### 5. Use generated labels for newly imported rows and preserve existing labels when present
Recovered entities that do not already exist locally will receive deterministic fallback labels such as `Identity <n>` / `Account <n>` based on local naming helpers or equivalent generated-label logic. Existing rows keep their current labels.

**Why:** Recovery should not stop for interactive naming prompts. Users can rename later using the existing rename commands.

**Alternative considered:** Prompt for every recovered entity label. Rejected because it makes long-running recovery tedious and brittle.

### 6. Support both interactive multiselect and explicit `--provider` filters
`seed sync` will support repeated `--provider` arguments for explicit provider selection. `--provider all` will mean “all recovery-capable providers in the resolved network scope”. Repeated `--provider <ID>` arguments will mirror the interactive multiselect result shape in non-interactive or scriptable usage. In interactive mode, when no explicit provider filters are supplied and more than one provider is available, the CLI will present a cliclack multiselect over available providers, defaulting to all providers selected. If there is exactly one provider, the selector is skipped.

`all` is mutually exclusive with specific provider ids.

**Why:** This keeps the interactive and non-interactive experiences aligned and gives automation an explicit way to express either “all providers” or “this subset of providers”.

**Alternative considered:** Always scan all providers with no prompt or explicit filter support. Rejected because user-driven narrowing is useful and already requested.

### 7. Run recovery with bounded concurrency and present progress as a cliclack status dashboard
Recovery will use bounded concurrency in two places:
- selected providers may be scanned in parallel;
- account discovery for recovered identities may also run in parallel.

Concurrency limits will be conservative and implementation-defined in v1 so the CLI improves wall-clock performance without overloading wallet proxy, identity providers, or the node.

Interactive progress will stay within the cliclack UI model. Instead of rendering separate progress widgets per worker, the CLI will maintain a compact live status dashboard composed of cliclack progress/log/status primitives:
- a determinate provider-level progress bar, because the number of selected providers is known;
- provider state counts, e.g. queued / running / complete / failed / skipped;
- aggregate discovery counters, e.g. identities recovered, accounts recovered, probes attempted;
- a compact "active work" snapshot listing a small number of currently running provider or account-scan tasks.

A representative interactive display is:

```text
Restoring seed 'main_seed' on 'testnet'
Providers complete: 2/5 [████████░░░░░░░]
Providers: 1 queued • 2 running • 2 complete • 0 failed
Recovered: 4 identities • 9 accounts
Skipped: 1 provider
Active:
- ScanCorp (id 2): probing identity 4
- ID North (id 7): probing identity 1
- ScanCorp (id 2) / identity 3: probing credential 2
```

**Why:** Provider count is known, so provider completion can use a truthful progress bar. Identity/account totals are unknown, especially under parallel discovery, so they are better represented as live counters and active-work snapshots. Keeping the presentation within cliclack preserves a consistent UX across the app.

**Alternative considered:** A single global percentage bar over identities/accounts. Rejected because the denominator is unknown and would be misleading. Also considered rendering one progress widget per concurrent worker; rejected because it would be visually noisy and inconsistent with the rest of the CLI.

### 8. Distinguish interactive and non-interactive recovery requirements clearly
Recovery will resolve seed and network from explicit arguments first, then from active defaults when allowed. Explicit `--provider` filters, when supplied, suppress the interactive provider multiselect. Interactive mode may prompt for any still-missing seed/network/provider selections. Non-interactive mode may use already-resolved active defaults, but must not prompt and must error if required scope remains unresolved.

**Why:** This matches existing CLI behavior for context-bearing commands and avoids hanging in automation while still making provider scope scriptable.

**Alternative considered:** Require explicit seed and network flags in all non-interactive recovery runs. Rejected because the CLI already has an active-context model that is useful in scripts when defaults are intentionally configured.

## Risks / Trade-offs

- **[Recovery metadata missing for some providers]** → Skip providers lacking `recoveryStart`, report them in the final summary, and keep the dependency on wallet-proxy metadata explicit.
- **[Long-running parallel scans with uncertain completion]** → Use a cliclack-based aggregate dashboard with truthful provider progress, worker-state counts, and incremental counters instead of misleading percentages.
- **[Too much parallelism could overwhelm external services]** → Use bounded concurrency with conservative defaults and centralized task scheduling.
- **[Repeated recovery runs may revisit already-known tuples]** → Make store import/update helpers idempotent and tuple-driven.
- **[Recovery may find identities but no accounts]** → Treat this as a valid outcome and summarize it clearly rather than as a hard failure.
- **[Node or provider partial outages]** → Continue provider/account discovery where safe, collect per-provider errors, and print a partial-success summary.
- **[Generated labels may be bland]** → Keep labels deterministic and rename-friendly; users can refine them after recovery.

## Migration Plan

- No DB schema migration is required if current storage already supports inserting completed identities/accounts by tuple.
- Add recovery-oriented store helpers and command orchestration behind new CLI subcommands.
- Rollback is low risk: removing the new commands leaves existing stored rows intact.
- If recovery import behavior proves problematic, the commands can be disabled without invalidating already imported identities/accounts.

## Open Questions

- Whether the Concordium Rust SDK already exposes all recovery request and credential-id helpers needed directly in the versions used by this repo, or whether small wrapper additions are needed in `ccd-wallet-core`.
- Whether explicit provider filters should accept only provider ids, or also stable provider labels/display names, in a future change.
- How aggressive the default stopping policy should be before user-tunable search controls are added in a later change.
- Whether recovered-account labels should incorporate identity/provider context by default to reduce immediate rename pressure.
- What the initial provider and account-discovery concurrency limits should be for a good balance of speed, reliability, and output stability.
