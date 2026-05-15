## Context

The project already supports seed management, network selection, node connectivity, and browser-assisted identity issuance. Issued identities are stored in SQLite as plaintext relational metadata plus encrypted private payload data under the owning seed password domain. The wallet does not yet support the next protocol step: deriving and deploying a credential from a stored identity to create an on-chain Concordium account.

Account creation introduces three cross-cutting concerns. First, the flow spans CLI UX, Concordium SDK credential deployment, SQLite persistence, and seed-domain encryption. Second, account creation is constrained by identity usability rules: the selected identity must be issued, belong to the resolved network, and not be expired at submission time. Third, account persistence needs to start with only an encrypted account address, but the encrypted model is expected to grow later to support additional credential/key details.

## Goals / Non-Goals

**Goals:**
- Add a wallet-managed account creation flow built on stored issued identities.
- Persist accounts using plaintext indexing metadata plus an extensible encrypted private payload under the owning seed password domain.
- Reject expired identities during identity selection and before credential deployment submission.
- Allocate credential counters in the correct derivation scope: `(network_genesis_hash, seed_id, ip_identity, identity_index)`.
- Extend identity metadata so account creation can determine identity usability without decrypting every stored identity.

**Non-Goals:**
- Supporting account key rotation, adding additional credentials, or editing account thresholds in this change.
- Exposing export/import flows for credential deployment payloads.
- Persisting a full long-term account key vault separate from the seed-domain derivation model.
- Redesigning identity issuance beyond the metadata additions needed for account creation.

## Decisions

### 1. Account creation will derive account material from the seed on demand
The wallet will continue to treat the seed as the root secret. Account signing keys, public keys, and credential-related randomness will be derived from the unlocked seed and the selected identity tuple instead of being stored as separate long-term secrets.

**Rationale:** This stays aligned with Concordium's derivation model and the wallet's current seed-domain architecture. It minimizes new secret storage and keeps account recovery tied to the existing seed unlock flow.

**Alternatives considered:**
- **Store derived account private keys separately:** rejected because it duplicates recoverable secrets and complicates key custody.
- **Generate random non-derived account keys:** rejected because it breaks the seed-centric recovery model already established by the project.

### 2. Accounts will use plaintext relational metadata plus encrypted private payloads
A new account persistence model will store indexing and uniqueness fields in plaintext while placing the account address and future account-owned details inside an encrypted `AccountPrivatePayload` structure.

**Rationale:** The relational metadata is needed for selection, uniqueness checks, counter allocation, and lifecycle tracking without requiring a seed unlock. Using a struct instead of a plain encrypted address makes the payload extensible for future account key/credential details.

**Alternatives considered:**
- **Store the address as plaintext:** rejected because the desired privacy model is that seed-owned account details live under the seed password domain.
- **Encrypt the address as a bare string:** rejected because it creates an awkward migration path once more encrypted account details are added.

### 3. Identity usability metadata will surface only expiry in plaintext
Identity storage will be extended so account creation can determine whether an identity has expired without decrypting every stored identity payload. For now, only identity expiry will be promoted to plaintext metadata; other identity usability details will remain deferred until a concrete UX need emerges.

**Rationale:** Identity selection must exclude expired identities or the deployment transaction will fail. Plaintext expiry enables efficient filtering, clearer UX, and preflight validation before password prompts and network submission while keeping the plaintext metadata surface minimal.

**Alternatives considered:**
- **Promote additional identity metadata now:** rejected because expiry is the only currently required account-creation precondition and broader promotion would expand plaintext state prematurely.
- **Decrypt every candidate identity during selection:** rejected because it adds unlock friction and makes selection dependent on decrypting all identities.
- **Do no pre-check and rely on chain failure:** rejected because it degrades UX and turns a known local validation rule into a late failure.

### 4. Credential counters will be allocated per derivation tuple, not per label
Account credential counters will be unique within `(network_genesis_hash, seed_id, ip_identity, identity_index)`, matching the derivation scope rather than a wallet-local identity label.

