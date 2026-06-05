## Why

The wallet currently models seeds as the only derivation authority for identities and derived accounts, while Ledger integration needs hardware-backed identities and accounts to participate in the same flows without storing signing secrets locally. Since the project is still pre-stable, this is the right moment to replace the seed-centric storage model with a long-term signer-owner model instead of adding permanent Ledger-specific special cases.

## What Changes

- **BREAKING**: Replace seed-centric identity/account ownership with a signer-owner model where both seed and Ledger owners can own identities, derived accounts, and a local password-protected encryption domain.
- Add Ledger signer-owner enrollment using a canonical Ledger public key retrieved at a fixed enrollment derivation path as the stable owner identity, with a short derived fingerprint for display.
- Store seed-specific secret payloads and Ledger-specific enrollment metadata in owner-kind detail tables beneath the shared signer-owner abstraction.
- Encrypt identity private payloads and derived-account private payloads under the owning signer owner's DEK, regardless of whether that owner is seed-backed or Ledger-backed.
- Keep imported accounts and governance key vaults separate from signer owners because they are not signer-owner-derived identity/account material.
- Update identity issuance, account creation, account signing-source resolution, listing/selection, and network reset semantics to understand signer owners.
- Add a higher-level Ledger identity/account construction layer that bridges wallet identity/account creation flows to the low-level Ledger APDU client without storing Ledger private signing material locally.
- Update database and encryption documentation to describe signer-owner ownership, vaults, AAD binding, uniqueness rules, and cascade behavior.

## Capabilities

### New Capabilities
- `signer-owner-storage`: Stores wallet-local signer owners, signer-owner password domains, seed-specific owner secrets, and Ledger-specific owner enrollment metadata.
- `ledger-signer-owner`: Enrolls and recognizes Ledger-backed signer owners using canonical Ledger public-key identity and supports Ledger owners as derivation authorities for identities and accounts.
- `ledger-identity-account-construction`: Prepares Ledger-backed identity issuance and account credential deployment payloads and routes required approvals/signatures through the Ledger app.

### Modified Capabilities
- `seed-storage`: Seeds become seed-kind signer owners with seed secret payloads stored under the signer-owner vault model.
- `identity-storage`: Identities are owned by signer owners instead of directly by seeds, and private identity payloads are encrypted under the signer-owner password domain.
- `account-storage`: Derived accounts are owned by signer owners instead of directly by seeds; imported accounts remain imported-vault backed.
- `account-signing-source`: Signing-source resolution distinguishes seed-backed derived accounts, Ledger-backed derived accounts, and imported accounts.
- `entity-listing`: Identity and account listing identifies seed and Ledger owner context without decrypting private payloads by default.
- `network-reset-delete`: Network reset removes network-scoped identities/accounts and network-scoped imported vault data while preserving signer owners and owner vaults.

## Impact

- Affected Rust store code: `crates/ccd-wallet-core/src/store/*`, schema migrations, account/identity records, encryption helpers, and tests.
- Affected CLI flows: seed commands, identity issuance, account creation/list/show/rename/export/signing-source resolution, connect account selection/signing, and network reset.
- Affected Ledger integration: higher-level CLI orchestration will use `crates/ccd-wallet-ledger` to enroll Ledger signer owners and to derive/sign for Ledger-backed identity/account flows.
- Affected documentation: `docs/db-structure.md`, `docs/encryption-model.md`, `docs/commands.md` where command behavior changes, and relevant README examples.
- Compatibility: pre-stable breaking DB reset/migration is acceptable for this change; favor the clean long-term schema over backwards-compatible incremental migration complexity.
