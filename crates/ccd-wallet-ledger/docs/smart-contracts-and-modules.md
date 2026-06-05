# Smart contracts and module deployment

This area covers Ledger commands for deploying Wasm modules and signing smart-contract init/update transactions.

## Module deployment

Use `DeployModuleSigningRequest` with `sign_deploy_module`.

The request is split into:

- `path`: signing key derivation path
- `header_and_version`: serialized header, transaction kind, module version, and source length bytes
- `source`: serialized module source bytes

The Ledger flow sends the path-prefixed header/version stage first, followed by source chunks.

```rust
use ccd_wallet_ledger::{
    ConcordiumLedgerApp, DeployModuleSigningRequest, DerivationPath, MockTransport,
};

let mut signature_reply = vec![9; 64];
signature_reply.extend_from_slice(&[0x90, 0x00]);
let mut app = ConcordiumLedgerApp::new(MockTransport::new([
    vec![0x90, 0x00],
    signature_reply,
]));

let signature = app.sign_deploy_module(&DeployModuleSigningRequest {
    path: DerivationPath::new([1])?,
    header_and_version: vec![0xAA],
    source: vec![0xBB],
})?;
assert_eq!(signature.0, [9; 64]);
# ccd_wallet_ledger::Result::Ok(())
```

## Contract init and update

Use `ContractSigningRequest` with:

- `sign_init_contract`
- `sign_update_contract`

The request contains:

- `path`: signing key derivation path
- `header_and_data`: fixed-size serialized transaction prefix for the contract operation
- `name`: init or receive name bytes without the two-byte length prefix
- `parameter`: parameter bytes without the two-byte length prefix

The crate adds two-byte big-endian length prefixes to `name` and `parameter` before chunking those stages.

## What this crate does not do

This crate does not:

- parse Wasm modules,
- validate contract names,
- estimate energy,
- dry-run contract calls,
- decode schema parameters,
- submit the signed transaction.

Those responsibilities belong in higher-level code that understands nodes, contract schemas, and wallet UX.
