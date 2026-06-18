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
├─ ccd                          [Implemented]
│  ├─ transfer
│  └─ schedule
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
│  ├─ export
│  ├─ list
│  ├─ new
│  ├─ show
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
│  ├─ proposal
│  │  ├─ create
│  │  ├─ sign
│  │  └─ submit
│  └─ update
├─ stake                        [Implemented]
│  ├─ show
│  ├─ configure
│  │  ├─ delegation
│  │  └─ validator              [Reserved / not yet implemented]
│  └─ remove
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
│  ├─ lock
│  │  ├─ create
│  │  ├─ fund
│  │  ├─ send
│  │  ├─ return
│  │  ├─ cancel
│  │  └─ show
│  └─ compose
│     ├─ <PLAN>
│     ├─ preview <PLAN>
│     └─ submit <PLAN>
└─ connect                      [Implemented]
```

## Key-source terminology

The internal storage model calls seed-backed and Ledger-backed derivation authorities **signer owners**. User-facing CLI text should call the same concept a **key source** when a command or prompt needs to cover both seed phrases and Ledger devices.

The `seed` command family remains the user-facing place for seed phrase management. Ledger setup uses the separate `ledger setup` flow and enrolls a Ledger-backed key source by reading a canonical public key from the Concordium Ledger app. `ledger setup <LABEL> --restore <NETWORK>` enrolls the Ledger key source and then immediately runs Ledger-backed recovery on the selected network. `ledger sync <LABEL>` recovers identities and accounts for an enrolled Ledger key source. Ledger recovery belongs to the `ledger` command space because it requires a connected matching Ledger device and explicit recovery-secret export approval. `ledger show` inspects the connected Concordium Ledger app and displays the app name plus app version when the app supports version reporting. `ledger remove <LABEL>` removes an enrolled Ledger key source from local wallet state after explicit confirmation, including locally stored Ledger-owned identities and accounts by cascade; it does not modify the physical Ledger device.

`identity new` can target either a seed-backed or Ledger-backed key source through `--seed <LABEL>` / `--key-source <LABEL>`. Ledger-backed identity issuance uses an explicit export security model and requires a Concordium Ledger app with purpose-based export support (version 5.5.0 or newer): interactive runs require confirmation, and non-interactive runs must include `--allow-ledger-secret-export` before the CLI exports identity issuance material temporarily from the Ledger app. This flow has been validated on a physical Ledger device running Concordium app `5.6.2`.

`governance update` signs and submits on-chain governance updates in one invocation. By default it signs with selected keys from the local governance key vault. `governance update --ledger` instead signs with a connected device running the Concordium Governance Ledger app. Interactive runs render a parsed governance update review and ask for cliclack yes/no approval before local or Ledger signing and submission; non-interactive runs skip that approval prompt. Ledger governance signing is exclusive for a command invocation: it does not mix local governance key vault signatures with Ledger signatures, and it does not support blind signing of unknown serialized payloads. The Ledger signer path is derived from the update authorization family using governance purpose `0` for root, `1` for level 1, and `2` for level 2, with governance key index `0` by default; `--ledger-key-index <N>` selects a different key index. When interactive Ledger signer authorization validation fails, the CLI warns and asks whether to continue with a signature map index equal to the selected Ledger key index; this is intended for diagnostics and produces signatures that nodes may reject. The all-in-one Ledger submission flow currently supports one Ledger signer, so updates whose on-chain threshold is greater than one should use the detached `governance proposal` flow.

`governance proposal` is the detached multi-party governance update flow. `governance proposal create --json <FILE> --out <FILE> --effective-time <TIME> --timeout <TIME>` creates a proposal file containing the version, network genesis hash, frozen update header, and canonical pretty JSON payload. Proposal creation intentionally requires explicit effective time and timeout inputs so detached signers do not coordinate around accidental timing defaults. `governance proposal sign <PROPOSAL> --out <FILE>` signs a proposal with a local governance key using the same local signer selection behavior as `governance update`; `--ledger --ledger-key-index <N>` signs with a connected Governance Ledger device. Interactive detached signing renders the parsed proposal payload and asks for cliclack yes/no approval before writing a detached signature, which lets Ledger signers compare the app-parsed details with the device display. Detached signature files contain the version, verify key, and a single-entry `UpdateInstructionSignature`-shaped signature map keyed by the live on-chain governance key index for the signer. `governance proposal submit <PROPOSAL> --signature <FILE>...` submits the proposal after online revalidation; `--signature-dir <DIR>` additionally loads JSON signature files from a directory. Interactive detached submission renders the parsed update and accepted signature indices and asks for cliclack yes/no approval before node submission; non-interactive detached signing and submission skip approval prompts. All detached proposal stages resolve the selected network and revalidate current on-chain authorization state through the node.

`transaction show <HASH>` inspects transaction lifecycle and outcome details through a resolved node. The optional `--show-payload` flag additionally attempts to display the original submitted block item payload and account transaction header when the transaction is present in committed or finalized block contents.

`ccd` is the user-facing home for native CCD transfer authoring. `ccd transfer <SENDER> --recipient <ADDRESS_OR_LABEL> --amount <CCD>` submits a simple CCD transfer, and `ccd schedule <SENDER> --recipient <ADDRESS_OR_LABEL> --release <RFC3339=CCD>...` submits a scheduled CCD transfer. Both commands accept finalized local account labels for senders, accept raw account addresses or finalized local account labels for recipients, support optional memos, and keep transaction inspection under `transaction show` rather than under the `ccd` command space.

`token compose <PLAN>` opens an interactive token MetaUpdate composer backed by a saved TOML plan. The composer supports adding token and lock operations, previewing the plan, and submitting the saved composition. `token compose preview <PLAN>` lists the operations recorded in the plan without requiring sender or network context. `token compose submit <PLAN> --sender <LABEL>` resolves the saved plan and submits the ordered operations as a single MetaUpdate account transaction.

## Planned command spaces

The following command spaces are planned taxonomy targets. They are documented here so future implementation work stays consistent with the intended CLI structure.

### `stake` [Implemented]

The staking area is grouped under a top-level `stake` command space with generic inspection and removal actions plus a nested configuration area.

```text
stake
├─ show <ACCOUNT>
├─ configure
│  ├─ delegation <ACCOUNT>
│  └─ validator <ACCOUNT>       [Reserved / not yet implemented]
└─ remove <ACCOUNT>
```

#### Implemented stake commands

- `stake show <ACCOUNT>` queries live account staking state for either a finalized local account label or a raw account address.
- `stake configure delegation <ACCOUNT>` submits modern `ConfigureDelegation` transactions with patch-style updates for delegation target, capital, and restake behavior.
- `stake remove <ACCOUNT>` removes the currently configured staking mode, whether the account is delegating or validating.
- `stake configure validator <ACCOUNT>` is reserved as the validator-oriented configuration branch for future work and is not implemented yet.

#### Account and network resolution

- Interactive commands that consume local account labels treat `--network` and compatible node overrides as hard constraints.
- Without an explicit network, an interactive local account label can infer its network from the wallet when it resolves uniquely, and the active network is used as a soft preference for otherwise ambiguous labels.
- Ambiguous local account labels are resolved with an account selector that shows network plus key-source or imported-account metadata instead of a network-only selector.
- Transaction sender inputs such as `stake configure delegation <ACCOUNT>`, `stake remove <ACCOUNT>`, contract submitter `--account`, token mutation `--account`, and `token compose submit --sender` require finalized local account labels; raw account addresses are rejected for signing flows.
- Read-only account-reference inputs such as `stake show <ACCOUNT>` and `contract invoke --invoker` still accept raw account addresses.
- Non-interactive commands stay deterministic: they do not infer a network from current account-label uniqueness and they do not let a supplied account label override the active network.

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
- this document does not finalize where miscellaneous non-token account transaction authoring beyond CCD transfers will live
- this document does not define any future token-composition syntax beyond the implemented `token compose` plan workflow

These areas can be added later, but they should not conflict with the implemented `ccd` and `token` taxonomy and planned `stake` taxonomy defined here.
