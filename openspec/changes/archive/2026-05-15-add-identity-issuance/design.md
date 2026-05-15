## Context

`ccd-wallet` is a CLI tool backed by an encrypted SQLite store. Seeds are stored and unlockable per-session. The next major user-facing capability is obtaining a Concordium identity from an identity provider (IP), which is a prerequisite for later account creation and transacting.

Concordium's v1 identity issuance protocol is browser-assisted: a wallet constructs a cryptographic `PreIdentityObject`, sends it to an IP endpoint, receives a redirect to a browser-based KYC UI, and later retrieves a signed `IdentityObject` by polling a `code_uri`. The redirect mechanism uses URL fragments (`#code_uri=...`) that are browser-local, which shapes the callback strategy.

The browser wallet (Concordium's reference implementation) uses a Chrome extension `webRequest` listener to observe the fragment when the browser navigates to the sentinel redirect URI `ConcordiumRedirectToken`. A CLI cannot observe browser navigation, so a different strategy is needed.

## Goals / Non-Goals

**Goals:**
- Implement `ccd-wallet identity new` driven by a chosen identity provider.
- Support two selection modes: `--provider <id>` (non-interactive) and `--interactive` (on-chain IP list picker).
- Allow overriding seed (`--seed`) and node (`--node`), while always resolving wallet-facing provider metadata from the selected network's `wallet_proxy`.
- Allow selecting a network with `--network <label>` or defaulting to the active network.
- Implement **manual callback paste** as the callback receiver: CLI prints the browser URL, user completes KYC, pastes final redirect URL back into the terminal.
- Design a `CallbackReceiver` abstraction so a future loopback HTTP receiver can be swapped in without changing the rest of the flow.
- Store issued identity objects in SQLite with their issuance state.

**Non-Goals:**
- Loopback HTTP callback server (deferred; the abstraction must accommodate it).
- Identity recovery flow.
- Account creation from an identity (separate change).
- Initial-account provisioning flows from older issuance models.
- Browser auto-open as a hard requirement (print URL as fallback).
- Offline / headless identity issuance.

## Decisions

### 1. Manual paste callback receiver as MVP

The browser wallet solves the URL fragment problem via `chrome.webRequest`. A CLI alternative would be a localhost HTTP server + JS fragment bridge page, but that adds meaningful complexity (random port, HTTP server, served HTML, cross-platform browser-open reliability).

For the initial change, a manual paste receiver is sufficient:

```
CLI prints browser URL
User completes KYC
CLI prompts: "Paste the final callback URL:"
CLI parses redirect_uri#code_uri=<url> from pasted text
```

This is robust, testable, requires no OS integration, and works in SSH/remote environments.

**Abstraction for future loopback:**

```
trait CallbackReceiver {
    fn receive(&self, redirect_uri: &str) -> Result<String>;  // returns code_uri
}

struct ManualPasteReceiver;       // MVP
struct LoopbackHttpReceiver;      // future
```

The rest of the issuance flow is receiver-agnostic.

### 2. `redirect_uri` value

The browser wallet uses the sentinel string `ConcordiumRedirectToken`. This tells us identity providers accept arbitrary strings, not validated URIs.

For the manual receiver, we use the same sentinel or a similar one (`ccd-wallet-cli`) so the final URL containing `#code_uri=` is easy to copy from the browser address bar or error page.

For the future loopback receiver, `redirect_uri` would be `http://127.0.0.1:<port>/callback`.

### 3. Wallet proxy metadata + issuance start

The on-chain `IpInfo` is not sufficient to derive the wallet-facing issuance endpoint. The CLI must resolve additional IDP metadata from the selected network's `wallet_proxy` service (for the v1 flow, `/v1/ip_info`) and join it with the on-chain provider identity.

Resolution model:

1. If `--node` is provided, connect to that node and query its `genesis_block`.
2. If both `--network` and `--node` are provided, validate that the node's genesis hash matches the selected network config.
3. If `--node` is provided without `--network`, select the configured network whose `genesis_hash` matches the node.
4. If `--node` is not provided, use `--network <label>` or the active network.
5. Read `wallet_proxy` from the resolved network entry in config.
6. Use the selected or overridden node endpoint for on-chain data (cryptographic parameters, IPs, ARs).
7. Use the wallet proxy metadata to resolve the issuance start URL for the chosen provider.
8. HTTP `GET` the issuance start URL (including query params with the identity request) as a preflight step.
9. If the response is a redirect, open the final redirect URL in the browser.
10. If the response is not a redirect, open the original issuance URL in the browser and let the browser drive the flow.

This keeps node override operational while treating the wallet proxy as part of the network definition, and avoids failing early when the provider exposes a browser entrypoint rather than an immediate HTTP redirect.

### 4. Key derivation: self-implemented SLIP-0010, concordium_base for PIO construction

The derivation chain is:

```
mnemonic → 64-byte seed (BIP39 PBKDF2)     bip39 crate, already present
         → 32-byte intermediate key (SLIP-0010 HD)  ~50 lines, self-implemented
         → BLS12-381 Fr scalar (keygen_bls)          ~30 lines, self-implemented
         → PreIdentityObject (generate_pio_v1)       concordium_base via SDK
```

The `key_derivation` and `wallet_library` local crates are **not used**. `concordium_base` at v10.0.0 is published on crates.io and already available transitively through `concordium-rust-sdk`. `generate_pio_v1` is accessible as `concordium_rust_sdk::id::account_holder::generate_pio_v1`. No local path dependencies are required.

The SLIP-0010 HD derivation is pure HMAC-SHA512 with hardened paths only (all Concordium paths are hardened). `keygen_bls` is HKDF-SHA256 over the 32-byte derived key, reducing the result into `Fr` (BLS12-381 scalar field). Both are small, well-specified, and verifiable against published Concordium test vectors.

IP and AR metadata are fetched from the chain node (gRPC). For `--provider <id>`, the CLI looks up the IP by numeric identity from the on-chain list. Wallet-facing issuance endpoints are fetched from the selected network's `wallet_proxy` HTTP service.

### 5. Identity index assignment

Each identity is associated with a (seed, IP, identity index) triple. The identity index is assigned by the CLI: use `0` for the first identity with a given IP, incrementing for subsequent ones. This matches the HD wallet spec.

### 6. Identity storage schema

New tables:

```sql
CREATE TABLE identities (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    seed_label      TEXT    NOT NULL,
    ip_identity     INTEGER NOT NULL,
    identity_index  INTEGER NOT NULL,
    label           TEXT    NOT NULL,
    status          TEXT    NOT NULL CHECK(status IN ('pending','done','error')),
    code_uri        TEXT,
    identity_object TEXT,   -- JSON, present when status=done
    created_at      INTEGER NOT NULL,
    UNIQUE(seed_label, ip_identity, identity_index)
);
```

Status transitions: `pending` → `done` | `error`.

### 7. Network config gains `wallet_proxy`

Network entries gain a required `wallet_proxy` field. This is the HTTP base URL used to resolve wallet-facing identity provider metadata.

This changes the semantics of identity issuance option resolution:

- `--network <label>` selects the network config and therefore the `wallet_proxy`
- `--node <url>` supplies the node used for chain queries and also determines the network by `genesis_hash`
- `--network` and `--node` are **not** mutually exclusive for identity issuance; when both are supplied they must match by `genesis_hash`
- if `--node` is supplied without `--network`, the CLI infers the configured network by matching the node's `genesis_hash`
- if neither `--network` nor an active network exists (and no node can be used to infer one), identity issuance errors before any browser/network flow

### 8. Interactive mode provider picker

In `--interactive` mode, query the active node for the IP list and present an arrow-key selection prompt instead of a numeric input. Each option shows both the provider name and the provider identity so the user can see the on-chain identifier without having to type it.

This is implemented with `cliclack::select`, which provides a human-friendly terminal selection flow and avoids ambiguity between the visible list position and the provider id.

### 9. `cliclack` as the interactive prompt layer

This change standardises new interactive identity issuance UX on `cliclack` rather than ad hoc `println!` + `stdin` prompts or introducing a separate prompt crate just for selection. `cliclack` is used for:

- arrow-key provider selection in `--interactive`
- password / input flows where appropriate in the CLI crate
- styled step / info / success messages during long-running issuance steps

The goal is to establish one coherent interaction layer for future wallet flows (identity issuance, callback entry, confirmations, etc.) while keeping non-interactive flag-driven usage intact.

### 10. Cargo workspace structure

This change introduces `ccd-wallet-core` (library crate) and restructures the existing single-crate binary into a workspace. The split is made now because the identity code — key derivation, IP client, `CallbackReceiver`, identity storage types — belongs in the library, not the CLI binary. The future loopback callback server will also be a separate crate (`ccd-wallet-callback`) that shares types with `ccd-wallet-core` without pulling a full HTTP server stack into every binary build.

```
ccd-wallet-cli/                   workspace root
├── Cargo.toml                    [workspace] members
└── crates/
    ├── ccd-wallet-core/          library: store, crypto, identity, types
    ├── ccd-wallet/               binary: CLI, clap, commands
    └── ccd-wallet-callback/      (future) loopback HTTP callback server
```

The `ccd-wallet` binary crate depends on `ccd-wallet-core` as a path dependency. The CLI command layer stays thin: parse args, delegate to core.

## Risks / Trade-offs

- **Manual paste UX is clunky** → Acknowledged; it is intentionally the MVP. The loopback receiver is the path forward and the abstraction is designed for it.
- **SLIP-0010 / keygen_bls correctness** → Self-implemented crypto must be validated against the published Concordium key derivation test vectors before any real issuance is attempted. Test vectors are available in the `key_derivation` crate test suite.
- **Wallet proxy dependency** → Identity issuance now depends on both node data and wallet proxy metadata. If the configured `wallet_proxy` is wrong or unavailable, issuance cannot start. Mitigated by treating `wallet_proxy` as explicit network configuration rather than guessing it.
- **Node/network mismatch** → A user can point `--node` at a chain that does not match the selected or active network. Mitigated by resolving or validating the network via `genesis_hash` before using `wallet_proxy`.
- **Provider `redirect_uri` policy** → If a real provider does not accept the sentinel string or a loopback URI, we may need to adapt. The browser wallet uses a plain sentinel string; consistent policy is assumed.
- **Fragment parsing on different browsers/OS** → Manual paste relies on the user seeing the fragment in the address bar. If the browser hides it or the provider renders a native-app-redirect page, the URL may not be visible. Mitigated by instructing the user clearly.
- **Polling timeout and cancellation** → Long-running `code_uri` polls need a timeout and clean cancellation (Ctrl-C). Use a configurable timeout with a default (e.g. 5 minutes) and handle SIGINT.
- **Prompt crate coupling** → Using `cliclack` improves UX consistency, but it is another opinionated dependency. Mitigated by keeping explicit flag-driven modes (`--provider`) available for scripting and by limiting `cliclack` usage to the CLI crate where practical.

## Open Questions

None.
