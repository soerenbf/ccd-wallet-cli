## Context

The current wallet storage model treats seeds as the only derivation authority for issued identities and seed-derived accounts. `identities` rows reference `seed_id`, derived `accounts` rows reference `seed_id` plus identity/credential coordinates, and identity/account private payloads are encrypted under the owning seed's DEK. Imported accounts and governance keys already use separate vault domains because they are not seed-derived.

Ledger integration changes the domain model: Ledger-backed identities and Ledger-backed accounts are still derived identity/account material, but the signing secret is held by hardware rather than in the wallet database. These Ledger-backed objects still need wallet-local encrypted private payloads such as identity issuance state, issued identity objects, and cached account addresses. The clean long-term abstraction is therefore a signer owner: a wallet-local derivation authority with its own password-protected local encryption domain.

The project is pre-stable and resets are expected, so this change intentionally favors a clean schema over compatibility-preserving migration complexity.

## Goals / Non-Goals

**Goals:**
- Introduce signer owners as the shared ownership model for seed-backed and Ledger-backed derivation authorities.
- Give every signer owner one wallet-local password domain that encrypts signer-owned identity private payloads and derived-account private payloads.
- Represent seed secrets and Ledger enrollment metadata as owner-kind-specific details below the shared signer-owner model.
- Identify Ledger signer owners by a stable canonical public key returned by the Ledger app at a fixed enrollment derivation path, with a short fingerprint for display.
- Keep imported accounts and governance key vaults separate from signer owners.
- Update identity/account storage, signing-source resolution, listing, and network reset semantics to use signer owners.

**Non-Goals:**
- Preserving existing pre-stable database contents through a compatibility migration.
- Treating a physical USB transport endpoint or device path as persistent Ledger identity.
- Storing Ledger private signing material locally.
- Folding imported account vaults or governance key vaults into signer-owner vaults.
- Adding a generic blind-signing abstraction over the low-level Ledger protocol crates.

## Decisions

### 1. Model seeds and Ledgers as signer owners

**Decision:** Add a top-level `signer_owners` table with `owner_kind IN ('seed', 'ledger')`, a stable owner id, label, and timestamps. Identities and derived accounts reference `signer_owner_id` instead of `seed_id`.

**Rationale:** Both seeds and Ledgers are derivation authorities for identities and accounts. The schema should represent that directly instead of permanently special-casing seed ownership and Ledger ownership.

**Alternatives considered:**
- Keep `seeds` as the primary owner and add nullable `ledger_id` columns. Rejected because it encodes the same owner concept twice and makes every identity/account query branch on source-specific columns.
- Add `source_kind = 'ledger'` to accounts only. Rejected because Ledger identities also need first-class ownership and an encryption domain.

### 2. Give signer owners a shared local vault domain

**Decision:** Add `signer_owner_vaults` as the common password-protected DEK wrapper for all signer owners. Identity private payloads and derived-account private payloads encrypt under the owning signer owner's DEK.

**Rationale:** Ledger owners need the same at-rest privacy model as seed owners for local identity state and account address payloads, while still requiring hardware for signing.

**Alternatives considered:**
- Store Ledger-owned identity/account payloads in plaintext. Rejected because it weakens the wallet's existing privacy model.
- Create per-network Ledger vaults. Rejected because the signer owner, not the network, is the derivation authority and password domain.

### 3. Split owner-kind details into separate tables

**Decision:** Store seed-only encrypted seed bytes in `seed_owner_secrets` and Ledger-only enrollment metadata in `ledger_owner_details` rather than adding nullable detail columns to `signer_owners` or `signer_owner_vaults`.

**Rationale:** The shared owner and vault tables stay conceptually tight. Seed owners store a local signing secret; Ledger owners store an enrollment identity and never store signing secret material.

**Alternatives considered:**
- Put encrypted seed payload columns directly in `signer_owner_vaults`. Rejected because those columns are meaningless for Ledger owners.
- Keep a separate `seeds` table as the detail table. Rejected for long-term clarity; a seed is now a kind of signer owner, not the primary abstraction.

### 4. Identify Ledger owners by canonical public key, not transport identity

**Decision:** During Ledger enrollment, request a public key from the fixed canonical enrollment path `m/44'/919'/0'/0'/0'` using the Concordium Ledger app. Store the full returned public key as the unique Ledger owner identity, store the enrollment path, and derive a short fingerprint for display.

**Rationale:** Physical transport identity is not stable across unplug/replug or device replacement. The stable identity is the hardware wallet root/key material. The current low-level Ledger crate exposes public-key retrieval by path, but not an app-level master-fingerprint API, so a canonical public key is the practical stable anchor.

**Alternatives considered:**
- Use a USB/HID path or device identifier. Rejected because it identifies the transport instance rather than the wallet root.
- Store only a short fingerprint. Rejected because fingerprints are display identifiers; the full canonical public key is the stronger unique key.

### 5. Keep derived and imported accounts as the account source split

**Decision:** Keep account `source_kind` as `derived | imported`. For derived accounts, `signer_owner_id` plus identity/credential coordinates identifies the source. The owning signer's kind determines whether signing is seed-backed or Ledger-backed.

**Rationale:** Ledger-backed accounts are still derived accounts. Imported accounts are different because their secret material is stored locally under an imported-account vault.

**Alternatives considered:**
- Add `source_kind = 'ledger'`. Rejected because it conflates derivation ownership with the account source family and would make seed-derived and Ledger-derived accounts look less related than they are.

### 6. Preserve network-scoped imported and governance vaults

