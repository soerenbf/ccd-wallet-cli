# Transport and testing

The crate exposes a small `GovernanceLedgerTransport` trait for APDU exchange. Command builders and client methods are written against this trait so protocol sequencing can be tested without a physical Ledger device.

## HID transport

The `hid` feature is enabled by default and provides `HidTransport` for direct Ledger HID APDU communication.

```rust,no_run
use ccd_wallet_ledger_governance::{GovernanceLedgerApp, HidTransport};

let transport = HidTransport::open_first()?;
let _app = GovernanceLedgerApp::new(transport);
# ccd_wallet_ledger_governance::Result::Ok(())
```

## Mock transport

`MockTransport` records every APDU command it receives and returns queued raw replies, including status words. Use it to validate exact command sequences.

```rust
use ccd_wallet_ledger_governance::{ApduCommand, GovernanceLedgerTransport, MockTransport};

let mut transport = MockTransport::new([vec![0x90, 0x00]]);
let _ = transport.exchange(&ApduCommand::new(0x01, 0, 0, vec![]))?;
assert_eq!(transport.commands().len(), 1);
# ccd_wallet_ledger_governance::Result::Ok(())
```
