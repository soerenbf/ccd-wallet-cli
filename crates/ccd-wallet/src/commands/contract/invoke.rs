//! Read-only contract invocation command.

use crate::{
    cli::ContractInvokeArgs,
    commands::account::resolve_account_network_context,
    smart_contracts::{query as query_core, shared},
};
use anyhow::{Context, Result};
use ccd_wallet_core::config as node_config;
use concordium_rust_sdk::types::Energy;
use rusqlite::Connection;

pub(super) async fn invoke(conn: &Connection, args: ContractInvokeArgs) -> Result<()> {
    let (_network_name, _network_entry, endpoint, endpoint_label, _network_source) =
        resolve_account_network_context(
            conn,
            args.network.as_deref(),
            args.node,
            false,
            args.no_defaults,
        )
        .await?;
    let mut client = node_config::connect_v2_client(endpoint.clone())
        .await
        .with_context(|| format!("failed to connect to Concordium node at {endpoint_label}"))?;
    let block = shared::parse_block_identifier(args.block.as_deref())?;
    let contract = shared::parse_contract_address(&args.contract)?;
    let (receive_name, contract_name, function_name) = shared::parse_receive_name(&args.receive)?;
    let amount = shared::parse_decimal_ccd_amount(args.amount.as_deref())?;
    let parameter = shared::resolve_parameter(
        &mut client,
        block,
        args.parameter_hex.as_deref(),
        args.parameter_json.as_deref(),
        args.parameter_json_file.as_deref(),
        shared::SchemaSource::Contract(contract),
        shared::SchemaParameter::Receive {
            contract_name,
            function_name,
        },
    )
    .await?;
    let invoker = args
        .invoker
        .as_deref()
        .map(shared::parse_account_address)
        .transpose()?;
    let prepared = query_core::prepare_contract_invoke(
        contract,
        receive_name,
        amount,
        parameter,
        args.energy.map(Energy::from),
        invoker,
    );
    let result = query_core::invoke_contract(&mut client, block, &prepared).await?;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&query_core::invoke_result_json(&result))?
        );
    } else {
        println!("{}", query_core::render_invoke_result(&result));
    }
    Ok(())
}
