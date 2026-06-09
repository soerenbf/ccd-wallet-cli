## MODIFIED Requirements

### Requirement: Ledger command taxonomy documents recovery flows
The canonical command taxonomy SHALL document Ledger enrollment, Ledger recovery, Ledger app inspection, and local Ledger key-source removal under the `ledger` command space. The documented implemented Ledger branch SHALL include `setup`, `sync`, `show`, and `remove`; Ledger setup documentation SHALL indicate that enrollment supports immediate recovery through `--restore <NETWORK>`, and Ledger removal documentation SHALL indicate that removal affects local wallet state rather than the physical Ledger device.

#### Scenario: Contributor reviews Ledger recovery command grouping
- **WHEN** a contributor reads the Ledger section of `docs/commands.md`
- **THEN** they can find `ledger setup`, `ledger sync`, `ledger show`, and `ledger remove` documented as implemented commands
- **AND** they can see that `ledger setup` supports immediate recovery through `--restore <NETWORK>`
- **AND** they can understand that Ledger recovery belongs to the `ledger` command space rather than the `seed` command space
- **AND** they can understand that `ledger remove` removes local Ledger key-source state and does not modify the physical Ledger device
