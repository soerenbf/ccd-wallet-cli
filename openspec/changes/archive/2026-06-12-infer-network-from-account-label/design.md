## Context

Many CLI flows follow this ordering:

```text
resolve network → resolve account label/address within that network → run command
```

That ordering is appropriate when the user explicitly supplies a network, when a raw account address is used, or when no account has been selected yet. It is less helpful for local account labels because the wallet already stores each account with a network genesis hash. If a label appears on only one configured network, asking the user to choose the network adds interaction without reducing ambiguity.

The goal is not to make account resolution always run before network resolution. The goal is to preserve explicit network filtering while allowing an explicit local account label to determine the network when no hard network constraint exists.

## Resolution Precedence

```text
1. Explicit network constraint
   ├─ --network NAME
   └─ compatible node override / node genesis hash

   The network is a hard filter. Account labels are looked up inside this
   scope. A label that exists elsewhere but not in this scope is an error.

2. No explicit network + explicit local account label
   ├─ if active network has an eligible matching account
   │     └─ use the active-network match
   ├─ else if exactly one eligible match exists globally
   │     └─ infer network from that account
   ├─ else if multiple eligible matches remain
   │     └─ prompt with an account selector
   └─ else
         └─ report missing/invalid account label

3. No explicit network + raw address, or no account supplied yet
   └─ use normal network resolution:
      active network → single configured network → prompt → error,
      according to the command's interactive/non-interactive rules
```

This makes the active network a soft default, not a hard constraint. It can filter account lists and select the active-network account when an explicit label is ambiguous, but it should not prevent an explicitly supplied unique account label from selecting its own network in interactive mode.

## Resolver Modes

Introduce or refactor toward two shared account-target resolver modes:

```text
account reference resolver
──────────────────────────
Used for read-only or non-signing account values such as recipients, token targets,
and contract invoke --invoker.

input account target
        │
        ├─ parses as raw account address
        │     └─ accept address; resolve network using existing network rules
        │
        └─ local label candidate
              └─ apply account-assisted network resolution when eligible

signing account / sender resolver
─────────────────────────────────
Used for transaction senders and options such as --sender, --account when it names
the signing account, stake configure/remove accounts, contract init/update senders,
and token submit senders.

input account target
        │
        ├─ parses as raw account address
        │     └─ reject; signing requires a local account with available signing material
        │
        └─ local label candidate
              └─ apply account-assisted network resolution when eligible
```

For commands with an explicit `--network`, resolution remains constrained to that network. A label that exists elsewhere but not on the selected network is an error for that selected network.

For commands with a node override, the node genesis hash remains authoritative. A local label may only infer/select a configured network whose genesis hash matches the node. If no configured account match is compatible with the node, the command fails with an actionable error.

## Context Headers

When a command does not prompt because a choice is obvious, the CLI should still tell the user what was selected:

```text
network:    local/p11-locks @ http://127.0.0.1:20000
account:    account-2
source:     imported account vault
```

or:

```text
network:    testnet @ grpc.testnet.concordium.com:20000
account:    account-0
key source: main-seed
```

Prompted choices can remain visually represented by the prompt itself, but silently inferred/defaulted choices should be logged through the existing resolved-context header mechanism.

## Non-interactive Determinism

Non-interactive commands must not infer a network solely because a local label is currently unique. That would make scripts change behavior when a new network/account is later added. Existing explicit/default network rules remain in force for non-interactive mode.

In particular, if non-interactive mode has an active network, that active network may be used only where existing command rules allow active defaults. A supplied account label that exists only on a different network must not override the active network in non-interactive mode.

## Contract Invoke

`contract invoke` is read-only and never signs. Its explicit invoker option (`--invoker`) should remain an account reference, not a signing account. A raw address is always eligible as the invoker value, and a local label is only a convenience for resolving an address in the selected/inferred network context.

## Scope Decisions

All applicable account-consuming command families should migrate to the shared behavior in this change, rather than limiting the first pass to stake commands. This includes account inspection/export, contract module/init/update sender selection, token `--sender` flows, token target/reference flows, and any command that already uses `resolve_export_account` after `resolve_account_network_context`, where the command's input semantics fit either the account-reference or signing-account/sender resolver mode.

Resolved context headers should show network, account label, and key-source/imported-source metadata, but they should not include account addresses. Keeping addresses out of the header avoids extra visual noise and avoids introducing address decryption solely for header rendering when a command does not otherwise need it at that point.
