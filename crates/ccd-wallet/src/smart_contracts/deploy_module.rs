//! Shared deploy-module preparation, validation, submission, and finalization helpers.

use anyhow::{Context, Result};
use concordium_rust_sdk::{
    common::types::{AccountAddress, TransactionTime},
    contract_client::ModuleDeployBuilder,
    types::{
        BlockItemSummary,
        hashes::{BlockHash, TransactionHash},
        smart_contracts::{ModuleReference, WasmModule},
        transactions::send,
    },
    v2,
};
use std::time::Duration;

/// Warning text used when deploy validation finds an already deployed module.
pub(crate) const DUPLICATE_MODULE_MESSAGE: &str = "module already exists on chain for this network";

/// Prepared deploy-module payload independent of a concrete command entrypoint.
#[derive(Clone, Debug)]
pub(crate) struct PreparedDeployModule {
    /// Account expected to submit the deployment.
    pub(crate) sender: AccountAddress,
    /// Parsed Concordium smart contract module.
    pub(crate) module: WasmModule,
    /// Derived module reference for the parsed module.
    pub(crate) module_ref: ModuleReference,
    /// Original serialized module size in bytes.
    pub(crate) module_size: usize,
}

/// Result of submitting a deploy-module transaction.
pub(crate) struct SubmittedDeployModule {
    /// Submitted transaction hash.
    pub(crate) transaction_hash: TransactionHash,
    /// Derived module reference for the submitted module.
    pub(crate) module_ref: ModuleReference,
}

/// Result of waiting for deploy-module finalization.
pub(crate) struct FinalizedDeployModule {
    /// Submitted transaction hash.
    pub(crate) transaction_hash: TransactionHash,
    /// Finalized block hash.
    pub(crate) block_hash: BlockHash,
    /// Finalized block item summary.
    pub(crate) summary: BlockItemSummary,
    /// Derived module reference for the submitted module.
    pub(crate) module_ref: ModuleReference,
}

/// Prepare a deploy-module transaction from serialized module bytes.
///
/// # Arguments
///
/// * `sender` - Account address expected to submit the transaction.
/// * `module_bytes` - Serialized Concordium module bytes.
///
/// # Errors
///
/// Returns an error if `module_bytes` is not a valid Concordium smart contract module.
///
/// # Examples
///
/// ```ignore
/// let prepared = prepare_deploy_module(sender, &bytes)?;
/// println!("{}", prepared.module_ref);
/// # anyhow::Ok(())
/// ```
pub(crate) fn prepare_deploy_module(
    sender: AccountAddress,
    module_bytes: &[u8],
) -> Result<PreparedDeployModule> {
    let module_size = module_bytes.len();
    let module = WasmModule::from_slice(module_bytes)
        .with_context(|| "module file is not a valid Concordium module")?;
    let module_ref = module.get_module_ref();
    Ok(PreparedDeployModule {
        sender,
        module,
        module_ref,
        module_size,
    })
}

/// Validate a deploy-module request by checking whether the module already exists.
///
/// # Arguments
///
/// * `client` - Node client connected to the target chain.
/// * `prepared` - Prepared deploy-module payload.
///
/// # Returns
///
/// A warning message when validation finds a duplicate module or cannot complete.
/// `None` means validation completed without a warning.
///
/// # Errors
///
/// This helper does not return node validation errors directly; transient failures are converted
/// to warnings so callers can preserve user approval as the final decision.
///
/// # Examples
///
/// ```ignore
/// if let Some(warning) = validate_deploy_module(client, &prepared).await {
///     eprintln!("{warning}");
/// }
/// ```
pub(crate) async fn validate_deploy_module(
    client: v2::Client,
    prepared: &PreparedDeployModule,
) -> Option<String> {
    match tokio::time::timeout(
        Duration::from_secs(10),
        ModuleDeployBuilder::dry_run_module_deploy(
            client,
            prepared.sender,
            prepared.module.clone(),
        ),
    )
    .await
    {
        Ok(Ok(_builder)) => None,
        Ok(Err(err)) if err.already_exists() => {
            Some(format!("Validation warning: {DUPLICATE_MODULE_MESSAGE}."))
        }
        Ok(Err(err)) => Some(format!("Validation warning: {err}")),
        Err(_elapsed) => Some(
            "Validation warning: timed out while checking whether the module already exists on chain."
                .to_owned(),
        ),
    }
}

/// Sign and submit a prepared deploy-module transaction.
///
/// # Arguments
///
/// * `client` - Node client connected to the target chain.
/// * `wallet` - Signer-capable wallet account.
/// * `prepared` - Prepared deploy-module payload to submit.
///
/// # Errors
///
/// Returns an error if nonce lookup or transaction submission fails.
///
/// # Examples
///
/// ```ignore
/// let submitted = submit_deploy_module(&mut client, &wallet, prepared).await?;
/// println!("{}", submitted.transaction_hash);
/// # anyhow::Ok(())
/// ```
pub(crate) async fn submit_deploy_module(
    client: &mut v2::Client,
    wallet: &concordium_rust_sdk::types::WalletAccount,
    prepared: PreparedDeployModule,
) -> Result<SubmittedDeployModule> {
    let nonce = client
        .get_next_account_sequence_number(&wallet.address)
        .await
        .context("failed to get next account sequence number")?
        .nonce;
    let expiry = TransactionTime::from_seconds((chrono::Utc::now().timestamp() + 300) as u64);
    let module_ref = prepared.module_ref;
    let tx = send::deploy_module(&wallet, wallet.address, nonce, expiry, prepared.module);
    let transaction_hash = client
        .send_account_transaction(tx)
        .await
        .context("failed to submit deploy-module transaction")?;
    Ok(SubmittedDeployModule {
        transaction_hash,
        module_ref,
    })
}

/// Wait until a submitted deploy-module transaction is finalized.
///
/// # Arguments
///
/// * `client` - Node client connected to the target chain.
/// * `submitted` - Submitted deploy-module transaction metadata.
///
/// # Errors
///
/// Returns an error if finalization waiting fails.
///
/// # Examples
///
/// ```ignore
/// let finalized = wait_for_deploy_module_finalization(&mut client, submitted).await?;
/// println!("{}", finalized.block_hash);
/// # anyhow::Ok(())
/// ```
pub(crate) async fn wait_for_deploy_module_finalization(
    client: &mut v2::Client,
    submitted: SubmittedDeployModule,
) -> Result<FinalizedDeployModule> {
    let (block_hash, summary) = client
        .wait_until_finalized(&submitted.transaction_hash)
        .await
        .context("failed while waiting for deploy-module finalization")?;
    Ok(FinalizedDeployModule {
        transaction_hash: submitted.transaction_hash,
        block_hash,
        summary,
        module_ref: submitted.module_ref,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_module_message_matches_chain_behavior() {
        assert_eq!(
            DUPLICATE_MODULE_MESSAGE,
            "module already exists on chain for this network"
        );
    }

    #[test]
    fn invalid_module_bytes_are_rejected() {
        let sender = "47b6Qe2XtZANHetanWKP1PbApLKtS3AyiCtcXaqLMbypKjCaRw"
            .parse()
            .unwrap();
        let err = prepare_deploy_module(sender, &[0, 1, 2]).unwrap_err();
        assert!(err.to_string().contains("valid Concordium module"));
    }
}
