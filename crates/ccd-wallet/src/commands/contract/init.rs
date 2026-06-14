//! Contract initialization command.

use crate::{
    cli::ContractInitArgs,
    commands::{
        contract::shared::{PreparedContractSubmission, resolve_prepared_submission_context},
        transaction::render::render_finalized_summary,
    },
    smart_contracts::{init as init_core, shared},
};
use anyhow::{Context, Result, bail};
use cliclack::{input, spinner};
use concordium_rust_sdk::types::Energy;
use rusqlite::Connection;

pub(super) async fn init(conn: &Connection, args: ContractInitArgs) -> Result<()> {
    let submission = PreparedContractSubmission::from_raw(
        args.account.as_deref(),
        args.network.as_deref(),
        args.node,
        args.non_interactive,
        args.no_defaults,
        args.no_wait,
    )?;
    let mut context = resolve_prepared_submission_context(conn, &submission).await?;
    let client = &mut context.client;

    let module_ref = args.module_ref;
    let amount = args
        .amount
        .unwrap_or_else(concordium_rust_sdk::common::types::Amount::zero);
    let block = concordium_rust_sdk::v2::BlockIdentifier::LastFinal;
    let contract_name = args
        .init_name
        .strip_prefix("init_")
        .unwrap_or(&args.init_name)
        .to_owned();
    let parameter = shared::resolve_parameter(
        client,
        block,
        args.parameter_hex.as_deref(),
        args.parameter_json.as_deref(),
        args.parameter_json_file.as_deref(),
        shared::SchemaSource::Module(module_ref),
        shared::SchemaParameter::Init {
            contract_name: contract_name.clone(),
        },
    )
    .await?;

    let simulation = if args.validate || args.energy.is_none() {
        let provisional = init_core::prepare_contract_init(
            context.wallet.address,
            module_ref,
            &args.init_name,
            amount,
            parameter.clone(),
            Energy::from(args.energy.unwrap_or(10_000_000)),
        )?;
        let spin = spinner();
        spin.start("Simulating contract init...");
        let simulation = init_core::simulate_contract_init(client.clone(), &provisional).await;
        spin.clear();
        Some(simulation)
    } else {
        None
    };
    let energy = resolve_energy(
        args.energy,
        simulation.as_ref(),
        !submission.input_mode().prompts_allowed(),
    )?;
    let prepared = init_core::prepare_contract_init(
        context.wallet.address,
        module_ref,
        &args.init_name,
        amount,
        parameter,
        energy,
    )?;

    print_review_prompt(
        &context.network_name,
        &context.endpoint_label,
        &prepared,
        simulation.as_ref(),
    )?;
    let confirmation: String = input("Approve and submit this contract init? Type y to approve:")
        .default_input("n")
        .interact()?;
    if !confirmation.eq_ignore_ascii_case("y") && !confirmation.eq_ignore_ascii_case("yes") {
        cliclack::log::warning("contract init declined by user")?;
        return Ok(());
    }

    let spin = spinner();
    spin.start("Submitting contract init transaction...");
    let submitted = init_core::submit_contract_init(client, &context.wallet, prepared).await?;
    spin.clear();
    let transaction_hash_label = submitted.transaction_hash.to_string();
    cliclack::log::success(format!(
        "Submitted contract init transaction on {} ({}): {transaction_hash_label}",
        context.network_name, context.endpoint_label
    ))?;
    if !submission.should_wait_for_finalization() {
        return Ok(());
    }

    let spin = spinner();
    spin.start("Waiting for contract init finalization...");
    let finalized = init_core::wait_for_contract_init_finalization(client, submitted).await?;
    spin.clear();
    let block_time = client
        .get_block_info(finalized.block_hash)
        .await
        .ok()
        .map(|info| {
            info.response
                .block_slot_time
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        });
    println!(
        "{}",
        render_finalized_summary(
            &finalized.transaction_hash,
            &format!("{} @ {}", context.network_name, context.endpoint_label),
            &finalized.block_hash,
            &finalized.summary,
            block_time.as_ref(),
        )?
    );
    Ok(())
}

fn resolve_energy(
    supplied: Option<u64>,
    simulation: Option<&init_core::ContractInitSimulation>,
    non_interactive: bool,
) -> Result<Energy> {
    if let Some(value) = supplied {
        return Ok(Energy::from(value));
    }
    if non_interactive {
        bail!("--energy is required in non-interactive mode");
    }
    let mut prompt = input("Maximum contract execution energy:");
    let default = simulation
        .and_then(|simulation| simulation.estimated_energy)
        .map(|energy| energy.energy.to_string());
    if let Some(default) = default.as_deref() {
        prompt = prompt.default_input(default);
    }
    let value: String = prompt.interact()?;
    Ok(Energy::from(
        value
            .trim()
            .parse::<u64>()
            .context("energy must be an unsigned integer")?,
    ))
}

fn print_review_prompt(
    network_name: &str,
    endpoint_label: &str,
    prepared: &init_core::PreparedContractInit,
    simulation: Option<&init_core::ContractInitSimulation>,
) -> Result<()> {
    cliclack::log::info(format!(
        "Contract init transaction\nnetwork: {network_name} ({endpoint_label})\naccount: {}\nmodule: {}\ninit: {}\namount: {} microCCD\nmax energy: {}\nparameter: {} bytes{}",
        prepared.sender,
        prepared.payload.mod_ref,
        prepared.payload.init_name,
        prepared.payload.amount.micro_ccd(),
        prepared.energy.energy,
        prepared.payload.param.as_ref().len(),
        simulation
            .map(|simulation| format!("\n{}", simulation.message))
            .unwrap_or_default(),
    ))?;
    Ok(())
}
