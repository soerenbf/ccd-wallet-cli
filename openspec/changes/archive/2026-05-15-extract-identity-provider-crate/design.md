## Context

The workspace currently has two crates:

- `ccd-wallet-core`: storage, wallet derivation, configuration helpers, and identity provider issuance helpers
- `ccd-wallet`: CLI orchestration

Identity provider functionality now includes request construction, HTTP protocol helpers, callback parsing, manual callback mode, loopback callback sessions, and tests. Keeping all of that inside `ccd-wallet-core` makes core a mixed wallet/storage/protocol crate and makes future reuse harder.

## Goals / Non-Goals

**Goals:**
- Move identity provider issuance code into a dedicated workspace crate.
- Keep behavior and CLI UX unchanged.
- Keep tests close to the code they exercise.
- Keep crate APIs small and explicit.
- Preserve the existing workspace development flow.

**Non-Goals:**
- No behavioral change to identity issuance.
- No database migration.
- No crate publishing or semver stability commitment.
- No renaming of CLI commands.
- No new callback behavior beyond the existing implemented loopback/manual modes.

## Decisions

### New crate name

Use `crates/ccd-wallet-identity-provider` with package name `ccd-wallet-identity-provider`.

Rationale:
- Keeps the crate clearly tied to this wallet workspace.
- Avoids implying an official standalone Concordium SDK crate.
- Leaves room for future extraction/publishing if desired.

Alternative considered: `identity-provider` or `ccd-identity-provider`. These are shorter but less clearly scoped to this repository.

### Move, do not duplicate

The existing `ccd_wallet_core::identity_provider` module should be moved to the new crate rather than copied.

Rationale:
- Avoids two implementations drifting apart.
- Keeps tests and public API focused in one crate.

### Public API shape

The new crate should expose roughly the same top-level concepts currently used by the CLI:

- `build_request(...)`
- `callback::{CallbackSession, ManualPasteSession, LoopbackCallbackSession, MANUAL_REDIRECT_URI, parse_callback_url}`
- `client::{fetch_wallet_proxy_ip_info, build_issuance_url, start_issuance, poll_code_uri, PollResult, WalletProxyIpEntry}`

Rationale:
- Minimizes integration churn.
- Keeps the refactor mechanical and low-risk.

### Dependency direction

The new crate may depend on `ccd-wallet-core` for wallet derivation types, specifically `ConcordiumHdWallet`, if moving wallet derivation out is not part of this change.

The CLI should depend on both:

- `ccd-wallet-core` for storage/config/wallet functions
- `ccd-wallet-identity-provider` for issuance protocol/callback logic

Rationale:
- Keeps the change small.
- Avoids a larger wallet-derivation crate split.

### `ccd-wallet-core` export cleanup

`ccd-wallet-core` should stop exporting `pub mod identity_provider` once the move is complete.

Rationale:
- Makes the new crate the only owner of identity provider behavior.
- Prevents accidental continued use of the old path.

## Risks / Trade-offs

- **Risk:** New crate depends on `ccd-wallet-core`, so the separation is not fully independent.  
  **Mitigation:** This change is about ownership and workspace boundaries first; wallet derivation can be extracted later if needed.

- **Risk:** Public import paths change across the workspace.  
  **Mitigation:** Update all imports in one mechanical pass and rely on clippy/tests.

- **Risk:** Tests may rely on crate-private helpers after moving.  
  **Mitigation:** Move tests with the code and keep helpers private within the new crate module where possible.

- **Risk:** Workspace dependency duplication.  
  **Mitigation:** Use existing workspace dependencies and only add crate-specific dependencies required by moved code.
