//! Shared contract initialization preparation, simulation, submission, and finalization helpers.

use anyhow::{Context, Result};
use concordium_rust_sdk::{
    common::types::{AccountAddress, Amount, TransactionTime},
    contract_client::ContractInitBuilder,
    smart_contracts::common::{ModuleReference, OwnedContractName, OwnedParameter},
    types::{
        BlockItemSummary, Energy,
        hashes::{BlockHash, TransactionHash},
        transactions::{InitContractPayload, send},
    },
    v2,
};

/// Prepared contract initialization transaction data.
#[derive(Clone)]
pub(crate) struct PreparedContractInit {
    /// Account expected to submit the transaction.
    pub(crate) sender: AccountAddress,
    /// Init payload.
    pub(crate) payload: InitContractPayload,
    /// Contract name without the `init_` prefix.
    pub(crate) contract_name: String,
    /// Maximum execution energy.
    pub(crate) energy: Energy,
}

/// Result of simulating a contract initialization.
pub(crate) struct ContractInitSimulation {
    /// Human-readable simulation message.
    pub(crate) message: String,
    /// Estimated energy when simulation succeeded.
    pub(crate) estimated_energy: Option<Energy>,
}

/// Submitted contract initialization metadata.
pub(crate) struct SubmittedContractInit {
    /// Submitted transaction hash.
    pub(crate) transaction_hash: TransactionHash,
}

/// Finalized contract initialization metadata.
pub(crate) struct FinalizedContractInit {
    /// Submitted transaction hash.
    pub(crate) transaction_hash: TransactionHash,
    /// Finalized block hash.
    pub(crate) block_hash: BlockHash,
    /// Finalized block item summary.
    pub(crate) summary: BlockItemSummary,
}

/// Prepare a contract initialization transaction payload.
pub(crate) fn prepare_contract_init(
    sender: AccountAddress,
    module_ref: ModuleReference,
    init_name: &str,
    amount: Amount,
    parameter: OwnedParameter,
    energy: Energy,
) -> Result<PreparedContractInit> {
    let init_name_owned = OwnedContractName::new(init_name.to_owned())
        .with_context(|| format!("invalid init name '{init_name}'"))?;
    let contract_name = init_name
        .strip_prefix("init_")
        .unwrap_or(init_name)
        .to_owned();
    Ok(PreparedContractInit {
        sender,
        payload: InitContractPayload {
            amount,
            mod_ref: module_ref,
            init_name: init_name_owned,
            param: parameter,
        },
        contract_name,
        energy,
    })
}

/// Simulate a prepared contract initialization.
pub(crate) async fn simulate_contract_init(
    client: v2::Client,
    prepared: &PreparedContractInit,
) -> ContractInitSimulation {
    match ContractInitBuilder::<()>::dry_run_new_instance_raw(
        client,
        prepared.sender,
        prepared.payload.mod_ref,
        &prepared.contract_name,
        prepared.payload.amount,
        prepared.payload.param.clone(),
    )
    .await
    {
        Ok(builder) => {
            let energy = builder.current_energy();
            ContractInitSimulation {
                message: format!(
                    "Simulation: contract init succeeded (estimated energy: {})",
                    energy.energy
                ),
                estimated_energy: Some(energy),
            }
        }
        Err(err) => ContractInitSimulation {
            message: format!("Simulation warning: {err}"),
            estimated_energy: None,
        },
    }
}

/// Submit a prepared contract initialization transaction.
pub(crate) async fn submit_contract_init(
    client: &mut v2::Client,
    wallet: &concordium_rust_sdk::types::WalletAccount,
    prepared: PreparedContractInit,
) -> Result<SubmittedContractInit> {
    let nonce = client
        .get_next_account_sequence_number(&wallet.address)
        .await
        .context("failed to get next account sequence number")?
        .nonce;
    let expiry = TransactionTime::from_seconds((chrono::Utc::now().timestamp() + 300) as u64);
    let tx = send::init_contract(
        wallet,
        wallet.address,
        nonce,
        expiry,
        prepared.payload,
        prepared.energy,
    );
    let transaction_hash = client
        .send_account_transaction(tx)
        .await
        .context("failed to submit contract init transaction")?;
    Ok(SubmittedContractInit { transaction_hash })
}

/// Wait until a submitted contract initialization finalizes.
pub(crate) async fn wait_for_contract_init_finalization(
    client: &mut v2::Client,
    submitted: SubmittedContractInit,
) -> Result<FinalizedContractInit> {
    let (block_hash, summary) = client
        .wait_until_finalized(&submitted.transaction_hash)
        .await
        .context("failed while waiting for contract init finalization")?;
    Ok(FinalizedContractInit {
        transaction_hash: submitted.transaction_hash,
        block_hash,
        summary,
    })
}
