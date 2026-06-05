# SDK integration

SDK integration follows the same pattern as `ccd-wallet-ledger`:

- crate-local request and response types are the stable public API,
- `concordium-rust-sdk` is optional,
- conversions are enabled by the `sdk` feature.

Default builds do not require SDK conversions:

```bash
cargo check -p ccd-wallet-ledger-governance --no-default-features
```

SDK-enabled builds provide `From` conversions from actual SDK payload types plus explicit Ledger-only context to crate-local request types:

- `(GovernanceUpdatePrefix, P: Serial)` → `FixedUpdateRequest` and fixed-update aliases
- `(GovernanceUpdatePrefix, updates::ProtocolUpdate)` → `ProtocolUpdateRequest`
- `(GovernanceUpdatePrefix, id::types::ArInfo<id::constants::ArCurve>)` → `AddAnonymityRevokerRequest`
- `(GovernanceUpdatePrefix, id::types::IpInfo<id::constants::IpPairing>)` → `AddIdentityProviderRequest`
- `(GovernanceUpdatePrefix, updates::CreatePlt)` → `CreatePltRequest`
- `(GovernanceUpdatePrefix, HigherLevelKeyUpdateType, updates::HigherLevelAccessStructure<Kind>)` → `HigherLevelKeyUpdateRequest`
- `(GovernanceUpdatePrefix, AuthorizationsKeyUpdateType, AuthorizationsVersion, updates::AuthorizationsV0/V1)` → `AuthorizationsUpdateRequest`

The tuple inputs use the real SDK types. The extra tuple fields carry Ledger-only context that SDK payload types do not contain: derivation path, update header, and where needed the exact key-update-type or authorizations-version selector. This keeps conversions explicit rather than guessing missing device context.

```bash
cargo check -p ccd-wallet-ledger-governance --features sdk
```

Ambiguous SDK-to-Ledger choices, such as which exchange-rate queue or key-update discriminator is intended, are represented as explicit tuple context values rather than guessed by the conversion.
