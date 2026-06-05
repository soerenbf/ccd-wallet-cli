# Governance key updates

The crate exposes typed requests for Governance Ledger app key-update flows:

- `HigherLevelKeyUpdateRequest` for root and level-1 key update flows.
- `AuthorizationsUpdateRequest` for level-2 authorization update flows.

`AuthorizationsUpdateRequest` includes an `AuthorizationsVersion` selector for V0, V1, and V2 protocol variants. The selector controls the P2 value sent to the Governance Ledger app.

The crate returns raw signatures only. Higher-level wallet code must map each signature to the correct governance key index and assemble the final update signature map.
