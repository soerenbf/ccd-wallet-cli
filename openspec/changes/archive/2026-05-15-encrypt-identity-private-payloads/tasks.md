## 1. Schema reset

- [x] 1.1 Consolidate existing SQLite migrations into a new `001_initial_schema.sql`.
- [x] 1.2 Reset migration metadata so the current schema version is the new initial schema version.
- [x] 1.3 Include `identities` with `seed_id REFERENCES seeds(id) ON DELETE CASCADE`.
- [x] 1.4 Add `identity_private_payloads` with `identity_id REFERENCES identities(id) ON DELETE CASCADE`.
- [x] 1.5 Remove plaintext `code_uri` and `identity_object` columns from the consolidated identity schema.
- [x] 1.6 Update migration tests for the reset development schema.

## 2. Seed unlock domain

- [x] 2.1 Add a seed unlock context that exposes the plaintext seed phrase and seed DEK to authorized in-process callers after password verification.
- [x] 2.2 Keep existing seed unlock behavior for callers that only need the seed phrase.
- [x] 2.3 Ensure key material remains zeroized after use.
- [x] 2.4 Add tests for seed unlock context correctness and wrong-password rejection.

## 3. Encrypted identity storage

- [x] 3.1 Update identity records to use `seed_id` instead of `seed_label` internally.
- [x] 3.2 Add an identity private payload model containing `code_uri` and optional `identity_object`.
- [x] 3.3 Implement private payload encryption using the owning seed DEK, unique nonce, cipher version, and identity-specific AAD.
- [x] 3.4 Implement private payload decryption for tests and future callers.
- [x] 3.5 Update `insert_pending` to insert public metadata and encrypted private payload data transactionally.
- [x] 3.6 Update completion transitions to keep private payload data encrypted and provider-error transitions to delete the pending identity.
- [x] 3.7 Update identity storage tests for encryption, wrong-key failure, AAD mismatch, uniqueness, index assignment, and cascade behavior.

## 4. Identity issuance integration

- [x] 4.1 Update `identity new` to unlock the selected seed once into the new seed unlock context.
- [x] 4.2 Use the unlocked seed phrase for identity request construction.
- [x] 4.3 Use the unlocked seed DEK when storing encrypted identity private payloads.
- [x] 4.4 Ensure `code_uri` and identity object JSON are not passed to storage APIs that write plaintext columns.
- [x] 4.5 Keep loopback/manual callback UX unchanged.

## 5. Documentation and developer reset guidance

- [x] 5.1 Update README to document that identity private payloads are encrypted under the seed password domain.
- [x] 5.2 Document that this development change requires deleting/recreating existing `wallet.db` files.
- [x] 5.3 Mention that identity labels and metadata remain plaintext.

## 6. Validation

- [x] 6.1 Run `cargo fmt`.
- [x] 6.2 Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [x] 6.3 Run `cargo test --workspace`.
