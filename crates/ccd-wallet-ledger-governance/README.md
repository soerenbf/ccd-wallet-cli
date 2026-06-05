# ccd-wallet-ledger-governance

Low-level Rust client for the Concordium Governance Ledger hardware wallet application.

This crate is intentionally APDU-close. It translates typed governance-oriented request data into Governance Ledger app command sequences, handles command-specific staging and chunking, and returns raw device outputs such as public keys and signatures.

It does **not**:

- select governance signers,
- read or write the wallet database,
- unlock or inspect the governance key vault,
- prompt for passwords,
- assemble signed governance update instructions or block items,
- submit updates to a node,
- wait for finalization,
- blind-sign unknown serialized governance payloads.

Higher-level wallet code can build on this crate to map device signatures to governance key indices, assemble signed update instructions, provide CLI UX, or submit updates.

## Feature flags

| Feature | Description |
| --- | --- |
| `default` | Enables `hid` transport support. Does not enable SDK conversions. |
| `hid` | Enables concrete Ledger HID APDU transport support. |
| `sdk` | Enables selected conversions from `concordium-rust-sdk` governance/update types into crate-local value types. |

## Command areas

- [Transport and testing](docs/transport-and-testing.md)
- [Public-key export](docs/public-key-export.md)
- [Fixed-shape updates](docs/fixed-shape-updates.md)
- [Staged and chunked updates](docs/staged-and-chunked-updates.md)
- [Governance key updates](docs/governance-key-updates.md)
- [SDK integration](docs/sdk-integration.md)
- [Protocol references and discrepancies](docs/protocol-references.md)

## Minimal example

```rust
use ccd_wallet_ledger_governance::{
    DerivationPath, GovernanceLedgerApp, MockTransport, PublicKeyOptions,
};

let mut reply = vec![7; 32];
reply.extend_from_slice(&[0x90, 0x00]);
let transport = MockTransport::new([reply]);
let mut app = GovernanceLedgerApp::new(transport);

let response = app.get_public_key(
    DerivationPath::new([1])?,
    PublicKeyOptions::default(),
)?;
assert_eq!(response.public_key, [7; 32]);
# ccd_wallet_ledger_governance::Result::Ok(())
```

## Request model

The public methods accept crate-local request types, for example:

- `FixedUpdateRequest` and fixed-update aliases such as `ExchangeRateUpdateRequest`
- `ProtocolUpdateRequest`
- `AddAnonymityRevokerRequest`
- `AddIdentityProviderRequest`
- `CreatePltRequest`
- `HigherLevelKeyUpdateRequest`
- `AuthorizationsUpdateRequest`

These types are shaped around the Governance Ledger app protocol. They usually contain already serialized governance payload fragments because this crate owns APDU choreography, not high-level governance update construction.

## Return model

Signing methods return `RawSignature`, a 64-byte Ed25519 signature returned by the device. The caller is responsible for wrapping that signature into Concordium governance update signature structures.
