## Context

The previous change introduced SQLite-backed seed storage and encryption primitives, but those APIs are not yet reachable from the CLI. The user needs a safe command for adding a seed phrase to the encrypted DB.

The CLI currently nests network management under `ccd-wallet config network ...`. As the wallet grows, `network`, `seed`, `account`, and later transaction commands should read as first-class wallet concepts rather than config internals.

## Goals / Non-Goals

**Goals:**
- Add `ccd-wallet seed add <LABEL>`.
- Prompt interactively for the seed phrase and password; never accept either as a CLI argument.
- Validate the seed phrase before encrypting/storing it.
- Normalize seed phrase whitespace before validation/storage.
- Use the existing `store::seeds::add` path for encrypted persistence.
- Add top-level `ccd-wallet network add` and `ccd-wallet network use` routes.
- Remove the `ccd-wallet config network ...` route.

**Non-Goals:**
- Generating new seed phrases.
- Listing, renaming, deleting, exporting, or unlocking seeds via CLI.
- Deriving accounts from seeds.
- Changing the `config.json` schema.
- Changing the seed encryption format or DB schema.

## Decisions

### D1: Use `seed add <LABEL>` with a positional label

**Decision**: The new command is `ccd-wallet seed add <LABEL>`.

**Rationale**: Labels are first-class user handles for seeds, similar to network names. A positional label keeps common usage terse:

```bash
ccd-wallet seed add main_seed
```

**Alternative considered**: `ccd-wallet seed add --label main_seed`. Rejected as more verbose without much clarity for the primary argument.

**Alternative considered**: `ccd-wallet seed import`. More precise, but `add` is simpler and leaves room for future `seed generate`.

### D2: Prompt sensitive values interactively and hidden

**Decision**: The seed phrase, password, and password confirmation are read interactively from the terminal with hidden input.

**Rationale**: Passing secrets via command-line arguments exposes them through shell history, terminal scrollback, and process inspection. Hidden prompts reduce accidental disclosure.

Implementation should use a small prompt abstraction so tests can provide deterministic input without relying on a real terminal.

### D3: Validate mnemonic using BIP39 validation

**Decision**: Validate the entered phrase as a BIP39 mnemonic before storage. Normalize input by trimming leading/trailing whitespace and collapsing internal whitespace to a single ASCII space before validation.

**Rationale**: Concordium wallet SDK docs use BIP39-style seed phrases. Early validation prevents storing unusable encrypted secrets that only fail later during account/identity derivation.

**Alternative considered**: Store arbitrary bytes now and validate during account derivation later. Rejected because it gives users false confidence and makes failures appear much later.

### D4: Store the normalized phrase as the encrypted seed payload

**Decision**: The seed payload stored through `store::seeds::add` is the normalized mnemonic phrase bytes.

**Rationale**: This keeps the first CLI change aligned with the existing storage API. Future derivation code can parse the normalized phrase from the decrypted payload.

### D5: Password confirmation required

**Decision**: `seed add` prompts for password and confirmation and rejects mismatches before writing anything.

**Rationale**: The password is needed to recover the seed from encrypted storage. A typo at import time would make the stored seed effectively inaccessible.

### D6: Promote `network` to a top-level command group

**Decision**: Move network management from `ccd-wallet config network ...` to `ccd-wallet network ...` and remove the old path.

**Rationale**: No users rely on the old route yet, so there is no compatibility cost. Top-level `network` is clearer alongside `seed` and future `account` commands.

```
Before:
  ccd-wallet config network add --name testnet --node ...
  ccd-wallet config network use testnet

After:
  ccd-wallet network add --name testnet --node ...
  ccd-wallet network use testnet
```

## Risks / Trade-offs

- **Mnemonic standard mismatch**: If Concordium accepts a narrower/different seed format than standard BIP39, validation could accept phrases that later fail derivation. Mitigation: use a widely adopted Rust `bip39` crate now; revisit when implementing derivation if Concordium-specific constraints emerge.
- **Hidden seed prompt hurts paste visibility**: Users cannot visually confirm pasted words. Mitigation: validate and report invalid mnemonic errors clearly; do not echo secrets.
- **Removing `config network` is breaking**: No external users yet, so the break is intentional. Mitigation: update README/examples and tests in the same change.
- **Prompt testing can be brittle**: Mitigation: route prompts through a trait/function abstraction so command logic can be tested with injected inputs.

## Migration Plan

No data migration is needed. The `config.json` schema remains unchanged. Existing local development commands must switch from `ccd-wallet config network ...` to `ccd-wallet network ...`.

### D7: Restrict seed labels to CLI-safe characters

**Decision**: Seed labels must be non-empty and contain only ASCII alphanumeric characters, dash (`-`), and underscore (`_`). Whitespace is not allowed.

**Rationale**: Labels are intended to be used frequently in commands. Restricting them to shell-friendly identifiers avoids quoting surprises and encourages `main_seed` / `cold-wallet` style names instead of whitespace-separated labels.

## Open Questions

None.
