//! Read-only contract invocation command.

use crate::{
    cli::ContractInvokeArgs,
    commands::{
        account::{
            AccountReferenceContext, AccountReferenceUnlocks, decrypt_local_account_address,
            local_account_context_lines, resolve_account_reference,
            resolve_account_reference_network_context,
        },
        ui::{ContextLine, log_resolved_context},
    },
    smart_contracts::{query as query_core, shared},
};
use anyhow::{Context, Result};
use ccd_wallet_core::config as node_config;
use concordium_rust_sdk::types::Energy;
use rusqlite::Connection;

pub(super) async fn invoke(conn: &Connection, args: ContractInvokeArgs) -> Result<()> {
    let (network_context, selected_invoker) = resolve_account_reference_network_context(
        conn,
        args.network.as_deref(),
        args.node,
        args.invoker.as_deref(),
        false,
        args.no_defaults,
    )
    .await?;
    let mut lines = vec![ContextLine {
        label: "network:",
        value: format!(
            "{} @ {}",
            network_context.network_name, network_context.endpoint_label
        ),
        source: network_context.source,
    }];
    if let Some(selection) = selected_invoker.as_ref() {
        lines.extend(local_account_context_lines(
            conn,
            &selection.record,
            selection.source,
        )?);
    }
    log_resolved_context(&lines)?;

    let network_name = network_context.network_name;
    let network_entry = network_context.network_entry;
    let endpoint = network_context.endpoint;
    let endpoint_label = network_context.endpoint_label;
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
    let invoker = match (args.invoker.as_deref(), selected_invoker.as_ref()) {
        (_, Some(selection)) => Some(decrypt_local_account_address(
            conn,
            &network_name,
            &selection.record,
        )?),
        (Some(invoker), None) => Some(resolve_account_reference(
            conn,
            AccountReferenceContext {
                network_name: &network_name,
                network_genesis_hash: &network_entry.genesis_hash,
            },
            Some(invoker),
            "Invoker account address or local label:",
            "invoker",
            true,
            &mut AccountReferenceUnlocks::new(),
        )?),
        (None, None) => None,
    };
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
