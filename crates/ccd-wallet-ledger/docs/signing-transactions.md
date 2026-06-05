# Signing account transactions

Signing methods return `RawSignature`, a 64-byte Ed25519 signature produced by the Ledger device. They do not assemble signed Concordium transactions.

Higher-level code is responsible for:

1. constructing the Concordium transaction or relevant serialized fragments,
2. calling this crate for the Ledger signature,
3. wrapping the raw signature into Concordium account-signature structures,
4. submitting through a node client if desired.

## Generic chunked signing

Some Ledger commands sign a serialized transaction by prefixing the first APDU payload with the derivation path and then sending sequential chunks.

Use `ChunkedSigningRequest` for:

- `sign_transfer`
- `sign_configure_delegation`
- `sign_plt`

```rust
use ccd_wallet_ledger::{
    ChunkedSigningRequest, ConcordiumLedgerApp, DerivationPath, MockTransport,
};

let mut signature_reply = vec![9; 64];
signature_reply.extend_from_slice(&[0x90, 0x00]);
let mut app = ConcordiumLedgerApp::new(MockTransport::new([signature_reply]));

let request = ChunkedSigningRequest::new(
    DerivationPath::new([1])?,
    vec![0xAA; 32],
)?;
let signature = app.sign_transfer(&request)?;
assert_eq!(signature.0, [9; 64]);
# ccd_wallet_ledger::Result::Ok(())
```

## Memo and schedule transfers

Memo and schedule commands use staged APDU flows rather than one generic chunk sequence.

| Method | Request type | Stages |
| --- | --- | --- |
| `sign_transfer_with_memo` | `TransferWithMemoSigningRequest` | initial header/address/memo length, memo, amount |
| `sign_scheduled_transfer` | `ScheduledTransferSigningRequest` | initial header/address/schedule length, schedule pairs |
| `sign_scheduled_transfer_with_memo` | `ScheduledTransferWithMemoSigningRequest` | initial header/address/schedule/memo lengths, memo, schedule pairs |

Schedule pair data is chunked in 15-pair groups, matching the referenced JavaScript client.

## Baker, delegation, register data, and shielded transfer

| Method | Request type | Notes |
| --- | --- | --- |
| `sign_configure_delegation` | `ChunkedSigningRequest` | generic chunked signing |
| `sign_configure_baker` | `ConfigureBakerSigningRequest` | staged command: header/bitmap, first batch, aggregation keys, URL, fees, suspended flag |
| `sign_register_data` | `RegisterDataSigningRequest` | header stage then data chunks |
| `sign_transfer_to_public` | `TransferToPublicSigningRequest` | header, amount/recipient/proof-length, proof chunks |

## Raw signature semantics

`RawSignature::from_response` treats a one-byte signing response as a user-decline sentinel, mirroring the referenced JavaScript client behavior. Any other non-64-byte signing response is returned as an invalid-signature-length error.
