## Context

The SDK dependency branch adds `UpdatePayload::MaxLockDuration`, `UpdateType::UpdateMaxLockDuration`, `NextUpdateSequenceNumbers::max_lock_duration`, `ChainParameters::max_lock_duration`, `Level2Keys::token_parameters`, V3 level-2 authorization variants, and `LockConfig::metadata`. The wallet already queries normalized chain parameters internally for governance, but it has no public chain-parameter inspection command. The normalized SDK chain-parameter types derive `Debug` but not Serde traits; the already-enabled `serde_deprecated` feature does not change that.

Read-only commands already establish the relevant CLI conventions: endpoint selection through `--network` or `--node`, active-network fallback, `--no-defaults` network selection, shared block-selector parsing with `LastFinal` as the default, and `--json` where structured output exists. The new command must reuse these behaviors rather than introduce a parallel input model.

The local Governance Ledger client models APDU operations only through Create PLT and authorization version V2. No trusted APDU instruction or version selector is available for Max Lock Duration or authorization V3.

## Goals / Non-Goals

**Goals:**
- Put chain-parameter inspection behind one generic read-only command and one stable wallet-owned output model.
- Expose pending governance updates and next update sequence numbers as one snapshot-consistent chain-update view.
- Make both commands indistinguishable from comparable read-only commands in flag spelling, endpoint and block resolution, prompting, output selection, and errors.
- Integrate Max Lock Duration and token-parameters authorization through every local governance path.
- Fail explicitly and early when a typed update is unsupported by Ledger firmware/protocol.

**Non-Goals:**
- Adding or guessing Governance Ledger APDU commands.
- Adding lock metadata authoring to token commands.
- Persisting chain parameters locally or changing database schemas.
- Creating a separate command dedicated only to Max Lock Duration.
- Reconstructing sequence numbers for individual pending updates, because the node API does not return them.

## Decisions

### Reuse the existing read-only command contract

`node chain-parameters` will expose the same public selectors as comparable node-backed inspection commands: `--network`, `--node`, `--block`, `--json`, and `--no-defaults`. It will reuse `node::resolve_endpoint_context` and the existing shared block parser. There is no required positional value and therefore no command-specific prompt or `--non-interactive` flag.

This is preferred over a bespoke chain-parameter argument group because it minimizes semantic drift. If shared clap groups can be used without changing existing conflict behavior, they may be reused; otherwise duplicating the established public shape is preferable to a premature abstraction.

### Query the normalized SDK response and retain query context

The handler will call `get_block_chain_parameters` with the parsed block identifier and retain both the returned normalized `ChainParameters` and resolved block hash. `LastFinal` remains the default. Endpoint and block context will be represented in the output so operators can identify the observed state.

A raw protobuf response was considered, but it would leak protocol-version wire variants and couple CLI output to generated types. The normalized SDK response is a better domain boundary.

### Keep effective parameters and update state as sibling commands

`node chain-parameters` represents the effective parameter state at a block. `node chain-updates` will represent scheduled future effects plus the counters required for the next submission. They will be sibling commands rather than flags or sections of one command:

```text
node
├─ chain-parameters   effective state
└─ chain-updates      pending state + next counters
```

This separation keeps each output coherent while avoiding two narrowly scoped commands for pending updates and sequence numbers. Pending effects and next sequence numbers belong together operationally: one explains accepted future changes, and the other explains which sequence a subsequent proposal must use.

### Resolve chain-update queries to one block snapshot

The chain-updates handler will first call `get_next_update_sequence_numbers` using the requested selector. It will then call `get_block_pending_updates` with `BlockIdentifier::Given` using the block hash returned by the first call. The emitted context uses that same hash.

This avoids a race where two calls using `best` or `last-final` observe different blocks. Sequence numbers are queried first because that endpoint returns a bounded response immediately; the pending-update endpoint then streams all records from the fixed block. Reversing the order would require opening a stream before obtaining the concrete snapshot hash.

