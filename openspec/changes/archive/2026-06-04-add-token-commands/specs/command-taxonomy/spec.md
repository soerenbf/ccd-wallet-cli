## MODIFIED Requirements

### Requirement: Token operations are documented under the `token` command space
The canonical command taxonomy SHALL document protocol-level token inspection, protocol-level token transfers, token policy operations, token admin-role changes, token metadata updates, and protocol-level lock operations under the `token` command space, using nested grouping where needed instead of exposing `metaupdate` as a user-facing command path. The documented user-facing branch SHALL use `show` for token inspection, `transfer` for holder transfers, `admin-roles` for token admin-role operations, and `lock show` for lock inspection. For lock fund, send, and return, the token identifier SHALL be documented as `--token` rather than a positional argument.

#### Scenario: Contributor reviews token command grouping
- **WHEN** a contributor reads the token section of `docs/commands.md`
- **THEN** they can find token show, token transfer, metadata, admin-role, and lock operations grouped under `token`
- **AND** they can find `token lock show` documented alongside the lock mutation commands
- **AND** they can see that `token lock fund`, `token lock send`, and `token lock return` accept `--token` for the token identifier
- **AND** they do not see `metaupdate` documented as a required user-facing command namespace
