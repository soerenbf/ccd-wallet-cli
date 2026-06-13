//! Deploy-module smart contract command.

use crate::commands::transaction::render::render_finalized_summary;
use crate::{
    cli::ContractDeployModuleArgs,
    commands::{
        account::{
            build_export_wallet_account, local_account_context_lines,
            resolve_signing_account_context,
        },
        ui::{ContextLine, log_resolved_context},
    },
    smart_contracts::deploy_module as deploy_core,
};
use anyhow::{Context, Result};
use ccd_wallet_core::config as node_config;
use cliclack::{input, spinner};
use rusqlite::Connection;
use std::fs;

pub(super) async fn deploy_module(conn: &Connection, args: ContractDeployModuleArgs) -> Result<()> {
    let module_bytes = fs::read(&args.file)
        .with_context(|| format!("failed to read module file {}", args.file.display()))?;
    let (network_context, selection) = resolve_signing_account_context(
        conn,
        args.account.as_deref(),
        args.network.as_deref(),
        args.node,
        args.non_interactive,
        args.no_defaults,
        false,
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
    lines.extend(local_account_context_lines(
        conn,
        &selection.record,
        selection.source,
    )?);
    log_resolved_context(&lines)?;
    let network_name = network_context.network_name;
    let network_entry = network_context.network_entry;
    let endpoint = network_context.endpoint;
    let endpoint_label = network_context.endpoint_label;
    let account = selection.record;
    let wallet = build_export_wallet_account(conn, &network_name, &network_entry, &account)?;
    let prepared = deploy_core::prepare_deploy_module(wallet.address, &module_bytes)?;

    let mut client = node_config::connect_v2_client(endpoint.clone())
        .await
        .with_context(|| format!("failed to connect to Concordium node at {endpoint_label}"))?;

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
        &network_name,
        &endpoint_label,
        &wallet.address.to_string(),
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
    let submitted = deploy_core::submit_deploy_module(&mut client, &wallet, prepared).await?;
    spin.clear();
    let transaction_hash_label = submitted.transaction_hash.to_string();
    cliclack::log::success(format!(
        "Submitted deploy-module transaction on {network_name} ({endpoint_label}): {transaction_hash_label}"
    ))?;

    if args.no_wait {
        return Ok(());
    }

    let spin = spinner();
    spin.start("Waiting for deploy-module finalization...");
    let finalized =
        deploy_core::wait_for_deploy_module_finalization(&mut client, submitted).await?;
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
            &format!("{network_name} @ {endpoint_label}"),
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
