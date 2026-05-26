## Context

The `ccd-wallet connect` command hosts a temporary WebSocket JSON-RPC server. Today it supports two methods: `pair` (challenge-based session establishment) and `requestAccount` (explicit account approval for a target network). The session state in `crates/ccd-wallet-connect` currently carries only a session token.

Browser dApps that have paired and obtained an account address still need to send transactions through their own signing stack. Wallet-mediated contract execution — where the CLI wallet reviews, signs, and submits — is the missing piece.

The implementation already has strong foundations:
- Source-aware account signing resolution (`resolve_signing_source` in `ccd-wallet-core`)
- SDK support for `send::init_contract`, `send::update_contract`, and `invoke_instance` dry-runs
- Interactive approval pattern established in `commands/connect.rs`
- Governance submit/wait pattern available as a reference for transaction submission and finalization display

## Goals / Non-Goals

**Goals:**
- Add `requestContractInit` and `requestContractUpdate` JSON-RPC methods to the connect server.
- Use the session-bound network and account as the authoritative execution context; no re-selection per transaction.
- Run a simulation when the caller requests validation (`validate: true`, default `false`); display results in the approval prompt and never block submission based on simulation outcome.
- Accept an optional contract schema alongside the parameter bytes; use it for readable parameter review when present.
- Return only the transaction hash from the RPC method; show finalization outcome locally in the wallet terminal.
- Expose typed `requestContractInit` and `requestContractUpdate` methods on the TypeScript connect client.

**Non-Goals:**
- Module deployment (static cost, different review concerns; deferring to a later change).
- Session context switching mid-session (account or network change while connected).
- Finalization polling or status methods through the browser protocol.
- Schema-aware serialization on the wallet side; the dApp is responsible for parameter bytes.

## Decisions

### 1. Session carries network and account from pairing

**Decision:** Extend `ActiveSession` in `crates/ccd-wallet-connect` to store `network_genesis_hash` and `account_address`. These are set during the `pair` approval, not on first `requestContractUpdate`.

**Rationale:** Contract execution must know both the sender account and the target network before building or signing the transaction. If resolution were deferred to each execution request, every method call would need account selection logic that duplicates pairing semantics. Session binding is already described as the intended model in the spec ("The approved session SHALL bind to exactly that selected network and account for the duration of the session") and aligns with what the existing pairing approval flow already does interactively.

**Implication:** `requestAccount` currently returns the selected account address but does not write it into session state. With this change, pairing itself binds network and account. `requestAccount` can continue to function as a way to read the session-bound address back to the browser, but contract execution does not depend on `requestAccount` having been called.

**Alternative considered:** Let the browser specify `networkGenesisHash` + `accountAddress` per execution request. Rejected because it would allow impersonating a different account than the one the user approved at pairing, bypassing the approval ceremony.

---

### 2. Parameter representation: bytes + optional schema

**Decision:** The JSON-RPC parameter fields accept the serialized parameter as a hex string (`parameterHex`) plus an optional `schema` object. The schema is used by the wallet for display only; the wallet does not re-serialize from schema.

**Rationale:** The dApp owns parameter construction and serialization. The schema is needed only for human-readable presentation in the approval prompt. Keeping serialization on the dApp side avoids the wallet needing to understand all schema formats and contract-specific types. This also matches the pattern used by existing wallet tooling (browser extension and node SDK).

Schema is optional because:
- Many contracts embed schema in the module and the dApp may not have it at call time.
- For contracts the operator trusts, hex-only review with just address/entrypoint/amount may be sufficient.
- The wallet should not block execution when schema is absent; it shows hex instead.

**Alternative considered:** Require schema always. Rejected because it blocks use cases where the dApp does not have the schema available.

---

### 3. Node endpoint resolved from session network genesis hash

**Decision:** When handling a contract execution request, the wallet resolves the node endpoint by finding the first registered network alias whose genesis hash matches the session's bound `network_genesis_hash`. If no matching network entry is found in the config, the request is rejected with an actionable error.

**Rationale:** The session is bound by genesis hash, not by alias. Multiple aliases can share one hash (e.g., two RPC endpoints for the same testnet). Picking the first alphabetically is deterministic and sufficient for a single-connection CLI wallet; the user controls which endpoint is registered for a given network.

