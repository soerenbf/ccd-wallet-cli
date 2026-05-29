//! Contract-initialization approval, submission, and finalization flow.

use super::shared;
use crate::smart_contracts::init as init_core;
use anyhow::Result;
use ccd_wallet_connect::{ContractExecutionRejection, ContractInitApproval, ContractInitRequest};
use ccd_wallet_core::config as node_config;
use cliclack::{input, spinner};
use concordium_rust_sdk::{
    common::types::AccountAddress,
    smart_contracts::common::{ModuleReference, OwnedContractName},
    types::{Energy, transactions::InitContractPayload},
    v2,
};
use rusqlite::Connection;
use std::{
    str::FromStr,
    sync::{Arc, Mutex},
};

pub(super) struct PreparedContractInit {
    request: ContractInitRequest,
    endpoint: v2::Endpoint,
    endpoint_label: String,
    network_name: String,
    sender: AccountAddress,
    payload: InitContractPayload,
    energy: Energy,
}

pub(super) fn prepare_request(
    request: ContractInitRequest,
) -> std::result::Result<PreparedContractInit, ContractExecutionRejection> {
    let (network_name, network_entry) =
        shared::resolve_network_entry(&request.network_genesis_hash)
            .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let endpoint = v2::Endpoint::from_str(&network_entry.node_endpoint).map_err(|err| {
        ContractExecutionRejection::other(format!(
            "invalid node endpoint for {network_name}: {err}"
        ))
    })?;
    let endpoint_label = node_config::endpoint_label(&endpoint);
    let module_ref = ModuleReference::from_str(&request.module_ref)
        .map_err(|err| ContractExecutionRejection::other(format!("invalid moduleRef: {err}")))?;
    let payload = InitContractPayload {
        amount: shared::parse_amount_micro_ccd(&request.amount_micro_ccd)
            .map_err(|err| ContractExecutionRejection::other(err.to_string()))?,
        mod_ref: module_ref,
        init_name: OwnedContractName::new(request.init_name.clone())
            .map_err(|err| ContractExecutionRejection::other(format!("invalid initName: {err}")))?,
        param: shared::parse_parameter_hex(&request.parameter_hex)
            .map_err(|err| ContractExecutionRejection::other(err.to_string()))?,
    };
    let energy = Energy::from(request.max_contract_execution_energy);
    let sender = AccountAddress::from_str(&request.account_address).map_err(|err| {
        ContractExecutionRejection::other(format!("invalid session account address: {err}"))
    })?;
    Ok(PreparedContractInit {
        request,
        endpoint,
        endpoint_label,
        network_name,
        sender,
        payload,
        energy,
    })
}

pub(super) async fn submit_request(
    conn: Arc<Mutex<Connection>>,
    prepared: PreparedContractInit,
) -> std::result::Result<ContractInitApproval, ContractExecutionRejection> {
    let mut client = node_config::connect_v2_client(prepared.endpoint.clone())
        .await
        .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let prepared_core = init_core::prepare_contract_init(
        prepared.sender,
        prepared.payload.mod_ref,
        &prepared.request.init_name,
        prepared.payload.amount,
        prepared.payload.param.clone(),
        prepared.energy,
    )
    .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let simulation = if prepared.request.validate {
        Some(
            init_core::simulate_contract_init(client.clone(), &prepared_core)
                .await
                .message,
        )
    } else {
        None
    };
    print_prompt(
        &prepared.request,
        &prepared.network_name,
        &prepared.endpoint_label,
        simulation.as_deref(),
    )
    .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let confirmation: String = input("Approve and submit this contract init? Type y to approve:")
        .default_input("n")
        .interact()
        .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    if !confirmation.eq_ignore_ascii_case("y") && !confirmation.eq_ignore_ascii_case("yes") {
        return Err(ContractExecutionRejection::user_declined(
            "contract init declined by user",
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
    spin.start("Submitting contract init transaction...");
    let submitted = init_core::submit_contract_init(&mut client, &wallet, prepared_core)
        .await
        .map_err(|err| ContractExecutionRejection::submission_failed(err.to_string()))?;
    let transaction_hash = submitted.transaction_hash;
    spin.clear();
    let transaction_hash_label = transaction_hash.to_string();
    cliclack::log::success(format!(
        "Submitted contract init transaction on {} ({}): {transaction_hash_label}",
        prepared.network_name, prepared.endpoint_label
    ))
    .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let endpoint = prepared.endpoint;
    tokio::spawn(async move {
        let spin = spinner();
        spin.start("Waiting for contract init finalization...");
        match node_config::connect_v2_client(endpoint).await {
            Ok(mut client) => match client.wait_until_finalized(&transaction_hash).await {
                Ok((block_hash, summary)) => {
                    spin.clear();
                    if let Some(event) = summary.contract_init() {
                        let _ = cliclack::log::success(format!(
                            "Contract init finalized in block {block_hash}. New contract address: <{}, {}>.",
                            event.address.index, event.address.subindex
                        ));
                    } else {
                        let _ = cliclack::log::success(format!(
                            "Contract init finalized in block {block_hash}."
                        ));
                    }
                }
                Err(err) => {
                    spin.clear();
                    let _ = cliclack::log::error(format!(
                        "Failed while waiting for contract init finalization: {err}"
                    ));
                }
            },
            Err(err) => {
                spin.clear();
                let _ = cliclack::log::error(format!(
                    "Failed to reconnect for contract init finalization: {err}"
                ));
            }
        }
    });
    Ok(ContractInitApproval {
        transaction_hash: transaction_hash_label,
    })
}

fn print_prompt(
    request: &ContractInitRequest,
    network_name: &str,
    endpoint_label: &str,
    simulation: Option<&str>,
) -> Result<()> {
    let parameter_display = display_parameter(request);
    cliclack::log::info(format!(
        "Contract init request\norigin: {}\nnetwork: {} ({})\naccount: {}\nmodule: {}\ninit: {}\namount: {} microCCD\nmax energy: {}\nparameter: {}{}",
        request.origin,
        network_name,
        endpoint_label,
        request.account_address,
        request.module_ref,
        request.init_name,
        request.amount_micro_ccd,
        request.max_contract_execution_energy,
        parameter_display,
        simulation
            .map(|value| format!("\n{value}"))
            .unwrap_or_default()
    ))?;
    Ok(())
}

fn display_parameter(request: &ContractInitRequest) -> String {
    let contract_name = request
        .init_name
        .strip_prefix("init_")
        .unwrap_or(&request.init_name);
    shared::display_parameter_with_schema(
        &request.parameter_hex,
        request.schema.as_ref(),
        |schema| {
            schema
                .get_init_param_schema(contract_name)
                .map_err(anyhow::Error::from)
        },
    )
}
