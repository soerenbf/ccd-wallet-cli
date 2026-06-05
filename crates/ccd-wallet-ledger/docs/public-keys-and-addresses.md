# Public keys and address verification

This area covers device commands that identify key material without signing transactions.

## Public key retrieval

Use `ConcordiumLedgerApp::get_public_key` with a derivation path and `PublicKeyOptions`.

```rust
use ccd_wallet_ledger::{ConcordiumLedgerApp, DerivationPath, MockTransport, PublicKeyOptions};

let mut reply = vec![7; 32];
reply.extend_from_slice(&[0x90, 0x00]);
let mut app = ConcordiumLedgerApp::new(MockTransport::new([reply]));

let response = app.get_public_key(
    DerivationPath::new([1])?,
    PublicKeyOptions {
        confirm_on_device: false,
        signed_key: false,
    },
)?;

assert_eq!(response.public_key, [7; 32]);
# ccd_wallet_ledger::Result::Ok(())
```

If `signed_key` is enabled, the response stores any bytes after the first 32-byte public key in `signed_public_key`.

## Address verification

Use `VerifyAddressRequest` for the current Ledger app address-verification command.

```rust
use ccd_wallet_ledger::{ConcordiumLedgerApp, MockTransport, VerifyAddressRequest};

let mut app = ConcordiumLedgerApp::new(MockTransport::new([vec![0x90, 0x00]]));
app.verify_address(&VerifyAddressRequest {
    payload: vec![0, 0, 0, 0],
})?;
# ccd_wallet_ledger::Result::Ok(())
```

Use `LegacyVerifyAddressRequest` and `verify_address_legacy` for the legacy address-verification P1 value.

```rust
use ccd_wallet_ledger::{ConcordiumLedgerApp, LegacyVerifyAddressRequest, MockTransport};

let mut app = ConcordiumLedgerApp::new(MockTransport::new([vec![0x90, 0x00]]));
app.verify_address_legacy(&LegacyVerifyAddressRequest {
    payload: vec![0, 0, 0, 0],
})?;
# ccd_wallet_ledger::Result::Ok(())
```

The crate does not derive or validate the address payload semantically. Callers pass the serialized payload expected by the Ledger app. This keeps the API APDU-close and avoids mixing protocol exchange with account-resolution logic. The legacy variant is a separate function instead of a flag on the current request type so callers select legacy behavior deliberately.

## App name lookup

Use `get_app_name` to query raw app-name bytes.

```rust
use ccd_wallet_ledger::{ConcordiumLedgerApp, MockTransport};

let mut reply = b"Concordium".to_vec();
reply.extend_from_slice(&[0x90, 0x00]);
let mut app = ConcordiumLedgerApp::new(MockTransport::new([reply]));
assert_eq!(app.get_app_name()?, b"Concordium");
# ccd_wallet_ledger::Result::Ok(())
```
