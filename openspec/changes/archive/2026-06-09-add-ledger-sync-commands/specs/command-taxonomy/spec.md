## ADDED Requirements

### Requirement: Ledger command taxonomy documents recovery flows
The canonical command taxonomy SHALL document Ledger enrollment, Ledger recovery, and Ledger app inspection under the `ledger` command space. The documented implemented Ledger branch SHALL include `setup`, `sync`, and `show`, and Ledger setup documentation SHALL indicate that enrollment supports immediate recovery through `--restore <NETWORK>`.

#### Scenario: Contributor reviews Ledger recovery command grouping
- **WHEN** a contributor reads the Ledger section of `docs/commands.md`
- **THEN** they can find `ledger setup`, `ledger sync`, and `ledger show` documented as implemented commands
- **AND** they can see that `ledger setup` supports immediate recovery through `--restore <NETWORK>`
- **AND** they can understand that Ledger recovery belongs to the `ledger` command space rather than the `seed` command space