**Rationale:** Concordium credential uniqueness follows derivation coordinates. Labels are UX metadata and are not the correct protocol-level uniqueness boundary.

**Alternatives considered:**
- **Allocate per identity label:** rejected because labels are wallet-local aliases and can obscure the real derivation identity.
- **Allocate globally per network:** rejected because it over-constrains unrelated identities.

### 5. The account creation flow will persist pending state before finalization
The flow will create a pending account record before submission/finalization and then mark it finalized once the credential deployment succeeds on chain.

By default, `account new` will wait for finalization. A flag will allow the user to skip waiting after successful submission, leaving the local account in `pending` status. Future account-using flows will apply lazy finalization: if the selected account is still marked pending locally, the wallet will check whether it has since finalized before proceeding.

**Rationale:** Waiting by default gives the clearest first-run UX. Supporting an explicit skip-wait path preserves flexibility for slow or asynchronous environments. Persisted pending state enables later lazy confirmation without losing submission context.

**Alternatives considered:**
- **Persist only after finalization:** simpler, but loses visibility into in-flight account creation and makes interruptions harder to reason about.
- **Never wait for finalization:** rejected because the default user expectation for `account new` is that the account is ready when the command succeeds.

### 6. Pending identities will use lazy confirmation when selected for account creation
If account creation is asked to use an identity that is still marked `pending`, the wallet will use the stored encrypted `code_uri` to poll the identity provider again. If the provider now reports `done`, the wallet will update the local identity record and continue. If it still reports `pending` or `error`, the wallet will stop with an actionable result.

Identity issuance should also support an explicit skip-wait option after browser callback success. By default, identity issuance continues to poll until completion, but the skip-wait option will leave the identity in `pending` status so that later account creation can lazily confirm it.

**Rationale:** Identity issuance and account creation are often separated in time. Lazy confirmation lets the wallet recover naturally from interrupted issuance flows without forcing users into a separate explicit identity-refresh command. Adding a symmetric skip-wait option to identity issuance gives users an intentional way to leave the identity pending instead of relying only on timeout or interruption behavior.

**Alternatives considered:**
- **Require a separate identity refresh command first:** rejected because it adds friction to a recoverable flow.
- **Treat all pending identities as unusable:** rejected because a provider-confirmed identity may already be available even if the local state was never updated.
- **Keep skip-wait only on account creation:** rejected because it makes the two long-polling flows inconsistent.

## Risks / Trade-offs

- **[Identity expiry parsing differs across provider payload shapes]** → Mitigation: normalize and validate the stored identity object/token shape when adding usability metadata, and fail fast with actionable errors if required fields are missing.
- **[Encrypted account address makes address display require decrypt-capable access paths]** → Mitigation: keep account selection and indexing driven by plaintext metadata, and reserve address display for flows that can unlock the owning seed when needed.
- **[Pending account state can diverge from final chain outcome on interruption]** → Mitigation: record explicit status transitions and transaction hash so future flows can inspect or reconcile stuck pending rows.
- **[Extensible encrypted payloads can accumulate incompatible shapes over time]** → Mitigation: use a structured payload model from the start and keep payload-schema evolution separate from cipher-version evolution.

## Migration Plan

1. Add a SQLite migration introducing account metadata and encrypted account private payload tables.
2. Extend identity persistence to record plaintext usability metadata needed for account creation, including expiry.
3. Add store-layer helpers for account insertion, status transitions, and next credential counter allocation.
4. Extend the wallet derivation wrapper with account-level derivation helpers required by Concordium credential deployment.
5. Add CLI account creation flow and wire it to existing seed/network selection patterns.
6. Update documentation with the new account creation command and identity eligibility behavior.

Rollback is limited to development/local wallet state because this wallet uses a local SQLite database. If the migration or flow is reverted during development, local databases created with the new schema may need to be recreated or explicitly migrated back.

## Open Questions

- How should future account-using commands expose the result of lazy finalization checks when a pending account is discovered to be failed, missing, or still not finalized?
- Should there be a dedicated explicit refresh/status command for identities and accounts in addition to the lazy confirmation behavior defined here?
