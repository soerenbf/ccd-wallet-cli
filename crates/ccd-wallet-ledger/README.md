# ccd-wallet-ledger

Low-level Rust client for the Concordium Ledger hardware wallet application.

This crate is intentionally APDU-close. It translates Concordium-oriented request data into the Ledger app command sequences, handles command-specific chunking and staged uploads, and returns raw device outputs such as public keys, signatures, address-verification status, app metadata, or exported bytes.

It does **not**:

- select accounts,
- read or write the wallet database,
- prompt for passwords,
- assemble signed Concordium transactions,
- submit transactions to a node,
- wait for finalization.

Higher-level wallet code can build on this crate to assemble Concordium signatures, signed block items, CLI UX, or storage integration.

## Feature flags

| Feature | Description |
| --- | --- |
| default | No optional integrations. Uses crate-local request types only. |
| `sdk` | Enables selected conversions from `concordium-rust-sdk` types into crate-local value types. |
| `hid` | Reserved for a concrete HID transport adapter. Current command logic is transport-agnostic. |

## Command areas

- [Transport and APDU model](docs/transport.md)
- [Derivation paths](docs/derivation-paths.md)
- [Public keys and address verification](docs/public-keys-and-addresses.md)
- [Signing account transactions](docs/signing-transactions.md)
- [Smart contracts and module deployment](docs/smart-contracts-and-modules.md)
- [Credentials and identity-related commands](docs/credentials-and-identity.md)
- [Private-key export commands](docs/private-key-export.md)
- [Testing protocol flows](docs/testing.md)

## Minimal example

```rust
use ccd_wallet_ledger::{
    ConcordiumLedgerApp, DerivationPath, MockTransport, PublicKeyOptions,
};

let mut reply = vec![7; 32];
reply.extend_from_slice(&[0x90, 0x00]);
let transport = MockTransport::new([reply]);
let mut app = ConcordiumLedgerApp::new(transport);

let response = app.get_public_key(
    DerivationPath::new([1])?,
    PublicKeyOptions::default(),
)?;
assert_eq!(response.public_key, [7; 32]);
# ccd_wallet_ledger::Result::Ok(())
```

## Request model

The public methods accept crate-local request types, for example:

- `ChunkedSigningRequest`
- `TransferWithMemoSigningRequest`
- `ScheduledTransferSigningRequest`
- `ConfigureBakerSigningRequest`
- `DeployModuleSigningRequest`
- `ContractSigningRequest`
- `CredentialDeploymentSigningRequest`
- `UpdateCredentialsSigningRequest`

These types are shaped around the Ledger app protocol. They usually contain already serialized Concordium payload fragments because this crate owns APDU choreography, not high-level transaction construction.

## Return model

Signing methods return `RawSignature`, a 64-byte Ed25519 signature returned by the device. The caller is responsible for wrapping that signature into Concordium account-signature structures if it wants to build a signed transaction.
