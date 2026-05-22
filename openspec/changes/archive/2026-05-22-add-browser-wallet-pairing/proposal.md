## Why

`ccd-wallet` is starting to grow beyond a pure command-by-command CLI. A browser-based dApp or companion web UI needs a safe way to discover wallet context before it can build a transaction proposal, especially the selected network and account address. Today there is no browser-facing pairing flow, no temporary connected-wallet session, and no way for the wallet user to approve which origin may read a chosen account/network context.

A narrowly scoped first step is to add browser pairing for normal account-based sessions only. This keeps the initial API small and security-reviewable while still unlocking the core UX needed by dApps: connect to the wallet, obtain the chosen network/account context, and prepare later transaction proposals against that context.

Governance-key pairing is intentionally a separate use case. It has different authority, UI, and approval requirements and should be designed as its own follow-up change.

## What Changes

- Add a `ccd-wallet connect` command that starts a temporary local browser-connectable session.
- Introduce a browser pairing flow with explicit user approval, origin visibility, and an application-provided pairing challenge that the wallet user verifies during approval.
- Require the wallet user to choose the session network and account during pairing for account-oriented sessions.
- Expose the approved session context to the paired browser over a single WebSocket channel using JSON-RPC 2.0 semantics so it can read the selected network genesis hash and account address before constructing later transaction proposals.
- Limit the initial connected-wallet API scope to pairing and session-context retrieval only; transaction proposal and signing methods stay out of scope for this change.
- Defer governance-key pairing to a later dedicated change.

## Capabilities

### New Capabilities
- `browser-wallet-pairing`: Pair a browser dApp with `ccd-wallet`, establish a temporary approved session, and expose the selected network/account context for that session.

## Impact

- Affected code: CLI surface in `crates/ccd-wallet/src/cli.rs`, new connect/session command handling under `crates/ccd-wallet/src/commands/`, and a dedicated `crates/ccd-wallet-connect` crate for the WebSocket JSON-RPC pairing/session API.
- Affected systems: localhost WebSocket transport, browser session handshake, origin validation, application-provided pairing challenge verification, interactive account/network selection, temporary session state, and browser-readable session context.
- User-facing behavior: a new `ccd-wallet connect` mode, terminal-driven pairing approval, a single paired browser session at a time, rejection of new pairing requests while a session is active, explicit network/account selection during account-oriented pairing, and a minimal browser API for retrieving only the paired network genesis hash and account address.