**Alternative considered:** Let the browser supply a node endpoint. Rejected because it would let a dApp redirect signing to an arbitrary node, enabling potential chain replay attacks.

---

### 4. Optional simulation via `validate` flag

**Decision:** Both contract execution methods accept a `validate` boolean parameter (default `false`). When `validate: false` (the default), no simulation is performed and the approval prompt shows the request as-is. When `validate: true`, the wallet runs an `invoke_instance` dry-run before the approval prompt.

Critically, a failed simulation — including a rejected dry-run or a node connectivity error during simulation — SHALL never block the user from proceeding. Instead, the wallet shows the simulation result as a warning in the approval prompt and follows up with a y/N confirmation, giving the user the choice to submit anyway.

**Rationale:** The dApp is closer to the contract than the wallet is and will typically have already validated the transaction against local contract state before proposing it to the wallet. Mandatory simulation would add latency and create friction without adding safety in the common case. Making simulation opt-in lets dApps that want on-screen energy estimates or a sanity check enable it, while keeping the default path fast.

Never blocking on simulation failure respects the user as the final authority. A failing simulation may reflect stale dApp state, race conditions in contract state, or temporary node unavailability — none of which should prevent the user from exercising their own judgment.

The `invoke_instance` dry-run uses the same gRPC call used by `ContractClient::dry_run_update` in the SDK. For init, `ContractInitBuilder::dry_run_new_instance` can be used.

**Alternative considered:** Mandatory simulation that blocks on rejection. Rejected because it treats the wallet as a safety oracle rather than an approval surface, and creates a poor UX when the dApp has already performed its own validation.

---

### 5. Signer unlocked per transaction approval

**Decision:** The seed password (for derived accounts) or vault password (for imported accounts) is prompted at approval time for each contract execution request, using the same unlock flow as `requestAccount` in `commands/connect.rs`. The session does not store decrypted key material between requests.

**Rationale:** Keeping unlocked key material in memory for the duration of a session would be a significant security regression. The existing pairing flow already prompts per `requestAccount`; contract execution follows the same pattern.

---

### 6. Finalization displayed locally, not returned in RPC response

**Decision:** After submission, the wallet starts a background task (or inline wait) to track finalization and print the outcome to the terminal. The RPC response returns only `transactionHash` immediately after submission.

**Rationale:** Finalization takes several seconds. Blocking the JSON-RPC response until finalization would hold the browser WebSocket open and make the dApp unresponsive. The hash is sufficient for the dApp to track status independently through the node. Meanwhile, the wallet terminal is already the primary UX surface for this operator-oriented tool, so surfacing outcome there is natural.

---

### 7. New JSON-RPC error codes for contract execution

The connect server will introduce specific error codes:
- `-32004`: contract execution rejected by user
- `-32005`: transaction submission failed at the node

Note: there is no separate error code for simulation failure. When `validate: true` and simulation fails, the failure is surfaced as a warning in the approval prompt; the user's explicit decline is the only reason the request returns an error from the wallet's perspective.

These extend the existing codes (`-32000` general, `-32001` session already active, `-32002` no session, `-32003` invalid token).

## Risks / Trade-offs

- **Multiple network aliases for same genesis hash** → The first-alphabetically resolution is deterministic but may not pick the endpoint the user expects. Mitigation: document the resolution rule clearly; if it causes friction, a `--prefer-network` flag can be added later.
- **Schema display complexity** → Not all schema types map cleanly to human-readable JSON. Mitigation: use the SDK's schema decoding utilities where available; fall back to hex display rather than rejecting.
- **Password prompt per transaction may feel repetitive** → Mitigation: this matches the current `requestAccount` pattern and is intentional for security. Session-level key caching can be explored as a future opt-in.
- **Validation is opt-in so dApps that forget to set it get no energy estimate** → Mitigation: document `validate` clearly in the TS client and README; the energy ceiling the user supplies is the primary protection. The dApp ecosystem can adopt `validate: true` as a best practice.
- **Dry-run can give a different result than on-chain execution** → Contract state may change between dry-run and submission. Mitigation: the approval prompt labels simulation output as a preview, not a guarantee; the user retains final say regardless of simulation outcome.
