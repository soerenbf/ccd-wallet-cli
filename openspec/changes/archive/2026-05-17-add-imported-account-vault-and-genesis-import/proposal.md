## Why

Local and private Concordium networks can be bootstrapped with genesis accounts that are not derived from a local seed and may exist without any identity provider. The wallet needs a way to import, protect, list, and eventually sign transactions with these orphan account bundles while preserving the same label and address-privacy expectations as derived accounts.

## What Changes

- Add an imported-account source model alongside seed-derived accounts so wallet-visible accounts can be backed by either derivation coordinates or imported secret material.
- Add an imported accounts vault scoped by network genesis hash, created implicitly on first import for that network and used to encrypt imported account secret material.
- Add a genesis account import flow for a single genesis account JSON file.
- Require imported accounts to have explicit labels; if omitted in interactive mode, prompt for one with the JSON filename stem as the suggested default/placeholder.
- Validate imported account labels with the normal account-label rules and reject labels already used by any account on the resolved network.
- Keep account addresses hidden by default for imported accounts and reveal them only through existing explicit reveal flows.
- Preserve a future-compatible internal imported-account secret representation so later browser-wallet export imports can map into the same account source model.

## Capabilities

### New Capabilities
- `imported-account-vault`: Imported account secret material is protected in a vault scoped by network genesis hash.
- `genesis-account-import`: A single genesis account JSON bundle can be imported as a wallet account with a user-provided label.
- `account-signing-source`: Wallet accounts expose a source-aware signing-material resolution path for both derived and imported accounts.

### Modified Capabilities
- `account-storage`: Account records distinguish derived and imported sources while preserving network-wide label uniqueness.
- `entity-listing`: Account list output includes imported accounts in normal account listings while preserving address privacy.
- `entity-rename`: Account rename continues to enforce label uniqueness across all account sources on a network.

## Impact

- SQLite schema and migrations for account source metadata, imported vault metadata, and imported account encrypted payloads.
- Core account store APIs for listing, importing, decrypting, and resolving account source data.
- CLI account command surface for genesis account import and source-aware account display/selection.
- Future transaction-signing code paths will resolve signing material through account source rather than assuming seed derivation.
- No directory import and no browser-wallet import UX are included in this initial change.
