# Private-key export commands

The Concordium Ledger app exposes private-key export commands. This crate includes low-level APDU wrappers for completeness with the referenced JavaScript client, but higher-level callers should treat these commands as security-sensitive.

## Legacy export

Use `ExportPrivateKeyLegacyRequest` with `export_private_key_legacy`.

The request contains:

- `mode`: APDU P1 display/export mode
- `export_type`: APDU P2 export type
- `payload`: serialized identity payload bytes

The method returns raw bytes from the device. The crate does not interpret whether those bytes are a private key only or a private key plus credential ID.

```rust
use ccd_wallet_ledger::{
    ConcordiumLedgerApp, ExportPrivateKeyLegacyRequest, MockTransport,
};

let mut reply = vec![1; 32];
reply.extend_from_slice(&[0x90, 0x00]);
let mut app = ConcordiumLedgerApp::new(MockTransport::new([reply]));
let exported = app.export_private_key_legacy(&ExportPrivateKeyLegacyRequest {
    mode: 0,
    export_type: 2,
    payload: vec![0, 0, 0, 0],
})?;
assert_eq!(exported.len(), 32);
# ccd_wallet_ledger::Result::Ok(())
```

## New export

Use `ExportPrivateKeyNewRequest` with `export_private_key_new`.

`ExportPrivateKeyNewType` maps to the APDU P1 values used by the referenced client:

- `IdentityCredentialCreation`
- `AccountCreation`
- `IdRecovery`
- `AccountCredentialDiscovery`
- `CreationOfZkProof`

The request's `payload` is passed through unchanged.

## Security guidance

This crate only exposes the protocol operation. Higher-level tools should decide whether these commands are appropriate, how to warn users, where bytes may be stored, and how to prevent accidental secret disclosure.
