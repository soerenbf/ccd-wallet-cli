# Fixed-shape governance updates

Fixed-shape update commands send the derivation path, 28-byte update header, update type, and update-family-specific payload in a single APDU packet. The crate represents these with `FixedUpdateRequest` plus public aliases for individual update families.

Supported fixed-shape method names include:

- `sign_exchange_rate`
- `sign_transaction_fee_distribution`
- `sign_gas_rewards`
- `sign_foundation_account`
- `sign_mint_distribution`
- `sign_baker_stake_threshold`
- `sign_cooldown_parameters`
- `sign_pool_parameters`
- `sign_time_parameters`
- `sign_timeout_parameters`
- `sign_min_block_time`
- `sign_block_energy_limit`
- `sign_finalization_committee_parameters`
- `sign_validator_score_parameters`

The request payload is intentionally serialized bytes. Higher-level code remains responsible for deriving those bytes from Concordium governance domain values.
