//! Contract-update approval, submission, and finalization flow.

use super::shared;
use crate::smart_contracts::update as update_core;
use anyhow::{Result, bail};
use ccd_wallet_connect::{
    ContractExecutionRejection, ContractUpdateApproval, ContractUpdateRequest,
};
use ccd_wallet_core::config as node_config;
use cliclack::{input, spinner};
use concordium_rust_sdk::{
    common::types::AccountAddress,
    smart_contracts::common::{ContractAddress as SdkContractAddress, OwnedReceiveName},
    types::{Energy, transactions::UpdateContractPayload},
    v2,
};
use rusqlite::Connection;
use std::{
    str::FromStr,
    sync::{Arc, Mutex},
};

pub(super) struct PreparedContractUpdate {
    request: ContractUpdateRequest,
    endpoint: v2::Endpoint,
    endpoint_label: String,
    network_name: String,
    sender: AccountAddress,
    payload: UpdateContractPayload,
    energy: Energy,
}

pub(super) fn prepare_request(
    request: ContractUpdateRequest,
) -> std::result::Result<PreparedContractUpdate, ContractExecutionRejection> {
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
    let amount = shared::parse_amount_micro_ccd(&request.amount_micro_ccd)
        .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let parameter = shared::parse_parameter_hex(&request.parameter_hex)
        .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let payload = UpdateContractPayload {
        amount,
        address: SdkContractAddress::new(
            request.contract_address.index,
            request.contract_address.subindex,
        ),
        receive_name: OwnedReceiveName::new(request.receive_name.clone()).map_err(|err| {
            ContractExecutionRejection::other(format!("invalid receive name: {err}"))
        })?,
        message: parameter,
    };
    let energy = Energy::from(request.max_contract_execution_energy);
    Ok(PreparedContractUpdate {
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
    prepared: PreparedContractUpdate,
) -> std::result::Result<ContractUpdateApproval, ContractExecutionRejection> {
    let mut client = node_config::connect_v2_client(prepared.endpoint.clone())
        .await
        .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let prepared_core = update_core::PreparedContractUpdate {
        sender: prepared.sender,
        payload: prepared.payload.clone(),
        energy: prepared.energy,
    };
    let simulation = if prepared.request.validate {
        Some(
            update_core::simulate_contract_update(&mut client, &prepared_core)
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
    let confirmation: String = input("Approve and submit this contract update? Type y to approve:")
        .default_input("n")
        .interact()
        .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    if !confirmation.eq_ignore_ascii_case("y") && !confirmation.eq_ignore_ascii_case("yes") {
        return Err(ContractExecutionRejection::user_declined(
            "contract update declined by user",
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
    spin.start("Submitting contract update transaction...");
    let submitted = update_core::submit_contract_update(&mut client, &wallet, prepared_core)
        .await
        .map_err(|err| ContractExecutionRejection::submission_failed(err.to_string()))?;
    let transaction_hash = submitted.transaction_hash;
    spin.clear();
    let transaction_hash_label = transaction_hash.to_string();
    cliclack::log::success(format!(
        "Submitted contract update transaction on {} ({}): {transaction_hash_label}",
        prepared.network_name, prepared.endpoint_label
    ))
    .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;

    let endpoint = prepared.endpoint;
    tokio::spawn(async move {
        let spin = spinner();
        spin.start("Waiting for contract update finalization...");
        match node_config::connect_v2_client(endpoint).await {
            Ok(mut client) => match client.wait_until_finalized(&transaction_hash).await {
                Ok((block_hash, _summary)) => {
                    spin.clear();
                    let _ = cliclack::log::success(format!(
                        "Contract update finalized in block {block_hash}."
                    ));
                }
                Err(err) => {
                    spin.clear();
                    let _ = cliclack::log::error(format!(
                        "Failed while waiting for contract update finalization: {err}"
                    ));
                }
            },
            Err(err) => {
                spin.clear();
                let _ = cliclack::log::error(format!(
                    "Failed to reconnect for contract update finalization: {err}"
                ));
            }
        }
    });

    Ok(ContractUpdateApproval {
        transaction_hash: transaction_hash_label,
    })
}

fn print_prompt(
    request: &ContractUpdateRequest,
    network_name: &str,
    endpoint_label: &str,
    simulation: Option<&str>,
) -> Result<()> {
    let parameter_display = display_parameter(request);
    cliclack::log::info(format!(
        "Contract update request\norigin: {}\nnetwork: {} ({})\naccount: {}\ncontract: <{}, {}>\nreceive: {}\namount: {} microCCD\nmax energy: {}\nparameter: {}{}",
        request.origin,
        network_name,
        endpoint_label,
        request.account_address,
        request.contract_address.index,
        request.contract_address.subindex,
        request.receive_name,
        request.amount_micro_ccd,
        request.max_contract_execution_energy,
        parameter_display,
        simulation
            .map(|value| format!("\n{value}"))
            .unwrap_or_default()
    ))?;
    Ok(())
}

pub(super) fn display_parameter(request: &ContractUpdateRequest) -> String {
    shared::display_parameter_with_schema(
        &request.parameter_hex,
        request.schema.as_ref(),
        |schema| {
            let Some((contract_name, function_name)) = request.receive_name.split_once('.') else {
                bail!("receiveName must be fully qualified as '<contract>.<function>'");
            };
            schema
                .get_receive_param_schema(contract_name, function_name)
                .map_err(anyhow::Error::from)
        },
    )
}
