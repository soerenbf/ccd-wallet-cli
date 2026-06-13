## 1. Shared Resolution Behavior

- [x] 1.1 Add shared helpers to find eligible local account-label matches across configured networks and map account genesis hashes back to configured network names.
- [x] 1.2 Add an interactive account-reference resolution path for raw-address-or-label inputs that respects explicit network constraints before considering account-assisted network inference.
- [x] 1.3 Add an interactive signing-account/sender resolution path for local-account-only inputs, including explicit `--sender` and sender aliases such as `--account` where applicable.
- [x] 1.4 Ensure signing-account/sender resolution rejects raw account addresses with an actionable error because signing keys are required.
- [x] 1.5 Treat active network as a soft default: prefer an active-network account match when an explicit label is ambiguous, but allow a unique non-active-network label to infer its network in interactive mode.
- [x] 1.6 Add ambiguous-label handling that opens an account selector with network and ownership metadata when prompting is available and no active-network preference resolves the ambiguity.
- [x] 1.7 Ensure explicit `--network` and node override inputs constrain or validate account-label resolution instead of being bypassed.
- [x] 1.8 Ensure non-interactive mode does not infer network from account-label uniqueness, does not let labels override an active network, and continues to fail or use existing explicit/default network rules.

## 2. Context Header and Selector UX

- [x] 2.1 Make single configured network selection in interactive mode skip the selector while still emitting a visible resolved network context header.
- [x] 2.2 Standardize account context header lines for local derived, Ledger-derived, and imported accounts without including account addresses.
- [x] 2.3 Ensure account disambiguation selector rows include local label, network, and key-source/imported-source metadata.
- [x] 2.4 Ensure account selectors use the active network as an initial/preferred choice where applicable without hiding other eligible networks when no explicit network was supplied.

## 3. Command Integration

- [x] 3.1 Apply account-assisted network resolution to `stake show <ACCOUNT>`.
- [x] 3.2 Apply compatible resolution to stake mutation commands that take or select a local signing account.
- [x] 3.3 Audit account, contract, and token command flows that select accounts after network resolution and migrate all applicable flows to the shared behavior.
- [x] 3.4 Ensure transaction sender options, explicitly including `--sender` and sender-account aliases, use signing-account/sender resolution and never accept raw addresses as signers.
- [x] 3.5 Ensure `contract invoke --invoker` remains an account-reference input that accepts raw addresses and never requires signing keys.
- [x] 3.6 Keep raw-address command behavior compatible with existing network/node resolution semantics for account-reference inputs.

## 4. Tests and Documentation

- [x] 4.1 Add tests that explicit `--network` constrains account lookup even when a matching label exists on another network.
- [x] 4.2 Add tests that active network selects the active-network match for an otherwise ambiguous account label.
- [x] 4.3 Add tests that interactive unique account-label network inference still works when the unique match is not on the active network.
- [x] 4.4 Add tests for ambiguous account labels prompting with account-level disambiguation when active network does not resolve the ambiguity.
- [x] 4.5 Add tests that non-interactive mode does not infer network from label uniqueness or let a label override the active network.
- [x] 4.6 Add tests that a single configured network is selected without a prompt and still appears in the resolved context header.
- [x] 4.7 Add or update command documentation for account/network resolution UX where applicable.
- [x] 4.8 Run Rust formatting and relevant Cargo tests.
