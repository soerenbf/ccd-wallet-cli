## ADDED Requirements

### Requirement: Canonical command taxonomy documents the CCD command space
The canonical command taxonomy SHALL document a top-level `ccd` command space for native CCD account-transaction authoring. The documented `ccd` branch SHALL include `transfer` for simple CCD transfer flows and `schedule` for scheduled CCD transfer flows.

#### Scenario: Contributor reviews CCD command grouping
- **WHEN** a contributor reads `docs/commands.md`
- **THEN** they can find a documented top-level `ccd` command space
- **AND** they can identify `ccd transfer` and `ccd schedule` as the initial CCD authoring commands

### Requirement: Canonical command taxonomy scopes CCD separately from transaction inspection
The canonical command taxonomy SHALL describe `ccd` as the user-facing home for native CCD transfer authoring while keeping transaction inspection under the `transaction` command space.

#### Scenario: Contributor reviews CCD and transaction roles
- **WHEN** a contributor reads the command taxonomy document
- **THEN** they can see that `transaction show` remains under `transaction`
- **AND** they can see that native CCD transfer authoring belongs under `ccd`
