# Private-key export commands

The Concordium Ledger app exposes private-key export commands. This crate includes low-level APDU wrappers for completeness with the referenced clients and app implementations, but higher-level callers should treat these commands as security-sensitive.

The crate returns raw bytes from the device. It does not decide whether export is appropriate for a wallet flow, where exported bytes may live, or whether exported material is sufficient for a recoverable identity/account model.

## Legacy-path export (`INS=0x05`)

Use `ExportPrivateKeyLegacyRequest` with `export_private_key_legacy`.

The request contains:

- `mode`: APDU P1 display/export mode
- `export_type`: APDU P2 output type
- `payload`: serialized identity payload bytes (`identity[uint32]` for the legacy derivation path)

For observed app 5.4.1 code, the accepted P1/P2 values are:

| P1 | Meaning |
| --- | --- |
| `0x00` | PRF key, decrypt-credentials display wording |
| `0x01` | PRF key, recover-credentials display wording |
| `0x02` | PRF key followed by IDCredSec, create-credentials display wording |

| P2 | Meaning |
| --- | --- |
| `0x01` | ed25519 seed bytes for BLS key generation (deprecated by app docs) |
| `0x02` | derived BLS key bytes |

Responses are raw concatenated 32-byte values: 32 bytes for single-key modes and 64 bytes for `P1=0x02`.

```rust
use ccd_wallet_ledger::{
    ConcordiumLedgerApp, ExportPrivateKeyLegacyRequest, MockTransport,
};

let mut reply = vec![1; 32];
reply.extend_from_slice(&[0x90, 0x00]);
let mut app = ConcordiumLedgerApp::new(MockTransport::new([reply]));
let exported = app.export_private_key_legacy(&ExportPrivateKeyLegacyRequest {
    mode: 0x00,
    export_type: 0x02,
    payload: vec![0, 0, 0, 0],
})?;
assert_eq!(exported.len(), 32);
# ccd_wallet_ledger::Result::Ok(())
```

## Legacy new-path export (`INS=0x37` on app 5.4.1)

Use `ExportPrivateKeyNewPathLegacyRequest` with `export_private_key_new_path_legacy`.

The checked-out app tag `flex_1.6.0_5.4.1_sdk_v26.0.2` routes `INS=0x37` to the same legacy-style export handler as `INS=0x05`, but with a new derivation prefix and `idp || identity` payload. It does **not** implement the later purpose-based identity-credential-creation export.

| P1 | `ExportPrivateKeyNewPathLegacyMode` | Meaning |
| --- | --- | --- |
| `0x00` | `PrfKey` | PRF key, decrypt-credentials display wording |
| `0x01` | `PrfKeyRecovery` | PRF key, recover-credentials display wording |
| `0x02` | `PrfKeyAndIdCredSec` | PRF key followed by IDCredSec, create-credentials display wording |

| P2 | `ExportPrivateKeyNewPathLegacyOutput` | Meaning |
| --- | --- | --- |
| `0x01` | `Seed` | ed25519 seed bytes for BLS key generation |
| `0x02` | `BlsKey` | derived BLS key bytes |

The request payload is `identity_provider[uint32] || identity[uint32]`, both big-endian. Responses are raw concatenated 32-byte values in PRF-then-IDCredSec order.

```rust
use ccd_wallet_ledger::{
    ConcordiumLedgerApp, ExportPrivateKeyNewPathLegacyMode,
    ExportPrivateKeyNewPathLegacyOutput, ExportPrivateKeyNewPathLegacyRequest, MockTransport,
};

let mut reply = vec![1; 64];
reply.extend_from_slice(&[0x90, 0x00]);
let mut app = ConcordiumLedgerApp::new(MockTransport::new([reply]));
let exported = app.export_private_key_new_path_legacy(&ExportPrivateKeyNewPathLegacyRequest {
    mode: ExportPrivateKeyNewPathLegacyMode::PrfKeyAndIdCredSec,
    output: ExportPrivateKeyNewPathLegacyOutput::BlsKey,
    payload: [0u32.to_be_bytes(), 0u32.to_be_bytes()].concat(),
})?;
assert_eq!(exported.len(), 64);
# ccd_wallet_ledger::Result::Ok(())
```

## Purpose-based new export (`INS=0x37` on app 5.5.0 and newer)

Use `ExportPrivateKeyNewRequest` with `export_private_key_new` only when targeting app 5.5.0 or newer, or another app implementation known to support purpose-based exports.

This API is kept separate from `ExportPrivateKeyNewPathLegacyRequest` because app 5.5.0+ reuses `INS=0x37` with incompatible P1/P2 semantics and response framing. Purpose-based exports select a purpose with P1 and use P2 as a network designation (`0x00` mainnet, `0x01` testnet).

Purpose-based responses are length-prefixed key fields such as `[32][key]`. Callers must not parse legacy raw 32/64-byte responses as purpose-based identity issuance material.

For identity issuance on app 5.5.0+, callers should use:

- `export_type`: `ExportPrivateKeyNewType::IdentityCredentialCreation`
- `network`: selected chain network (`Mainnet` or `Testnet`)
- `payload`: `identity_provider[uint32] || identity[uint32]`
- response order: IDCredSec, PRFKey, signature blinding randomness

## Identity issuance warning

App 5.4.1 legacy new-path export can provide `PRFKey || IDCredSec`, but it does not expose deterministic signature blinding randomness. App 5.5.0+ purpose-based identity credential creation export provides all three recovery-critical issuance values. Higher-level Ledger-backed identity issuance flows must reject app 5.4.1 raw responses rather than completing with host-generated replacement randomness.

Real-device validation confirmed that this purpose-based identity issuance path works end-to-end on a physical Ledger device running Concordium app `5.6.2`.

## Security guidance

This crate only exposes protocol operations. Higher-level tools should decide whether these commands are appropriate, how to warn users, where bytes may be stored, and how to prevent accidental secret disclosure.