**Decision:** Imported account vaults and governance key vaults remain network-scoped and separate from signer-owner vaults. Network reset prunes network-scoped child rows and network-scoped vaults, but does not delete signer owners or signer-owner vaults.

**Rationale:** Imported accounts and governance keys are not identities/accounts derived from a signer owner. Signer owners are reusable across networks, so network reset should not delete them.

### 7. Keep seed commands intact and add separate Ledger setup UX

**Decision:** The external CLI surface keeps the existing `seed` command family for seed-backed signer owners. Ledger-backed signer owners will be introduced through a separate Ledger setup/enrollment section rather than by generalizing the seed command family into public `owner` or `signer` commands.

**Rationale:** Seeds already have a clear user-facing command space. Ledger setup is a distinct workflow, and keeping it separate avoids exposing the internal signer-owner abstraction directly in the CLI.

**Alternatives considered:**
- Replace `seed` commands with public `signer owner` commands. Rejected because it exposes internal terminology and would unnecessarily destabilize existing seed UX.
- Fold Ledger setup into `seed` commands. Rejected because Ledger enrollment is not seed management and should remain a distinct user workflow.

### 8. Use `key source` as the user-facing term

**Decision:** The implementation will keep `signer owner` as the internal storage and design term, but use `key source` as the user-facing umbrella term when the UI needs a concept that covers both seeds and Ledgers.

**Rationale:** `signer owner` is a good internal modeling term but not good CLI language. `key source` is short, understandable, and accurate for both locally stored seed roots and enrolled Ledger-backed roots.

**Alternatives considered:**
- `master key`. Rejected because it overemphasizes direct secret ownership and sounds more dangerous than intended.
- `root` or `root key`. Rejected because the terminology is too technical for the CLI.
- Expose `signer owner` directly. Rejected because it is internal modeling language rather than user-facing UX.

### 9. Add a Ledger identity/account construction bridge before full Ledger issuance/signing

**Decision:** Add a dedicated higher-level construction layer between CLI identity/account flows and the low-level `ccd-wallet-ledger` APDU client. This layer is responsible for preparing Concordium identity issuance and credential deployment values, staging Ledger request payloads, invoking Ledger approval/signing commands, and returning SDK-compatible outputs to the existing CLI orchestration. It must be explicit about which Ledger app capabilities are used and must fail safely when a flow cannot be represented by the Ledger app.

**Rationale:** The seed-backed flows currently construct identity requests and credential deployments with `ConcordiumHdWallet`, which exposes local derivation methods such as `get_id_cred_sec`, `get_prf_key`, `get_blinding_randomness`, account signing key derivation, and attribute randomness derivation. The low-level Ledger crate intentionally exposes APDU-close primitives such as public-key retrieval, public-info-for-IP signing, credential-deployment signing, update-credentials signing, and private-key export commands. It does not currently construct the cryptographic identity or credential payloads required by the CLI. A bridge layer keeps this construction logic explicit, testable, and separate from storage and prompt code.

**Private-key export policy:** Ledger private-key export commands SHALL NOT be used as an implicit fallback for Ledger-backed identities or accounts. If a Ledger-backed flow requires exporting key material from the device, that must be modeled as an explicit, user-approved capability with clear security wording. The preferred path is to keep Ledger-backed signing/approval on-device and avoid storing exported Ledger secrets in the wallet database.

**Alternatives considered:**
- Reuse `ConcordiumHdWallet` by exporting Ledger private key material and constructing everything locally. Rejected as the default because it undermines the purpose of Ledger-backed key sources and stores/handles hardware-derived secrets locally.
- Put high-level construction directly in command handlers. Rejected because it would mix prompt flow, database state, Ledger APDU choreography, and Concordium credential construction in one place.
- Expand the low-level Ledger crate into a wallet orchestration crate. Rejected because the Ledger crate is intentionally protocol-focused and reusable without wallet database dependencies.

## Risks / Trade-offs

- **Ledger canonical path choice may conflict with future ecosystem convention** → Store `enrollment_path` explicitly and keep matching keyed by the full public key returned at that path, allowing future path-version handling.
- **Cross-table owner-kind invariants are not fully expressible with simple SQLite CHECK constraints** → Enforce them in store APIs and tests: seed owners must have seed details and no Ledger details; Ledger owners must have Ledger details and no seed secret row.
- **Large pre-stable schema rewrite touches many flows** → Keep implementation staged: store model first, then seed parity, then Ledger enrollment, then identity/account flows.
- **Ledger-backed account signing requires different transaction assembly from local-key signing** → Preserve source-aware signing resolution and keep low-level Ledger command details inside the Ledger crates/higher-level integration helpers.
- **Owner labels and existing seed labels become one namespace** → Use a single signer-owner label namespace to avoid selection ambiguity and make active-owner behavior straightforward.

## Migration Plan

Because the project is pre-stable, implementation may replace the baseline schema and reset local development databases rather than preserving existing rows. If a compatibility path is desired later, it can map each existing seed row to a seed-kind signer owner, move `seed_vaults` DEK metadata into `signer_owner_vaults`, move seed payload ciphertext into `seed_owner_secrets`, and replace identity/account `seed_id` references with `signer_owner_id`.

Rollback during development is straightforward: restore the previous baseline schema and seed-centric store modules before releasing the signer-owner model.

## Open Questions

- Should active wallet state be renamed immediately from active seed to active signer owner internally, or introduced as a compatibility layer in the first implementation step while the CLI uses `active key source` as the user-facing term?
