## Why

`ccd-wallet transaction show` currently reports transaction status and summary details, but it does not let users inspect the original submitted transaction payload. That makes it harder to debug what was actually sent on chain, especially for contract calls and other transactions whose summary omits important request data.

## What Changes

- Add a `--show-payload` flag to `ccd-wallet transaction show <HASH>`.
- When the flag is set and the transaction payload can be retrieved from block contents, show the original submitted payload in addition to the existing status and summary output.
- Keep the default `transaction show` output unchanged when the flag is not supplied.
- Provide clear behavior when the transaction is absent or only received and the original payload is not yet retrievable from block contents.
- Render decoded payload details when possible, with a stable fallback for payloads that cannot be decoded into a richer structured form.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `transaction-show`: extend transaction inspection so users can explicitly request the original submitted transaction payload alongside the existing lifecycle and summary output.

## Impact

- Affects `crates/ccd-wallet/src/cli.rs` for the new `--show-payload` flag.
- Affects `crates/ccd-wallet/src/commands/transaction/show.rs` and `crates/ccd-wallet/src/commands/transaction/render.rs` for payload retrieval and rendering.
- May require fetching block items in addition to transaction status/block info queries.
- Will require tests covering payload visibility across received, committed, finalized, and absent transaction states.