### Introduce explicit serializable output models

A wallet-owned `ChainParametersOutput` hierarchy will mirror every normalized SDK chain-parameter field, including update keys and access structures. It will derive Serde serialization with camel-case names. Optional protocol-version fields will serialize as `null`, preserving a stable schema across protocol versions. Human rendering and JSON serialization will consume this same model.

Direct serialization is unavailable: neither the normalized SDK type nor several of its nested normalized structures implement `Serialize`, and Rust's orphan rules prevent implementing it in the wallet. Debug formatting was rejected because it is unstable, Rust-oriented, and unsuitable for `--json` automation.

Concordium domain values will be converted without losing precision. Durations will use their Concordium string representation; amounts, rates, ratios, thresholds, indices, keys, and addresses will use explicit string or structured representations consistent with existing wallet JSON conventions. Conversion helpers will be total over a successfully normalized SDK response and return errors only where formatting can genuinely fail.

### Keep stdout mode-pure

Human mode will print a scannable sectioned view. JSON mode will print exactly one pretty JSON document to stdout with no cliclack decorations or explanatory prefix. Connection and query failures will use contextual errors naming the endpoint and operation. The commands will not use spinners because comparable short read-only queries do not.

The chain-updates JSON model will contain `endpoint`, `blockHash`, `nextSequenceNumbers`, and `pendingUpdates`. Sequence fields will be exhaustive over the normalized SDK structure. Pending updates will preserve stream order and expose effective time plus a tagged typed effect. An `Upward::Unknown` effect will become an explicit unknown marker rather than a serialization failure; protobuf does not preserve residual payload bytes for this wrapper, so no unknown payload can be shown.

### Model token parameters as a first-class governance capability

The governance capability model will gain token-parameters authority, with label, live access-structure lookup, key listing, key matching, threshold selection, blind-sign aliases, and default sequence queue support. Max Lock Duration maps to that capability and to its dedicated sequence queue.

This is preferred over grouping the update under Create PLT because P11 provides a distinct access structure and sequence number; conflating them could select unauthorized keys or submit a stale sequence number.

### Recognize authorization V3 without pretending Ledger support

Root and level-1 V3 authorization variants will be accepted by typed local preparation and signing. Ledger validation will classify Max Lock Duration and authorization V3 as known but unsupported and reject them before opening the HID transport. This preserves typed review semantics while avoiding guessed APDU bytes or malformed device requests.

If a future trusted Governance Ledger protocol adds these operations, a separate change can extend the low-level client, APDU tests, SDK conversions, and CLI dispatch.

### Treat lock metadata as compatibility-only

Existing lock creation will initialize the SDK's new `metadata` field to `None`. This preserves the current encoded behavior and keeps metadata UX outside this change.

## Risks / Trade-offs

- [The explicit output model is sizeable and must track future SDK fields] → Keep conversion code exhaustive and test representative old- and P11-shaped responses so SDK additions fail visibly during maintenance.
- [Duration strings are human-friendly but require parsing for numeric automation] → Follow the Concordium duration representation consistently and document it; retain exact millisecond precision.
- [Human output for all parameters can be long] → Use stable sections and indentation rather than omitting fields that operators requested.
- [Pending-update streams can fail after returning some items] → Collect the complete stream before printing anything so the command either emits a complete snapshot or an actionable failure.
- [A future unknown pending effect has no protobuf residual payload] → Preserve its effective time and mark the effect unknown without inventing details.
- [Ledger users may expect every decoded update to be signable] → Reject before device interaction with an error that distinguishes unsupported typed updates from blind payloads and points to local signing.
- [Existing read-only commands are not perfectly uniform internally] → Treat observable flags, resolution behavior, prompts, stdout mode, and errors as the compatibility contract rather than copying incidental implementation structure.

## Migration Plan

No persisted migration is required. Ship the command, SDK compatibility updates, governance mappings, tests, and documentation together. Rollback consists of reverting the wallet change and SDK branch update; no wallet state requires repair.
