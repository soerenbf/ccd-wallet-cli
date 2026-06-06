# Concordium Ledger app 5.4.1 identity-export analysis

## Source checkout

- Repository: `/Users/sorenbz/Developer/Concordium/app-concordium`
- Checked-out tag: `flex_1.6.0_5.4.1_sdk_v26.0.2`
- Device-observed app: Concordium `5.4.1`

This analysis treats the checked-out tag as authoritative for the app installed on the physical Ledger device.

## Dispatcher

`src/common/handler.c` routes the relevant instructions as follows:

```c
case INS_EXPORT_PRIVATE_KEY_LEGACY:
    handleExportPrivateKey(cdata, p1, p2, lc, true, flags);
    break;
case INS_EXPORT_PRIVATE_KEY_NEW:
    handleExportPrivateKey(cdata, p1, p2, lc, false, flags);
    break;
case INS_APP_VERSION:
    ... handler_get_version();
    break;
```

`src/common/handler.h` defines:

```c
#define INS_EXPORT_PRIVATE_KEY_LEGACY 0x05
#define INS_EXPORT_PRIVATE_KEY_NEW    0x37
#define INS_APP_VERSION               0x40
```

So app 5.4.1 does expose `INS=0x37`, but this tag routes it to the same legacy-style export handler with `legacyDerivationPath=false`.

## Export command shape in app 5.4.1

`doc/export_private_key.md` documents exactly two exportable key types:

1. PRF-key
2. IdCredSec

It documents no signature-blinding-randomness export.

For new derivation paths, the documented APDUs are:

| INS | P1 | P2 | CDATA | Meaning |
| --- | --- | --- | --- | --- |
| `0x37` | `0x00` | `0x01` | `idp[uint32] || identity[uint32]` | PRF seed |
| `0x37` | `0x01` | `0x01` | `idp[uint32] || identity[uint32]` | PRF seed, recovery display |
| `0x37` | `0x02` | `0x01` | `idp[uint32] || identity[uint32]` | PRF seed + IdCredSec seed |
| `0x37` | `0x00` | `0x02` | `idp[uint32] || identity[uint32]` | PRF BLS key |
| `0x37` | `0x01` | `0x02` | `idp[uint32] || identity[uint32]` | PRF BLS key, recovery display |
| `0x37` | `0x02` | `0x02` | `idp[uint32] || identity[uint32]` | PRF BLS key + IdCredSec BLS key |

`src/exportPrivateKey.c` implements the same constants:

```c
#define P1_PRF_KEY          0x00
#define P1_PRF_KEY_RECOVERY 0x01
#define P1_BOTH             0x02

#define P2_SEED 0x01
#define P2_KEY  0x02
```

Parameter validation rejects all other `P1`/`P2` combinations:

```c
if ((p1 != P1_BOTH && p1 != P1_PRF_KEY && p1 != P1_PRF_KEY_RECOVERY) ||
    (p2 != P2_KEY && p2 != P2_SEED)) {
    THROW(ERROR_INVALID_PARAM);
}
```

## New-path derivation in app 5.4.1

When `INS=0x37`, `handler.c` calls:

```c
handleExportPrivateKey(cdata, p1, p2, lc, false, flags);
```

Inside `handleExportPrivateKey`, `ctx->isNewPath = !legacyDerivationPath`, so `INS=0x37` uses the new derivation prefix:

```c
keyDerivationPath = (uint32_t[4]){
    NEW_PURPOSE | HARDENED_OFFSET,
    NEW_COIN_TYPE | HARDENED_OFFSET,
    identity_provider | HARDENED_OFFSET,
    identity | HARDENED_OFFSET
};
```

`src/globals.h` defines only these new export subpaths:

```c
NEW_ID_CRED_SEC = 2,
NEW_PRF_KEY = 3
```

No `NEW_SIGNATURE_BLINDING_RANDOMNESS` or purpose-specific export subpaths exist in this tag.

## Response format

For BLS output (`P2=0x02`), `exportPrivateKeyBls` returns raw concatenated 32-byte BLS scalars:

- PRF-only (`P1=0x00` or `P1=0x01`): `32` bytes
- PRF + IdCredSec (`P1=0x02`): `64` bytes (`PRFKey || IDCredSec`)

For seed output (`P2=0x01`), `exportPrivateKeySeed` returns raw concatenated 32-byte ed25519 seed bytes in the same key order.

There is no `[length][key]` framing in this tag.

## UI wording explains observed device screens

`handleExportPrivateKey` sets review text by `P1`:

- `P1=0x00`: `to decrypt credentials`
- `P1=0x01`: `to recover credentials`
- `P1=0x02`: `to create credentials`

The observed `review operation to decrypt credentials` screen for `P1=0x00` is therefore expected for this tag. The observed `IDP#0 ID#0` screen is also expected because `INS=0x37` new-path mode displays both identity provider and identity index.

