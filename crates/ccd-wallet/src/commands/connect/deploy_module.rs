//! Deploy-module approval, submission, validation, and finalization flow.

use super::shared;
use crate::smart_contracts::deploy_module as deploy_core;
use ccd_wallet_connect::{ContractExecutionRejection, DeployModuleApproval, DeployModuleRequest};
use ccd_wallet_core::config as node_config;
use cliclack::{input, spinner};
use concordium_rust_sdk::{common::types::AccountAddress, v2};
use rusqlite::Connection;
use std::{
    str::FromStr,
    sync::{Arc, Mutex},
};

struct PreparedDeployModule {
    request: DeployModuleRequest,
    endpoint: v2::Endpoint,
    endpoint_label: String,
    network_name: String,
    deploy: deploy_core::PreparedDeployModule,
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
    let deploy = deploy_core::prepare_deploy_module(sender, &module_bytes).map_err(|err| {
        ContractExecutionRejection::other(format!(
            "moduleHex is not a valid Concordium module: {err}"
        ))
    })?;
    Ok(PreparedDeployModule {
        request,
        endpoint,
        endpoint_label,
        network_name,
        deploy,
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
        let warning = deploy_core::validate_deploy_module(client.clone(), &prepared.deploy).await;
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
    let spin = spinner();
    spin.start("Submitting deploy-module transaction...");
    let submitted = deploy_core::submit_deploy_module(&mut client, &wallet, prepared.deploy)
        .await
        .map_err(|err| ContractExecutionRejection::submission_failed(err.to_string()))?;
    spin.clear();
    let transaction_hash_label = submitted.transaction_hash.to_string();
    cliclack::log::success(format!(
        "Submitted deploy-module transaction on {} ({}): {transaction_hash_label}",
        prepared.network_name, prepared.endpoint_label
    ))
    .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;

    let endpoint = prepared.endpoint;
    tokio::spawn(async move {
        let spin = spinner();
        spin.start("Waiting for deploy-module finalization...");
        match node_config::connect_v2_client(endpoint).await {
            Ok(mut client) => {
                match deploy_core::wait_for_deploy_module_finalization(&mut client, submitted).await
                {
                    Ok(finalized) => {
                        spin.clear();
                        let _ = cliclack::log::success(format!(
                            "Deploy-module transaction finalized in block {}. Module reference: {}.",
                            finalized.block_hash, finalized.module_ref
                        ));
                    }
                    Err(err) => {
                        spin.clear();
                        let _ = cliclack::log::error(format!(
                            "Failed while waiting for deploy-module finalization: {err}"
                        ));
                    }
                }
            }
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
        "Deploy module request\norigin: {}\nnetwork: {} ({})\naccount: {}\nmodule reference: {}\nmodule size: {} bytes",
        prepared.request.origin,
        prepared.network_name,
        prepared.endpoint_label,
        prepared.request.account_address,
        prepared.deploy.module_ref,
        prepared.deploy.module_size,
    ))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_module_message_matches_chain_behavior() {
        assert_eq!(
            deploy_core::DUPLICATE_MODULE_MESSAGE,
            "module already exists on chain for this network"
        );
    }
}
