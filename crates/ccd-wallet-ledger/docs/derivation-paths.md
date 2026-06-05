# Derivation paths

Ledger commands that operate on account key material use `DerivationPath`.

## Raw paths

Use `DerivationPath::new` when the caller already knows the exact BIP32 indices expected by the Ledger app.

```rust
use ccd_wallet_ledger::DerivationPath;

let path = DerivationPath::new([
    44 + 0x8000_0000,
    919 + 0x8000_0000,
    0 + 0x8000_0000,
])?;
# ccd_wallet_ledger::Result::Ok(())
```

## String parsing

`DerivationPath` accepts paths with or without the leading `m/` and supports `'`, `h`, or `H` hardened markers.

```rust
use ccd_wallet_ledger::DerivationPath;

let path: DerivationPath = "m/44'/919'/0'/0'/0'".parse()?;
assert_eq!(path.to_string(), "m/44'/919'/0'/0'/0'");
# ccd_wallet_ledger::Result::Ok(())
```

## Concordium helper

`DerivationPath::concordium_account` constructs a hardened path of the form:

```text
m/44'/coin_type'/idp_index'/identity_index'/credential_index'
```

Use coin type `919` for mainnet and `1` for testnet-like networks.

```rust
use ccd_wallet_ledger::DerivationPath;

let mainnet_first_credential = DerivationPath::concordium_account(919, 0, 0, 0)?;
# ccd_wallet_ledger::Result::Ok(())
```

## Ledger byte encoding

The Ledger app path format is:

```text
component_count || component_0_be || component_1_be || ...
```

`DerivationPath::to_ledger_bytes` returns that encoding and is used internally by command builders.

## Why paths are crate-local

The crate keeps path handling local because different Concordium tooling has historically used slightly different path conventions. Higher-level code should decide which path convention is appropriate for its account model and pass the resulting `DerivationPath` into this crate.
