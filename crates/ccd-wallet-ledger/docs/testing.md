# Testing protocol flows

The crate is designed to test command construction and sequencing without a Ledger device.

## Unit-test command builders

Command-builder tests should assert APDU instruction values, P1/P2 sequencing, and chunk boundaries.

```rust
use ccd_wallet_ledger::{
    ScheduledTransferSigningRequest,
    commands::signing::build_scheduled_transfer_commands,
};

let request = ScheduledTransferSigningRequest {
    header_address_schedule_length: vec![1],
    schedule: vec![2; 300],
};
let commands = build_scheduled_transfer_commands(&request);
assert_eq!(commands.len(), 3);
assert_eq!(commands[1].data.len(), 240);
```

## Mock transport tests

Use `MockTransport` when testing client methods. Queue raw replies including APDU status words, then inspect captured commands.

```rust
use ccd_wallet_ledger::{
    ChunkedSigningRequest, ConcordiumLedgerApp, DerivationPath, MockTransport,
};

let mut reply = vec![9; 64];
reply.extend_from_slice(&[0x90, 0x00]);
let mut app = ConcordiumLedgerApp::new(MockTransport::new([reply]));
let request = ChunkedSigningRequest::new(DerivationPath::new([1])?, vec![2])?;
let signature = app.sign_transfer(&request)?;
assert_eq!(signature.0, [9; 64]);
assert_eq!(app.transport().commands().len(), 1);
# ccd_wallet_ledger::Result::Ok(())
```

## Recommended coverage

For each command family, prefer tests for:

- exact `Instruction` byte,
- command-specific P1/P2 stages,
- first/last chunk markers where applicable,
- path prefixing on first payload where applicable,
- status-word handling,
- final raw output parsing.

## Hardware or emulator tests

Physical-device or Speculos-style tests can be layered later. They should not replace mock APDU tests because mock tests document the exact protocol sequence expected by this crate.
