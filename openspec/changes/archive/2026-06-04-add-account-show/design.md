## Context

The wallet CLI already has account commands for local account lifecycle operations (`new`, `list`, `rename`, `export`, and `import genesis`) and several `show` commands for on-chain inspection of other resources. Account state inspection is currently missing: users cannot ask the CLI to show the CCD balance, release schedule, protocol-level token balances, or lock-derived availability for an account.

The account store encrypts finalized account addresses for local derived and imported accounts. A local-label `account show` therefore needs to resolve the selected network first, find the local record in that network, and unlock the relevant seed or imported account vault to obtain the address before querying the node. Raw address targets do not need local unlocking.

## Goals / Non-Goals

**Goals:**

- Add `ccd-wallet account show <ACCOUNT>` for on-chain account state inspection.
- Support both raw Concordium account addresses and local account labels as targets from the first implementation.
- Keep default output balance-oriented: CCD balance, available balance, locked balance, release schedule, and token balances with available/locked information.
- Annotate local account targets with minimal wallet context in the header only.
- Move protocol details such as nonce and account index behind `--verbose`.
- Provide stable JSON output for automation.

**Non-Goals:**

- Exporting signing material or revealing private keys.
- Replacing `account export`, `transaction show`, `token show`, or future staking-specific commands.
- Adding a persisted active account concept.
- Implementing historical balance indexing beyond querying the selected node block.

## Decisions

### Target parsing uses a single positional `<ACCOUNT>`

`account show` will accept one positional target. The CLI first attempts to parse it as a Concordium account address. If parsing succeeds, the command queries that address directly. Otherwise, the target is treated as a local account label and resolved within the selected network.

Alternative considered: use `--address <ADDRESS>` for raw address queries. A single positional target is simpler for users and matches the command's account-inspection purpose.

### Network context is resolved before local-label lookup

For local labels, the command resolves the network/node context first, then searches for a local account with that label on the resolved network genesis hash. This keeps the command oriented around querying a specific network and avoids inferring network from local account storage.

If a raw `--node` endpoint is used without a registered network label, the command should still query raw address targets. Local label targets require enough network identity to match stored accounts; when the node endpoint is not associated with a configured genesis hash, the command should query the node genesis hash if needed before resolving the label.

### Default rendering focuses on balances and locks

The human default view will render:

- Header: `[<seed-label> : <local-label>] <address> @ <network-label-or-endpoint>` for derived local accounts. Raw address targets omit the bracketed local prefix. Imported local accounts have no seed and use `[<local-label>] <address> @ <network-label-or-endpoint>`.
- CCD balance and available balance.
- CCD locked balance when non-zero, calculated from total balance minus available balance.
- CCD release schedule entries when present.
- Protocol-level token balances, with available and locked amounts when account module state exposes availability.

Nonce, account index, credential count, signature threshold, encrypted balance internals, and staking summary belong under `--verbose` because they are useful for debugging or protocol inspection but not part of the primary balance view.

### Token availability uses decoded account module state

For each token in `AccountInfo.tokens`, the renderer will decode the token account module state and use `available` when provided. If no available value is present, availability is treated as equal to total balance, matching existing token transfer helper behavior. Locked token balance is total minus available and is shown when non-zero.

### Pending local accounts do not query account info

A pending local account may not have a finalized address. When a local target resolves to a pending record, the command renders pending status and the submission transaction hash, if present, instead of trying to query account info.

### JSON uses a wallet-owned schema

`--json` should not print raw SDK debug output. It should emit a stable schema containing the resolved address, network label or endpoint, optional local metadata, CCD balances and releases, token balances, and verbose protocol fields when requested or as a dedicated protocol object if useful.

## Risks / Trade-offs

- **Local label resolution can require unlocking secrets to obtain the address** → Prompt only after target and network resolution succeed; raw address targets never prompt for wallet secrets.
- **`locked` CCD is broader than release schedules** → Render locked as `total - available` and render release schedule separately so users can see which portion has explicit release entries.
- **Token module state decoding may fail for some token state** → Treat decode failure as an actionable query/rendering error rather than silently showing misleading availability.
- **Network labels may be unavailable for raw node endpoints** → Use the configured network label when available, otherwise use the endpoint label in the header and JSON context.
- **The default output may grow as account info evolves** → Keep default limited to balances, locks, and release schedules; add new protocol fields under `--verbose` unless clearly user-facing.
