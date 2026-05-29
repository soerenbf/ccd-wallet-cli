//! Contract parameter template command.

use crate::{
    cli::{
        ContractParameterTemplateArgs, ContractParameterTemplateInitArgs,
        ContractParameterTemplateReceiveArgs, ContractParameterTemplateSubcommand,
    },
    commands::account::resolve_account_network_context,
    smart_contracts::shared,
};
use anyhow::{Context, Result, bail};
use ccd_wallet_core::config as node_config;
use rusqlite::Connection;

pub(super) async fn parameter_template(
    conn: &Connection,
    args: ContractParameterTemplateArgs,
) -> Result<()> {
    match args.command {
        ContractParameterTemplateSubcommand::Init(args) => init(conn, *args).await,
        ContractParameterTemplateSubcommand::Receive(args) => receive(conn, *args).await,
    }
}

async fn init(conn: &Connection, args: ContractParameterTemplateInitArgs) -> Result<()> {
    let mut client = client(conn, args.network.as_deref(), args.node, args.no_defaults).await?;
    let block = shared::parse_block_identifier(args.block.as_deref())?;
    let module_ref = shared::parse_module_reference(&args.module_ref)?;
    let contract_name = args
        .init_name
        .strip_prefix("init_")
        .unwrap_or(&args.init_name)
        .to_owned();
    let template = shared::parameter_template(
        &mut client,
        block,
        shared::SchemaSource::Module(module_ref),
        shared::SchemaParameter::Init { contract_name },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&template)?);
    Ok(())
}

async fn receive(conn: &Connection, args: ContractParameterTemplateReceiveArgs) -> Result<()> {
    let mut client = client(conn, args.network.as_deref(), args.node, args.no_defaults).await?;
    let block = shared::parse_block_identifier(args.block.as_deref())?;
    let (source, contract_name, function_name) =
        match (args.contract.as_deref(), args.module_ref.as_deref()) {
            (Some(_), Some(_)) | (None, None) => {
                bail!("provide exactly one of --contract or --module-ref")
            }
            (Some(contract), None) => {
                let contract = shared::parse_contract_address(contract)?;
                let (_receive, contract_name, function_name) =
                    shared::parse_receive_name(&args.receive)?;
                (
                    shared::SchemaSource::Contract(contract),
                    contract_name,
                    function_name,
                )
            }
            (None, Some(module_ref)) => {
                let module_ref = shared::parse_module_reference(module_ref)?;
                let (_receive, contract_name, function_name) =
                    shared::parse_receive_name(&args.receive)?;
                (
                    shared::SchemaSource::Module(module_ref),
                    contract_name,
                    function_name,
                )
            }
        };
    let template = shared::parameter_template(
        &mut client,
        block,
        source,
        shared::SchemaParameter::Receive {
            contract_name,
            function_name,
        },
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&template)?);
    Ok(())
}

async fn client(
    conn: &Connection,
    network: Option<&str>,
    node: Option<concordium_rust_sdk::v2::Endpoint>,
    no_defaults: bool,
) -> Result<concordium_rust_sdk::v2::Client> {
    let (_network_name, _network_entry, endpoint, endpoint_label, _network_source) =
        resolve_account_network_context(conn, network, node, false, no_defaults).await?;
    node_config::connect_v2_client(endpoint.clone())
        .await
        .with_context(|| format!("failed to connect to Concordium node at {endpoint_label}"))
}
