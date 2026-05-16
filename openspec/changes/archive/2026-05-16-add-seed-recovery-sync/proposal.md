## Why

Users who re-add an existing seed currently have to manually recreate or wait for identities and accounts to appear through other wallet flows. The wallet needs a recovery-oriented seed workflow that can discover existing Concordium identities and accounts from a stored seed on a chosen network.

## What Changes

- Add `seed sync` to recover identities and accounts for a stored seed on a selected network.
- Add `seed add --restore <NETWORK>` as a convenience flow that stores a seed and immediately runs seed recovery for the chosen network.
- Recover identities by querying wallet-proxy identity provider metadata, building Concordium identity recovery requests, and calling each provider's `recoveryStart` endpoint.
- Discover accounts for recovered identities by deriving credential registration IDs from the seed and querying the node for matching on-chain accounts.
- Run provider recovery and account discovery with bounded concurrency to reduce restore time without overwhelming external services.
- Import newly found identities and accounts into local storage without duplicating existing rows.
- Add interactive provider selection plus explicit `--provider` filtering, including `--provider all` and repeated `--provider <ID>` arguments, together with cliclack-based recovery progress feedback suitable for long-running parallel scans with unknown totals.
- Surface clear recovery summaries, partial-failure reporting, and actionable non-interactive requirements.

## Capabilities

### New Capabilities
- `seed-recovery-sync`: Recover wallet state for a stored seed by discovering identities via identity providers and accounts via credential-id lookups.

### Modified Capabilities
- `seed-command`: Add `seed sync` and `seed add --restore <NETWORK>` command behavior.
- `interactive-cli-prompts`: Add prompt behavior for recovery network/provider selection and long-running recovery progress presentation.
- `identity-provider-client`: Add identity recovery request support using provider `recoveryStart` URLs.
- `identity-storage`: Allow importing recovered completed identities without duplicating existing derivation tuples.
- `account-storage`: Allow importing recovered confirmed accounts without duplicating existing derivation tuples.

## Impact

- Affected code: `crates/ccd-wallet/src/commands/seed.rs`, command UI helpers, recovery orchestration in `ccd-wallet-core`, store query/import helpers, identity-provider client crate, and bounded-concurrency progress aggregation.
- External systems: wallet proxy `/v2/ip_info`, identity provider recovery endpoints, Concordium node account lookup by credential registration ID.
- Dependencies: likely reuse existing Concordium SDK primitives; may need small client additions for recovery calls and progress UI support.
