# CLI Command Taxonomy

This document is the canonical reference for the `ccd-wallet` command taxonomy.
It describes:
- the current implemented command surface
- planned command spaces that future work should follow
- naming rules for keeping user-facing commands stable even when protocol payloads evolve

## Source of truth

- **Implemented command surface:** `crates/ccd-wallet/src/cli.rs` and the command modules it dispatches to
- **Canonical taxonomy document:** `docs/commands.md`

When a change affects the command surface or the intended taxonomy, update both the clap definitions and this document in the same change.

## Taxonomy rules

- Prefer **user-facing domain names** over protocol payload names.
- Keep protocol transport terms such as `TokenUpdate` and `MetaUpdate` as **implementation details**, not primary command paths.
- Use **nested grouping** when it improves clarity.
- Mark documented command branches as **Implemented** or **Planned**.
- Model staking and validator flows for **recent protocol versions** only; do not revive deprecated legacy baker transaction families.

## Implemented command spaces

The following tree reflects the current command surface implemented in `crates/ccd-wallet/src/cli.rs`.

```text
ccd-wallet
├─ node                         [Implemented]
│  └─ info
├─ network                      [Implemented]
│  ├─ add
│  ├─ delete
│  ├─ list
│  ├─ rename
│  ├─ reset
│  ├─ show
│  └─ use
├─ transaction                  [Implemented]
│  └─ show [--show-payload]
├─ contract                     [Implemented]
│  ├─ deploy-module
│  ├─ init
│  ├─ update
│  ├─ invoke
│  ├─ show
│  ├─ parameter-template
│  │  ├─ init
│  │  └─ receive
│  └─ download-module
├─ seed                         [Implemented]
│  ├─ add
│  ├─ delete
│  ├─ list
│  ├─ rename
│  ├─ sync
│  ├─ use
│  └─ show
├─ ledger                       [Implemented]
│  ├─ setup                     (supports --restore <NETWORK>)
│  ├─ sync
│  ├─ show
│  └─ remove
├─ identity                     [Implemented]
│  ├─ list
│  ├─ new
│  └─ rename
├─ account                      [Implemented]
│  ├─ export
│  ├─ import
│  │  └─ genesis
│  ├─ list
│  ├─ new
│  ├─ show
│  └─ rename
├─ governance                   [Implemented]
│  ├─ keys
│  │  ├─ import
│  │  ├─ list
│  │  └─ remove
│  └─ update
├─ token                        [Implemented]
│  ├─ show
│  ├─ transfer
│  ├─ mint
│  ├─ burn
│  ├─ allow-list
│  │  ├─ add
│  │  └─ remove
│  ├─ deny-list
│  │  ├─ add
│  │  └─ remove
│  ├─ pause
│  ├─ unpause
│  ├─ admin-roles
│  │  ├─ assign
│  │  └─ revoke
│  ├─ metadata
│  │  └─ update
│  └─ lock
│     ├─ create
│     ├─ fund
│     ├─ send
│     ├─ return
│     ├─ cancel
│     └─ show
└─ connect                      [Implemented]
```

## Key-source terminology

The internal storage model calls seed-backed and Ledger-backed derivation authorities **signer owners**. User-facing CLI text should call the same concept a **key source** when a command or prompt needs to cover both seed phrases and Ledger devices.

The `seed` command family remains the user-facing place for seed phrase management. Ledger setup uses the separate `ledger setup` flow and enrolls a Ledger-backed key source by reading a canonical public key from the Concordium Ledger app. `ledger setup <LABEL> --restore <NETWORK>` enrolls the Ledger key source and then immediately runs Ledger-backed recovery on the selected network. `ledger sync <LABEL>` recovers identities and accounts for an enrolled Ledger key source. Ledger recovery belongs to the `ledger` command space because it requires a connected matching Ledger device and explicit recovery-secret export approval. `ledger show` inspects the connected Concordium Ledger app and displays the app name plus app version when the app supports version reporting. `ledger remove <LABEL>` removes an enrolled Ledger key source from local wallet state after explicit confirmation, including locally stored Ledger-owned identities and accounts by cascade; it does not modify the physical Ledger device.

`identity new` can target either a seed-backed or Ledger-backed key source through `--seed <LABEL>` / `--key-source <LABEL>`. Ledger-backed identity issuance uses an explicit export security model and requires a Concordium Ledger app with purpose-based export support (version 5.5.0 or newer): interactive runs require confirmation, and non-interactive runs must include `--allow-ledger-secret-export` before the CLI exports identity issuance material temporarily from the Ledger app. This flow has been validated on a physical Ledger device running Concordium app `5.6.2`.

`transaction show <HASH>` inspects transaction lifecycle and outcome details through a resolved node. The optional `--show-payload` flag additionally attempts to display the original submitted block item payload and account transaction header when the transaction is present in committed or finalized block contents.

## Planned command spaces

The following command spaces are planned taxonomy targets. They are documented here so future implementation work stays consistent with the intended CLI structure.

### `stake` [Planned]

The staking area should be grouped by **validator** and **delegation** flows rather than as a flat collection of stake actions.

```text
stake
├─ validator
│  └─ ...modern validator configuration flows...
└─ delegation
   └─ ...delegation configuration flows...
```

#### Validator branch rules

- The validator branch is scoped to **modern `ConfigureBaker`-compatible behavior**.
- The taxonomy SHALL NOT introduce deprecated legacy baker transaction families such as pre-`ConfigureBaker` add/remove/update-style commands.
- Validator-oriented commands should be framed around current concerns such as stake capital, restake behavior, pool openness, metadata, commissions, keys, and suspension state.

#### Delegation branch rules

- Delegation-oriented commands should reflect modern `ConfigureDelegation` behavior.
- Delegation actions should stay separate from validator actions even when both live under the same top-level `stake` space.

## Deferred areas

Some future transaction authoring areas remain intentionally unresolved in this change.
In particular:
- this document does not define a dedicated top-level `ccd` command space
- this document does not finalize where miscellaneous non-token account transaction authoring will live
- this document does not define the exact future token-composition syntax beyond the implemented single-command token workflows

These areas can be added later, but they should not conflict with the implemented `token` taxonomy and planned `stake` taxonomy defined here.
