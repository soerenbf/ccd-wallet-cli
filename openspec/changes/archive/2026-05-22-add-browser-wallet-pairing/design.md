## Context

The current wallet is a CLI with strong local-secret handling and interactive approval patterns, but no browser-facing API surface. The intended direction is not an always-on daemon. Instead, the wallet should remain an explicit CLI and only become connectable when the user intentionally starts a dedicated session.

The first connected-wallet slice should be small:

1. start a temporary localhost session with `ccd-wallet connect`
2. let a browser dApp request pairing
3. show the calling origin and complete a richer pairing ceremony
4. choose the network and account during pairing
5. make the approved network/account context available to the browser for subsequent dApp preparation work

This change stops there. It does not yet cover transaction proposal submission, signing, governance flows, background daemon lifecycle, or broad wallet discovery.

## Goals / Non-Goals

**Goals:**
- Add a dedicated `ccd-wallet connect` mode for temporary browser connectivity.
- Support a safe browser pairing ceremony with explicit user approval.
- Bind an approved browser session to one selected network and one selected account for the duration of the session.
- Make the selected session context readable by the paired browser after approval.
- Keep the first browser API intentionally narrow and easy to reason about.

**Non-Goals:**
- Adding transaction proposal, signing, or submission methods.
- Supporting governance-key pairing in the same flow.
- Supporting per-request account or network overrides after pairing.
- Creating a persistent trusted-site registry in this change.
- Building a full background wallet daemon.

## Decisions

### 1. `ccd-wallet connect` is an explicit temporary session mode
The wallet will remain CLI-first. Browser connectivity exists only while the user runs a dedicated command such as `ccd-wallet connect`.

The command hosts a localhost server, manages one pairing lifecycle, surfaces browser requests in the terminal, and exits cleanly when the user stops the session.

**Rationale:** This preserves the current product identity and keeps wallet exposure deliberate rather than ambient.

### 1a. The connect protocol lives in a dedicated crate
The WebSocket JSON-RPC pairing/session API will live in a dedicated `ccd-wallet-connect` crate. That crate owns transport concerns such as WebSocket session handling, JSON-RPC message types, and protocol dispatch. The CLI crate depends on it to host the connect server, while `ccd-wallet-core` remains responsible for wallet storage and domain integration logic.

The intended dependency direction is:
- `ccd-wallet-core`: storage, crypto, wallet/domain logic
- `ccd-wallet-connect` → depends on `ccd-wallet-core`
- `ccd-wallet` → depends on `ccd-wallet-core` and `ccd-wallet-connect`

**Rationale:** This keeps transport/protocol concerns out of the CLI crate, keeps `ccd-wallet-core` focused on wallet logic rather than wire protocol details, and gives the browser API a clear home as it evolves.

### 2. Pairing is richer than raw localhost access
The browser pairing flow must not treat any localhost caller as implicitly trusted. The wallet will validate the browser origin, present that origin to the user, and require an interactive pairing approval ceremony.

The ceremony uses an application-provided user-visible challenge or pairing code included in the pairing request. The wallet validates the challenge format, displays that same challenge in the terminal together with the browser origin, and requires the user to confirm that it matches what is shown in the browser before approval completes.

**Rationale:** Browser-to-localhost flows need layered trust boundaries. Origin plus a visible pairing ceremony is safer and more legible than a silent local API, and an application-provided challenge gives the user a concrete browser-to-wallet comparison point.

### 3. Pairing chooses the session network and account
For the first account-oriented connected-wallet flow, the wallet user chooses the session network and account as part of approving the pairing request.

The chosen network/account become session state and are the only browser-visible authority context for the lifetime of that session.

**Rationale:** dApps need account and network context early so they can inspect balances, addresses, or other account-linked state before building transaction proposals. Choosing them during pairing keeps later request semantics simple.

### 4. Session context is readable after pairing, but the API surface remains narrow
Once a session is paired, the browser may retrieve the approved session context, including at minimum:
- selected network identity as genesis hash
- selected account address

The first transport for this change is a single WebSocket channel using JSON-RPC 2.0 message semantics. Pairing, approval-state communication, and session-context retrieval all occur over that same channel.

The browser API for this change should stop at session/context methods. It should not yet expose transaction proposal entry points.

**Rationale:** This unlocks the dApp preparation workflow without forcing broader API decisions before the trust model is settled, keeps the first browser-visible context minimal, and a single message-oriented transport fits the interactive session model better than splitting the first API across HTTP and WebSocket.

### 5. One account-oriented pairing flow now; governance pairing later
This change covers account-based pairing only. Governance-key sessions are intentionally excluded and will be specified through a separate endpoint and follow-up change.

**Rationale:** governance authority differs materially from account authority in both signer selection and approval UX. Treating them as one API now would blur an important boundary.

### 6. Session context is stable for the life of the session
Once pairing finishes, the browser session uses the selected network/account as its fixed context. If the dApp needs a different account or network, the user should pair again rather than mutating the session in place.

This change supports only one active paired browser session at a time. While a session is active, new pairing requests are rejected instead of queued or allowed to create additional sessions.

**Rationale:** fixed session context avoids ambiguity, reduces API complexity, and aligns with how many wallet integrations treat account and network as connected-session state rather than per-request parameters. Limiting the first cut to a single paired browser keeps approval and session ownership semantics simple.

## Risks / Trade-offs

- **[Pairing UX may feel heavy for local development]** → Mitigation: keep the handshake explicit but compact, and allow future ergonomics improvements once the security shape is proven.
- **[Single-context sessions may require reconnecting when a dApp needs another account or network]** → Mitigation: accept the constraint in v1 to keep semantics simple and safe.
- **[Browsers vary in how reliably they surface origin and localhost connection details]** → Mitigation: make the wallet terminal the source of truth for approval, including origin and pairing code.
- **[A minimal context-only API does not yet let dApps submit proposals]** → Mitigation: this change is intentionally foundational; proposal APIs can build on the established session model in a follow-up change.

## Migration Plan

1. Add the `ccd-wallet-connect` crate and define its dependency on `ccd-wallet-core`.
2. Add CLI surface for `ccd-wallet connect`.
3. Introduce temporary localhost session hosting and browser handshake primitives.
4. Implement origin-aware pairing approval with browser-visible pairing code/challenge.
5. Reuse existing network/account selection patterns to bind approved session context.
6. Add a minimal browser-readable session-context API.
7. Add tests and documentation for connect lifecycle, pairing approval, and session-context retrieval.

## Open Questions

None currently.
