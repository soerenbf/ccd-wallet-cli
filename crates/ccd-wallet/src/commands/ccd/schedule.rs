//! Native CCD scheduled-transfer command implementation.

use crate::{
    cli::CcdScheduleArgs,
    commands::{
        ccd::shared::{
            self, PreparedCcdMutationContext, ResolvedScheduledTransfer, confirm_submission,
            parse_memo_option, render_memo_for_review, render_release_schedule, resolve_recipient,
            submit_scheduled_transfer, wait_for_finalization,
        },
        input::{InputMode, ReleaseScheduleEntryInput},
    },
};
use anyhow::{Result, bail};
use cliclack::{confirm, input, log};
use rusqlite::Connection;

/// Run `ccd schedule`.
pub(crate) async fn run(conn: &Connection, args: CcdScheduleArgs) -> Result<()> {
    let input_mode = InputMode::from(&args.input_mode);
    let prepared = PreparedCcdMutationContext {
        sender: args.sender,
        network: args.network_node.network,
        node: args.network_node.node,
        input_mode,
        finalization: args.submission.into(),
    };
    let mut context = shared::resolve_mutation_context(conn, &prepared).await?;
    let recipient = resolve_recipient(conn, &mut context, args.recipient)?;
    let release_inputs = resolve_release_inputs(args.releases, input_mode)?;
    let mut schedule = Vec::with_capacity(release_inputs.len());
    for entry in &release_inputs {
        schedule.push((entry.timestamp()?, entry.amount()?));
    }
    let memo = match args.memo {
        Some(memo) => parse_memo_option(Some(memo))?,
        None if input_mode.prompts_allowed() => shared::prompt_optional_memo()?,
        None => None,
    };
    let resolved = ResolvedScheduledTransfer {
        recipient,
        schedule,
        memo,
    };

    let mut review = vec![
        "CCD scheduled transfer".to_owned(),
        format!(
            "network: {} ({})",
            context.network_name, context.endpoint_label
        ),
        format!("sender: {}", context.sender_address),
        format!("recipient: {}", resolved.recipient),
        "releases:".to_owned(),
    ];
    review.extend(
        render_release_schedule(&resolved.schedule)
            .into_iter()
            .map(|line| format!("  - {line}")),
    );
    if let Some(memo) = &resolved.memo {
        review.push(format!("memo: {}", render_memo_for_review(memo)));
    }
    review.push(format!(
        "finalization: {}",
        if prepared.should_wait_for_finalization() {
            "wait"
        } else {
            "submit only"
        }
    ));
    log::info(review.join("\n"))?;

    if input_mode.prompts_allowed()
        && !confirm_submission(
            "Approve and submit this CCD scheduled transfer?",
            "CCD scheduled transfer declined by user",
        )?
    {
        return Ok(());
    }

    let transaction_hash = submit_scheduled_transfer(conn, &mut context, resolved).await?;
    log::success(format!(
        "Submitted CCD scheduled transfer on {} ({}): {transaction_hash}",
        context.network_name, context.endpoint_label
    ))?;
    if prepared.should_wait_for_finalization() {
        wait_for_finalization(
            &mut context.client,
            &transaction_hash,
            &context.network_name,
            &context.endpoint_label,
        )
        .await?;
    }
    Ok(())
}

fn resolve_release_inputs(
    releases: Vec<ReleaseScheduleEntryInput>,
    input_mode: InputMode,
) -> Result<Vec<ReleaseScheduleEntryInput>> {
    if !releases.is_empty() {
        return Ok(releases);
    }
    if !input_mode.prompts_allowed() {
        bail!("missing required command-line value: release");
    }

    let mut resolved = Vec::new();
    loop {
        let prompt = if resolved.is_empty() {
            "Release entry (RFC3339=CCD):"
        } else {
            "Additional release entry (RFC3339=CCD):"
        };
        let value: String = input(prompt).interact()?;
        let parsed: ReleaseScheduleEntryInput = value
            .parse()
            .map_err(|_| anyhow::anyhow!("--release must use RFC3339=CCD format"))?;
        resolved.push(parsed);
        if !confirm("Add another release entry?")
            .initial_value(false)
            .interact()?
        {
            break;
        }
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_release_inputs_requires_explicit_values_in_non_interactive_mode() {
        let err = resolve_release_inputs(Vec::new(), InputMode::non_interactive()).unwrap_err();

        assert!(err.to_string().contains("release"));
    }

    #[test]
    fn resolve_release_inputs_preserves_explicit_entries() -> Result<()> {
        let releases = vec!["2026-07-01T00:00:00Z=10".parse()?];

        assert_eq!(
            resolve_release_inputs(releases.clone(), InputMode::interactive())?,
            releases
        );
        Ok(())
    }
}
