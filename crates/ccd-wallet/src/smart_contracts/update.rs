//! Shared contract update preparation, simulation, submission, and finalization helpers.

use anyhow::{Context, Result};
use concordium_rust_sdk::{
    common::types::{AccountAddress, Amount, TransactionTime},
    smart_contracts::common::{ContractAddress, OwnedParameter, OwnedReceiveName},
    types::{
        BlockItemSummary, Energy,
        hashes::{BlockHash, TransactionHash},
        smart_contracts::ContractContext,
        transactions::{UpdateContractPayload, send},
    },
    v2,
};

/// Prepared contract update transaction data.
#[derive(Clone)]
pub(crate) struct PreparedContractUpdate {
    /// Account expected to submit the transaction.
    pub(crate) sender: AccountAddress,
    /// Update payload.
    pub(crate) payload: UpdateContractPayload,
    /// Maximum execution energy.
    pub(crate) energy: Energy,
}

/// Result of simulating a contract update.
pub(crate) struct ContractUpdateSimulation {
    /// Human-readable simulation message.
    pub(crate) message: String,
    /// Used energy when simulation returned a node response.
    pub(crate) estimated_energy: Option<Energy>,
}

/// Submitted contract update metadata.
pub(crate) struct SubmittedContractUpdate {
    /// Submitted transaction hash.
    pub(crate) transaction_hash: TransactionHash,
}

/// Finalized contract update metadata.
pub(crate) struct FinalizedContractUpdate {
    /// Submitted transaction hash.
    pub(crate) transaction_hash: TransactionHash,
    /// Finalized block hash.
    pub(crate) block_hash: BlockHash,
    /// Finalized block item summary.
    pub(crate) summary: BlockItemSummary,
}

/// Prepare a contract update transaction payload.
pub(crate) fn prepare_contract_update(
    sender: AccountAddress,
    contract: ContractAddress,
    receive_name: OwnedReceiveName,
    amount: Amount,
    parameter: OwnedParameter,
    energy: Energy,
) -> PreparedContractUpdate {
    PreparedContractUpdate {
        sender,
        payload: UpdateContractPayload {
            amount,
            address: contract,
            receive_name,
            message: parameter,
        },
        energy,
    }
}

/// Simulate a prepared contract update.
pub(crate) async fn simulate_contract_update(
    client: &mut v2::Client,
    prepared: &PreparedContractUpdate,
) -> ContractUpdateSimulation {
    let context = ContractContext::new_from_payload(
        prepared.sender,
        prepared.energy,
        prepared.payload.clone(),
    );
    match client
        .invoke_instance(v2::BlockIdentifier::Best, &context)
        .await
    {
        Ok(response) => {
            let used = response.response.used_energy();
            ContractUpdateSimulation {
                message: format!(
                    "Simulation: {:?} (used energy: {})",
                    response.response, used.energy
                ),
                estimated_energy: Some(used),
            }
        }
        Err(err) => ContractUpdateSimulation {
            message: format!("Simulation warning: {err}"),
            estimated_energy: None,
        },
    }
}

/// Submit a prepared contract update transaction.
pub(crate) async fn submit_contract_update(
    client: &mut v2::Client,
    wallet: &concordium_rust_sdk::types::WalletAccount,
    prepared: PreparedContractUpdate,
) -> Result<SubmittedContractUpdate> {
    let nonce = client
        .get_next_account_sequence_number(&wallet.address)
        .await
        .context("failed to get next account sequence number")?
        .nonce;
    let expiry = TransactionTime::from_seconds((chrono::Utc::now().timestamp() + 300) as u64);
    let tx = send::update_contract(
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
        .context("failed to submit contract update transaction")?;
    Ok(SubmittedContractUpdate { transaction_hash })
}

/// Wait until a submitted contract update finalizes.
pub(crate) async fn wait_for_contract_update_finalization(
    client: &mut v2::Client,
    submitted: SubmittedContractUpdate,
) -> Result<FinalizedContractUpdate> {
    let (block_hash, summary) = client
        .wait_until_finalized(&submitted.transaction_hash)
        .await
        .context("failed while waiting for contract update finalization")?;
    Ok(FinalizedContractUpdate {
        transaction_hash: submitted.transaction_hash,
        block_hash,
        summary,
    })
}
