# Public-key export

Public-key export is part of the initial Governance Ledger client surface because operators need device-derived governance public keys when updating governance key sets on chain.

The client supports the Governance Ledger public-key command (`INS 0x01`) with options for device confirmation and signed public-key responses.

```rust
use ccd_wallet_ledger_governance::{
    DerivationPath, GovernanceLedgerApp, MockTransport, PublicKeyOptions,
};

let mut reply = vec![7; 32];
reply.extend_from_slice(&[0x90, 0x00]);
let mut app = GovernanceLedgerApp::new(MockTransport::new([reply]));

let response = app.get_public_key(
    DerivationPath::new([1])?,
    PublicKeyOptions { confirm_on_device: true, signed_key: false },
)?;
assert_eq!(response.public_key, [7; 32]);
# ccd_wallet_ledger_governance::Result::Ok(())
```

Higher-level wallet code can compare exported device public keys with chain authorization structures and include them in governance key update payloads.
