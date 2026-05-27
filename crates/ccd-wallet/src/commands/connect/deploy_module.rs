//! Deploy-module approval, submission, validation, and finalization flow.

use super::shared;
use ccd_wallet_connect::{ContractExecutionRejection, DeployModuleApproval, DeployModuleRequest};
use ccd_wallet_core::config as node_config;
use cliclack::{input, spinner};
use concordium_rust_sdk::{
    common::types::{AccountAddress, TransactionTime},
    contract_client::ModuleDeployBuilder,
    types::{
        smart_contracts::{ModuleReference, WasmModule},
        transactions::send,
    },
    v2,
};
use rusqlite::Connection;
use std::{
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

const DUPLICATE_MODULE_MESSAGE: &str = "module already exists on chain for this network; submitting again is expected to reuse the same module reference";

struct PreparedDeployModule {
    request: DeployModuleRequest,
    endpoint: v2::Endpoint,
    endpoint_label: String,
    network_name: String,
    sender: AccountAddress,
    module: WasmModule,
    module_ref: ModuleReference,
    module_size: usize,
}

pub(super) async fn submit_request(
    conn: Arc<Mutex<Connection>>,
    request: DeployModuleRequest,
) -> std::result::Result<DeployModuleApproval, ContractExecutionRejection> {
    let prepared = prepare_request(request)?;
    submit_prepared_request(conn, prepared).await
}

fn prepare_request(
    request: DeployModuleRequest,
) -> std::result::Result<PreparedDeployModule, ContractExecutionRejection> {
    let (network_name, network_entry) =
        shared::resolve_network_entry(&request.network_genesis_hash)
            .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let endpoint = v2::Endpoint::from_str(&network_entry.node_endpoint).map_err(|err| {
        ContractExecutionRejection::other(format!(
            "invalid node endpoint for {network_name}: {err}"
        ))
    })?;
    let endpoint_label = node_config::endpoint_label(&endpoint);
    let sender = AccountAddress::from_str(&request.account_address).map_err(|err| {
        ContractExecutionRejection::other(format!("invalid session account address: {err}"))
    })?;
    let module_bytes = shared::parse_hex_bytes(&request.module_hex, "moduleHex")
        .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let module_size = module_bytes.len();
    let module = WasmModule::from_slice(&module_bytes).map_err(|err| {
        ContractExecutionRejection::other(format!(
            "moduleHex is not a valid Concordium module: {err}"
        ))
    })?;
    let module_ref = module.get_module_ref();
    Ok(PreparedDeployModule {
        request,
        endpoint,
        endpoint_label,
        network_name,
        sender,
        module,
        module_ref,
        module_size,
    })
}

async fn submit_prepared_request(
    conn: Arc<Mutex<Connection>>,
    prepared: PreparedDeployModule,
) -> std::result::Result<DeployModuleApproval, ContractExecutionRejection> {
    let mut client = node_config::connect_v2_client(prepared.endpoint.clone())
        .await
        .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let validation_warning = if prepared.request.validate {
        let spin = spinner();
        spin.start("Validating deploy-module request...");
        let warning = match tokio::time::timeout(
            Duration::from_secs(10),
            ModuleDeployBuilder::dry_run_module_deploy(
                client.clone(),
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
        };
        spin.clear();
        warning
    } else {
        None
    };

    print_prompt(&prepared).map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    if let Some(warning) = validation_warning {
        cliclack::log::warning(warning)
            .map_err(|log_err| ContractExecutionRejection::other(log_err.to_string()))?;
    }
    let confirmation: String =
        input("Approve and submit this deploy-module transaction? Type y to approve:")
            .default_input("n")
            .interact()
            .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    if !confirmation.eq_ignore_ascii_case("y") && !confirmation.eq_ignore_ascii_case("yes") {
        return Err(ContractExecutionRejection::user_declined(
            "deploy-module transaction declined by user",
        ));
    }

    let (resolved_network_name, network_entry) =
        shared::resolve_network_entry(&prepared.request.network_genesis_hash)
            .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let wallet = {
        let conn = conn.lock().map_err(|_| {
            ContractExecutionRejection::other("wallet database connection is unavailable")
        })?;
        shared::unlock_wallet_account(
            &conn,
            &resolved_network_name,
            &network_entry,
            &prepared.request.account_address,
        )
        .map_err(|err| ContractExecutionRejection::other(err.to_string()))?
    };
    let nonce = client
        .get_next_account_sequence_number(&wallet.address)
        .await
        .map_err(|err| ContractExecutionRejection::submission_failed(err.to_string()))?
        .nonce;
    let expiry = TransactionTime::from_seconds((chrono::Utc::now().timestamp() + 300) as u64);
    let tx = send::deploy_module(&wallet, wallet.address, nonce, expiry, prepared.module);
    let spin = spinner();
    spin.start("Submitting deploy-module transaction...");
    let transaction_hash = client
        .send_account_transaction(tx)
        .await
        .map_err(|err| ContractExecutionRejection::submission_failed(err.to_string()))?;
    spin.clear();
    let transaction_hash_label = transaction_hash.to_string();
    cliclack::log::success(format!(
        "Submitted deploy-module transaction on {} ({}): {transaction_hash_label}",
        prepared.network_name, prepared.endpoint_label
    ))
    .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;

    let endpoint = prepared.endpoint;
    let module_ref = prepared.module_ref;
    tokio::spawn(async move {
        let spin = spinner();
        spin.start("Waiting for deploy-module finalization...");
        match node_config::connect_v2_client(endpoint).await {
            Ok(mut client) => match client.wait_until_finalized(&transaction_hash).await {
                Ok((block_hash, _summary)) => {
                    spin.clear();
                    let _ = cliclack::log::success(format!(
                        "Deploy-module transaction finalized in block {block_hash}. Module reference: {module_ref}."
                    ));
                }
                Err(err) => {
                    spin.clear();
                    let _ = cliclack::log::error(format!(
                        "Failed while waiting for deploy-module finalization: {err}"
                    ));
                }
            },
            Err(err) => {
                spin.clear();
                let _ = cliclack::log::error(format!(
                    "Failed to reconnect for deploy-module finalization: {err}"
                ));
            }
        }
    });

    Ok(DeployModuleApproval {
        transaction_hash: transaction_hash_label,
    })
}

fn print_prompt(prepared: &PreparedDeployModule) -> anyhow::Result<()> {
    cliclack::log::info(format!(
        "Deploy module request\norigin: {}\nnetwork: {} ({})\naccount: {}\nmodule reference: {}\nmodule size: {} bytes\nvalidation requested: {}",
        prepared.request.origin,
        prepared.network_name,
        prepared.endpoint_label,
        prepared.request.account_address,
        prepared.module_ref,
        prepared.module_size,
        prepared.request.validate,
    ))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_module_message_matches_chain_behavior() {
        assert!(DUPLICATE_MODULE_MESSAGE.contains("module already exists on chain"));
        assert!(DUPLICATE_MODULE_MESSAGE.contains("expected to reuse the same module reference"));
    }
}