## Test client confirmation

`tests/application_client/boilerplate_command_sender.py` sends exports with:

```python
ins = InsType.EXPORT_PRIVATE_KEY_NEW  # when idp_index is supplied
p1 = P1.P1_EXPORT_PRIVATE_KEY              # 0x00, standard PRF
p1 = P1.P1_EXPORT_WITH_ALTERNATIVE_DISPLAY # 0x01, recovery wording
p1 = P1.P1_EXPORT_PRFKEY_AND_IDCREDSEC     # 0x02, PRF + IDCredSec
p2 = P2.P2_EXPORT_BLS_KEY                  # 0x02
```

The new-path test asserts a `32`-byte response for `P1=0x00, P2=0x02`.

## Identity issuance implication

Concordium identity issuance needs three values for a recoverable Ledger-backed identity model:

1. `IDCredSec`
2. `PRFKey`
3. signature blinding/retrieval randomness

The installed app tag can export the first two via:

```text
CLA  = 0xE0
INS  = 0x37
P1   = 0x02
P2   = 0x02
DATA = identity_provider[uint32 big-endian] || identity[uint32 big-endian]
RESP = PRFKey[32] || IDCredSec[32]
```

It cannot export deterministic signature blinding randomness. Generating that randomness locally would let request construction proceed, but would make recovery depend on local encrypted state instead of the Ledger. Therefore `identity new` for Ledger key sources must remain fail-safe until either:

- a newer Ledger app installed on the device exposes deterministic signature blinding randomness, or
- a formally specified deterministic derivation for signature blinding randomness from existing Ledger-exportable material is accepted by the wallet's recovery model.

## When signature blinding randomness export appears

Repository history shows that deterministic signature blinding randomness export is not present in app 5.4.1. It appears in commit:

```text
41a5e81e577e641355e3d6234a0c5ad5bd8c8987
feat: implement exporting private keys for new path
AuthorDate: 2025-08-27 18:07:27 +0200
```

The first release tags found to contain `NEW_SIGNATURE_BLINDING_RANDOMNESS` are app version `5.5.0`, for example:

```text
nanos+_1.5.1_5.5.0_sdk_e8100ee31eeece2be02c3a2da39662fa3e7b72c0
nanox_2.6.1_5.5.0_sdk_e8100ee31eeece2be02c3a2da39662fa3e7b72c0
stax_1.9.1_5.5.0_sdk_e8100ee31eeece2be02c3a2da39662fa3e7b72c0
flex_1.5.1_5.5.0_sdk_e8100ee31eeece2be02c3a2da39662fa3e7b72c0
apex_p_1.0.5_5.5.0_sdk_e8100ee31eeece2be02c3a2da39662fa3e7b72c0
```

In app `5.5.0`, `INS=0x37` is purpose-based:

```text
P1=0x00 identity credential creation:
  [32] IDCredSec || [32] PRFKey || [32] Signature Blinding Randomness
P1=0x02 identity recovery:
  [32] IDCredSec || [32] Signature Blinding Randomness
P2=0x00 mainnet coin type 919'
P2=0x01 testnet coin type 1'
DATA=idp[uint32] || identity[uint32]
```

The app `5.5.0` source also changes the response framing to repeated `[length=32][key]` fields.

## Required `ccd-wallet-ledger` model

The low-level crate should model app 5.4.1's `INS=0x37` as a new-path legacy export protocol:

- `ExportPrivateKeyNewPathLegacyMode::PrfKey` -> `P1=0x00`
- `ExportPrivateKeyNewPathLegacyMode::PrfKeyRecovery` -> `P1=0x01`
- `ExportPrivateKeyNewPathLegacyMode::PrfKeyAndIdCredSec` -> `P1=0x02`
- `ExportPrivateKeyNewPathLegacyOutput::Seed` -> `P2=0x01`
- `ExportPrivateKeyNewPathLegacyOutput::BlsKey` -> `P2=0x02`
- payload: `idp || identity`
- response: raw `32` or `64` bytes depending on mode

The purpose-based protocol inspected in later source branches must remain separate and must not be advertised as app 5.4.1 behavior.

## Real-device validation after implementation

After this change was implemented, a physical Ledger device running Concordium app `5.6.2` was used to validate the purpose-based identity issuance flow end-to-end. `identity new` with a Ledger-backed key source completed successfully on the physical device.

This confirms that the implemented app `5.5.0+` assumption is correct in practice for app `5.6.2`:

```text
INS=0x37
P1=0x00  // IdentityCredentialCreation
P2=0x00 mainnet or 0x01 testnet
DATA=idp[uint32] || identity[uint32]
RESP=[32]IDCredSec || [32]PRFKey || [32]SignatureBlindingRandomness
```

It also confirms that Ledger-backed `identity new` can be stored under the correct signer owner when the connected Ledger canonical public key matches the selected enrolled Ledger signer owner.
