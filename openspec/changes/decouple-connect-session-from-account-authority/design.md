## Context

The current connect protocol presents a session-first API to browser applications: the browser pairs, receives a session token, and can later call `requestAccount`. Internally, however, the wallet approval flow selects both network and account during pairing and stores both in the active session. That makes `requestAccount` effectively a read-back of already approved context rather than a true authority-acquisition step.

This mismatch is manageable for the current narrow example flow, but it becomes problematic as the API expands. The example application is evolving from a single pairing/account demo into a sectioned showcase for multiple capability areas, starting with Smart Contracts and later including Transactions and Chain Updates. Some of these areas require an account, while others may not. The protocol therefore needs a session model that can exist without account authority while still preserving an explicit, trusted network context.

The change spans the Rust connect server, the wallet CLI approval flow, the TypeScript client package, and the example web app. It also changes security-relevant semantics around what pairing approves and what later requests are allowed to do, so the design should make those boundaries explicit before implementation.

## Goals / Non-Goals

**Goals:**
- Separate browser-session trust from account authority.
- Keep pairing as the approval point for browser trust and network selection.
- Make `requestAccount` the explicit step that grants account authority to an already paired session.
- Preserve session-bound network safety so later account-oriented requests cannot silently switch networks.
- Update smart contract execution flows to require previously granted account authority instead of assuming pairing already selected an account.
- Restructure the example app into a paired shell with feature navigation and feature-specific authority prompts.
- Make the Smart Contracts showcase derive schema automatically from embedded module metadata instead of requiring manual schema entry.

**Non-Goals:**
- Supporting multiple simultaneous browser sessions.
- Supporting multiple networks in a single active session.
- Adding governance-key or other future authority types in this change.
- Changing the public method names of `pair` or `requestAccount`.
- Designing the full Transactions or Chain Updates feature pages beyond placeholder navigation and authority expectations.

## Decisions

### 1. Pairing binds network, but not account

**Decision:** A successful `pair` approval SHALL establish a trusted browser session and bind exactly one approved network to that session, but SHALL NOT require account selection.

**Rationale:** Pairing is the right place to approve origin trust and stable session context. Network selection still belongs here because most wallet-backed actions are network-specific and because keeping the session network-authoritative preserves the current safety property against browser-specified network switching. Account selection, however, is not universal across current and future API areas and should therefore be deferred until needed.

**Alternative considered:** Make pairing completely context-free and defer both network and account. Rejected because it pushes network selection into every feature flow, weakens the usefulness of the paired session shell, and makes later account-oriented requests more repetitive.

### 2. Session state stores granted authorities separately from core session context

**Decision:** The active session model in `ccd-wallet-connect` SHALL be split into:
- core session context: session token, origin, and network genesis hash
- granted authority state: optional approved account authority for the session's network

The first iteration only needs zero-or-one account authority for the session network.

**Rationale:** This reflects the conceptual boundary between “this browser is trusted to talk to the wallet in the current session” and “this browser may use this specific account authority.” It also leaves a clear path for future authority types without requiring another semantic rewrite.

**Alternative considered:** Keep a flat session model and use an empty string or placeholder for “no account yet.” Rejected because it obscures the security model and makes missing-authority behavior easy to mishandle.

### 3. `requestAccount` becomes a true authority-acquisition flow

**Decision:** `requestAccount(sessionToken, networkGenesisHash)` SHALL require an active paired session, verify that the requested network matches the session-bound network, then prompt the wallet user to select and approve an account for that session. On approval, the wallet returns the approved account address and stores it as the session's current account authority.

Subsequent `requestAccount` calls for the same paired session and network SHALL return the already granted account authority unless the session has been reset. Changing the approved account for an active session is deferred to a later change.

**Rationale:** The browser API already exposes `requestAccount` as a separate step, so changing its semantics avoids a breaking method rename while bringing the behavior in line with developer expectations. Caching the granted authority within the session avoids repeated prompts for every account-backed feature call.

**Alternative considered:** Require fresh account approval on every `requestAccount` call. Rejected for now because it would make the example app and feature pages unnecessarily noisy while not improving trust boundaries relative to session-scoped authority.

### 4. Account-requiring feature methods depend on prior session authority

**Decision:** `requestContractInit` and `requestContractUpdate` SHALL continue to use the session-bound network, but they SHALL require that the active session already has approved account authority. If no account authority has been granted yet, the wallet rejects the request with a dedicated actionable error (`-32006`) instructing the caller to acquire account authority first.

**Rationale:** This keeps signing authority controlled entirely by the wallet and avoids letting the browser name an account as the source of truth. It also keeps feature flows explicit in the example app: the Smart Contracts page can detect missing authority and guide the user to request it.

**Alternative considered:** Have contract requests trigger account selection inline when missing. Rejected because it hides an important protocol step and makes the example app less useful as a reference for capability-specific authority management.

### 5. The example app becomes a session-gated API showcase shell

