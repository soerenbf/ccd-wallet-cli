//! Deploy-module smart contract command.

use crate::commands::transaction::render::render_finalized_summary;
use crate::{
    cli::ContractDeployModuleArgs,
    commands::contract::shared::{PreparedContractSubmission, resolve_prepared_submission_context},
    smart_contracts::deploy_module as deploy_core,
};
use anyhow::{Context, Result};
use cliclack::{input, spinner};
use rusqlite::Connection;
use std::fs;

pub(super) async fn deploy_module(conn: &Connection, args: ContractDeployModuleArgs) -> Result<()> {
    let module_bytes = fs::read(&args.file)
        .with_context(|| format!("failed to read module file {}", args.file.display()))?;
    let submission = PreparedContractSubmission::from_raw(
        args.account.as_deref(),
        args.network.as_deref(),
        args.node,
        args.non_interactive,
        args.no_defaults,
        args.no_wait,
    )?;
    let mut context = resolve_prepared_submission_context(conn, &submission)
        .await
        .with_context(|| {
            format!(
                "failed to connect to Concordium node at {}",
                args.network.as_deref().unwrap_or("selected endpoint")
            )
        })?;
    let prepared = deploy_core::prepare_deploy_module(context.wallet.address, &module_bytes)?;

    let client = &mut context.client;

    let validate = !args.no_validate;
    let validation_warning = if validate {
        let spin = spinner();
        spin.start("Validating deploy-module transaction...");
        let warning = deploy_core::validate_deploy_module(client.clone(), &prepared).await;
        spin.clear();
        warning
    } else {
        None
    };

    print_review_prompt(
        &context.network_name,
        &context.endpoint_label,
        &context.wallet.address.to_string(),
        &prepared,
    )?;
    if let Some(warning) = validation_warning {
        cliclack::log::warning(warning)?;
    }
    let confirmation: String =
        input("Approve and submit this deploy-module transaction? Type y to approve:")
            .default_input("n")
            .interact()?;
    if !confirmation.eq_ignore_ascii_case("y") && !confirmation.eq_ignore_ascii_case("yes") {
        cliclack::log::warning("deploy-module transaction declined by user")?;
        return Ok(());
    }

    let spin = spinner();
    spin.start("Submitting deploy-module transaction...");
    let submitted = deploy_core::submit_deploy_module(client, &context.wallet, prepared).await?;
    spin.clear();
    let transaction_hash_label = submitted.transaction_hash.to_string();
    cliclack::log::success(format!(
        "Submitted deploy-module transaction on {} ({}): {transaction_hash_label}",
        context.network_name, context.endpoint_label
    ))?;

    if !submission.should_wait_for_finalization() {
        return Ok(());
    }

    let spin = spinner();
    spin.start("Waiting for deploy-module finalization...");
    let finalized = deploy_core::wait_for_deploy_module_finalization(client, submitted).await?;
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

fn print_review_prompt(
    network_name: &str,
    endpoint_label: &str,
    account_address: &str,
    prepared: &deploy_core::PreparedDeployModule,
) -> Result<()> {
    cliclack::log::info(format!(
        "Deploy module transaction\nnetwork: {network_name} ({endpoint_label})\naccount: {account_address}\nmodule reference: {}\nmodule size: {} bytes",
        prepared.module_ref, prepared.module_size,
    ))?;
    Ok(())
}
