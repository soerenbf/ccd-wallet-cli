# Staged and chunked governance updates

Some Governance Ledger app commands require multiple APDU packets so the device can display typed fields before signing.

The crate provides typed request structs and command sequencing for:

- `ProtocolUpdateRequest`
- `AddAnonymityRevokerRequest`
- `AddIdentityProviderRequest`
- `CreatePltRequest`

## Protocol updates

Protocol updates stage the initial update header and payload length, message bytes, specification URL bytes, specification hash, and auxiliary data chunks.

## Add anonymity revoker and identity provider

These flows stage description fields individually and then send key material in the protocol order expected by the device.

## Create PLT

Create PLT sends the initial update prefix, token metadata and initialization-parameter length, then chunks initialization parameters into APDU-sized payloads.
