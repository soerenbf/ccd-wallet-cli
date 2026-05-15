## Why

`ccd-wallet` can store seeds but cannot yet obtain a Concordium identity from an identity provider. Identity issuance is the prerequisite for creating accounts and transacting on-chain, so it is the next critical capability to add.

## What Changes

- Add `ccd-wallet identity new` command that initiates the Concordium v1 identity issuance protocol with a chosen identity provider.
- Two modes of operation:
  - **Non-interactive**: `identity new --provider <id>` — specify provider ID directly, optionally override seed and node, while the selected network provides the wallet proxy metadata.
  - **Interactive**: `identity new --interactive` — look up available identity providers on-chain and present an interactive selector.
- The command drives a browser-assisted flow: the CLI combines on-chain provider data from the selected node with wallet-facing IDP metadata from the selected network's `wallet_proxy`, builds the cryptographic issuance request, opens a URL in the system browser, and waits for the provider callback.
- Initial implementation uses **manual callback paste** (user copies the final redirect URL from the browser); a loopback HTTP callback receiver is out of scope for this change but the abstraction will accommodate it later.
- Issued identity objects are stored in the local SQLite database.
- Add `identity` top-level subcommand group.
- New dependency: `concordium-rust-sdk` (or equivalent) for `createIdentityRequest` cryptographic primitives and on-chain IP/AR metadata lookup.
- New dependency: wallet proxy HTTP metadata lookup for resolving issuance endpoints from the selected network's `wallet_proxy`.

## Capabilities

### New Capabilities

- `identity-issuance`: The end-to-end `identity new` command — argument parsing, mode selection, provider resolution, wallet-proxy metadata lookup, issuance request construction, browser handoff, callback receipt, polling, and identity storage.
- `identity-storage`: SQLite schema and CRUD for identity objects, their issuance state, and their association with a seed index and identity provider.
- `identity-provider-client`: HTTP client that implements the Concordium v1 identity issuance protocol: issuance start request, redirect handling, `code_uri` polling, and identity object deserialisation.

### Modified Capabilities

- `seed-command`: `identity new` requires access to the active (or specified) seed's cryptographic material (idCredSec, prfKey, blindingRandomness). No user-facing behaviour changes; the seed unlock path already exists.
- `config-storage`: Network entries must now persist `wallet_proxy` in addition to `node_endpoint` and `genesis_hash`.
- `network-config-add`: `network add` must accept and persist `--wallet-proxy <URL>`.

## Impact

- New: `src/commands/identity/mod.rs`, `src/commands/identity/new.rs`
- New: `src/store/identities.rs`
- New: `src/identity_provider/client.rs`
- Modified: `src/cli.rs` — add `identity` subcommand group
- Modified: `src/commands/mod.rs` — wire identity module
- Modified: `src/store/migrations/` — add migration for identity tables
- New dependency: Concordium SDK crate(s) for HD wallet derivation and identity request construction
- New dependency: `open` crate for launching the system browser
