# Transport and APDU model

`ccd-wallet-ledger` separates Ledger app command logic from device transport.

## Transport abstraction

The crate uses `LedgerTransport` as the only transport requirement:

```rust
use ccd_wallet_ledger::{ApduCommand, LedgerTransport, Result};

struct MyTransport;

impl LedgerTransport for MyTransport {
    fn exchange_raw(&mut self, command: &ApduCommand) -> Result<Vec<u8>> {
        // Send command.cla / command.ins / command.p1 / command.p2 / command.data.
        // Return raw reply including the trailing two-byte APDU status word.
        # let _ = command;
        Ok(vec![0x90, 0x00])
    }
}
```

`LedgerTransport::exchange` is provided by the trait. It calls `exchange_raw`, strips the APDU status word, maps non-success status words into `LedgerError`, and returns status-stripped bytes.

## APDU commands

`ApduCommand` contains:

- `cla`: APDU class byte, normally `0xE0` for the Concordium Ledger app
- `ins`: instruction byte
- `p1`: first command parameter
- `p2`: second command parameter
- `data`: payload bytes

The command builders in `commands::device`, `commands::public_key`, and `commands::signing` construct the exact command sequence for each supported Ledger app flow.

## Status handling

`apdu::split_status_word` expects replies to end in a two-byte status word. Common statuses include:

| Status | Meaning |
| --- | --- |
| `0x9000` | Success |
| `0x6985` | User declined / conditions not satisfied |
| `0x6D00` | Instruction not supported |
| `0x6E00` | Invalid class byte |
| `0x6700` | Invalid length |

## Mock transport

`MockTransport` records commands and returns queued raw replies. It is the preferred way to test command sequencing without hardware.

```rust
use ccd_wallet_ledger::{ApduCommand, LedgerTransport, MockTransport};

let mut transport = MockTransport::new([vec![0x90, 0x00]]);
let command = ApduCommand::new(0x21, 0x00, 0x00, vec![]);
transport.exchange(&command)?;
assert_eq!(transport.commands()[0].ins, 0x21);
# ccd_wallet_ledger::Result::Ok(())
```