**Decision:** The example app SHALL have two top-level states:
- unpaired: a pairing screen that collects server URL, network genesis hash, and challenge
- paired: an application shell that shows global session context and a navigation bar

The paired shell SHALL include a Smart Contracts section and placeholder sections for Transactions and Chain Updates. Smart Contracts SHALL surface whether account authority is missing and offer an explicit account-request action before contract forms become available. The paired shell SHALL NOT automatically acquire account authority when entering an account-backed section; the user must trigger `requestAccount` explicitly.

**Rationale:** This structure matches the future intent of the app as an API showcase rather than a single linear demo. It also demonstrates the new session/authority split clearly to integrators.

**Alternative considered:** Keep a one-page form and bolt on more panels. Rejected because it does not scale well to future API areas and would blur global session concerns with feature-specific authority needs.

### 6. TypeScript client API shape stays stable, but semantics and docs change

**Decision:** The TypeScript client SHALL keep the current method names and parameters for `pair` and `requestAccount`, but its documentation and examples SHALL reflect the new semantics:
- `pair` establishes a session token for a network-approved session
- `requestAccount` acquires account authority for that session and returns the approved account address

**Rationale:** The existing client surface already supports the intended staged flow, so a semantic change is sufficient and avoids needless churn for example consumers.

**Alternative considered:** Rename `requestAccount` to something like `grantAccountAuthority`. Rejected because the current name remains understandable and the ecosystem is still small enough that documentation can carry the semantic clarification.

### 7. The example app uses `@concordium/web-sdk` with embedded schemas derived from the chain or referenced module

**Decision:** The Smart Contracts section in the example app SHALL depend on `@concordium/web-sdk` for browser-side smart contract schema handling and typed parameter workflows, but it SHALL support only contracts whose schema is embedded in the deployed module. The example app SHALL NOT ask the user to paste a base64-encoded schema manually.

For contract init flows, the example app SHALL derive the embedded schema from the module referenced by the supplied `moduleRef`.

For contract update flows, the example app SHALL query the target contract instance, derive the instance's `sourceModule`, and fetch the embedded schema from that module before preparing parameter bytes.

To support those lookups, the example app SHALL maintain browser-reachable node access as part of its paired application context for Smart Contracts workflows. `@ccd-wallet/connect-client` itself SHALL remain focused on transport/protocol concerns and SHALL NOT take on a `web-sdk` dependency.

**Rationale:** Embedded-schema-only support produces a cleaner showcase than manual schema entry, reduces user error, and better demonstrates the intended browser-side toolchain for realistic contract integrations. Deriving the schema from the chain or referenced module ensures the prepared parameter bytes correspond to the actual contract module in use.

**Alternative considered:** Continue to accept pasted base64 schemas in the example app. Rejected because it introduces avoidable friction, makes schema/module mismatches easier, and weakens the example's value as a realistic reference integration.

## Risks / Trade-offs

- **Session semantics change may surprise existing expectations** → Update specs, READMEs, and example flows together so the staged authority model is documented everywhere a contributor is likely to look.
- **Cached account authority may hide when approval actually occurs** → Make the example app and status messaging explicit about account authority state and when it was granted.
- **Missing-authority contract errors may feel like extra friction** → Present the prerequisite clearly in the Smart Contracts page rather than letting users discover it only through an RPC failure.
- **Future authority types could require a richer session model than zero-or-one account authority** → Model session authority as a separate concept now so future expansion is additive rather than another rewrite.
- **Adding `@concordium/web-sdk` and node access increases example-app complexity and bundle surface** → Keep the dependency confined to the example app, use it only for Smart Contracts concerns, and avoid leaking it into the connect client package.
- **Embedded-schema-only support excludes modules without embedded schema** → Surface a clear error that the showcase only supports contracts whose module exposes embedded schema.

## Migration Plan

1. Update protocol/session specs to define the new pairing and account-authority semantics.
2. Change the connect server and wallet pairing flow so pairing stores only network context and empty authority state.
3. Rework `requestAccount` to perform account selection/approval and store approved account authority into the session.
4. Update contract execution handling to require granted account authority and return actionable missing-authority errors otherwise.
5. Update the TypeScript client documentation and the example app shell/navigation to teach the new flow, including `@concordium/web-sdk`-based Smart Contracts workflows that derive embedded schema from the chain or referenced module.
6. Validate the new flow end-to-end with example-app tests and connect-client / Rust integration tests.

## Resolved Follow-ups

- Repeated `requestAccount` calls for the same active session return the already granted account authority. Changing accounts within a session is deferred to a later change.
- The example app keeps account acquisition explicit. Entering an account-backed section does not auto-request authority.
- Missing account authority on account-backed methods uses the next dedicated JSON-RPC error code, `-32006`, with an actionable message telling the caller to invoke `requestAccount` first.
- The first Smart Contracts page exposes schema-driven JSON value input that is translated through embedded module schema into serialized parameters.
- The example app supports only contracts with embedded schema and derives that schema automatically from the chain or supplied module reference rather than requiring pasted schema input.
