# Governance update JSON templates

This directory contains editable JSON templates for Concordium governance update payloads.

The files are compatible with commands that accept a governance update JSON payload:

```sh
ccd-wallet governance update \
  --json examples/governance-updates/protocol.json \
  --network testnet

ccd-wallet governance proposal create \
  --json examples/governance-updates/protocol.json \
  --out protocol-proposal.json \
  --network testnet \
  --effective-time 2026-07-01T00:00:00Z \
  --timeout 2026-06-30T00:00:00Z
```

These templates only contain the update payload. Header values such as sequence number, effective time, timeout, and signatures are resolved or supplied by the command.

Review and replace every placeholder value before signing or proposing an update. The example values are not recommendations for production chain parameters.

## Templates

- `protocol.json` — protocol announcement update.
- `create-plt.json` — create a protocol-level token. This template uses the wallet convenience field `initializationParametersJson`; the CLI converts it to CBOR hex before parsing the payload.
- `timeout-parameters-cpv2.json` — consensus timeout parameters.
- `min-block-time-cpv2.json` — minimum block time.
- `block-energy-limit-cpv2.json` — block energy limit.
- `finalization-committee-parameters-cpv2.json` — finalization committee parameters.
- `validator-score-parameters-cpv3.json` — validator score parameters.
