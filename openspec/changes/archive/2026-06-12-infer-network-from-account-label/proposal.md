## Why

Interactive account-consuming commands currently resolve the network before resolving an explicit local account label. This can force users through a network selector even when the supplied account label exists on exactly one configured network and therefore has no real ambiguity. For example, `ccd-wallet stake show account-2` may ask the user to select `local/p11-locks` even though `account-2` only exists there.

At the same time, the network remains a valuable filter when the user supplies it explicitly, and the active network remains useful as a soft default for filtering account pickers or choosing between otherwise ambiguous accounts. The desired behavior is not “always resolve accounts first”; it is account-assisted network resolution when no explicit network constraint exists.

## What Changes

- Add shared account-assisted network resolution for interactive account-consuming commands.
- Treat explicit network inputs, such as `--network` and compatible node overrides, as hard constraints for account lookup.
- Treat the active network as a soft default: use it to filter account selectors when no account was supplied and to prefer the active-network match when an explicit account label is ambiguous.
- If an explicit local account label does not match the active network but uniquely matches another configured network, infer that account's network in interactive mode instead of failing or prompting for network selection.
- Distinguish account references (raw address or local label) from signing accounts/senders (local account label only, because keys are required).
- Name sender options explicitly where applicable, including `--sender` and existing sender aliases such as `--account`.
- When an explicit local account label remains ambiguous after applying the active-network preference, disambiguate with an account selector that shows network and ownership/key-source metadata rather than a network selector.
- Preserve deterministic non-interactive behavior: non-interactive commands must not infer a network from account-label uniqueness or let account labels override the active network.
- Treat a single configured network as an obvious interactive selection: do not prompt, but still show the resolved network in the context header.
- Align account selectors and context headers so resolved network, account, and key-source/imported-source information is visible when selected or inferred silently.

## Capabilities

### Modified Capabilities
- `account-reference-resolution`: Define account-assisted network inference, active-network preference, and ambiguity behavior for interactive account-consuming commands.
- `account-signing-source`: Define local-account-only sender resolution for `--sender` and sender aliases.
- `active-network-selection`: Clarify active-network soft-default behavior, single-network interactive fallback, and non-interactive determinism.
- `interactive-cli-prompts`: Require context headers for silently selected single choices and ownership-decorated account disambiguation.
- `stake-command-execution`: Apply account-assisted network resolution to `stake show` and stake mutation account selection.

## Impact

- Shared Rust CLI account/network resolution helpers in `crates/ccd-wallet/src/commands/account.rs` and related command modules.
- Stake inspection and mutation context resolution in `crates/ccd-wallet/src/commands/stake/`.
- Contract and token command flows that accept local account labels, explicit `--sender`/sender-account options, or select local signing accounts, where they use the same account-selection pattern.
- Context rendering helpers in `crates/ccd-wallet/src/commands/ui.rs`.
- Tests for explicit network constraints, active-network preference, interactive unique-label inference, ambiguous-label account selection, single-network context display, and non-interactive determinism.
- Command documentation in `docs/commands.md` if user-visible network/account resolution semantics are described there.
