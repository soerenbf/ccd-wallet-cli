//! Native CCD transfer command implementation.

use crate::{
    cli::CcdTransferArgs,
    commands::{
        ccd::shared::{
            self, PreparedCcdMutationContext, ResolvedTransfer, confirm_submission,
            parse_memo_option, prompt_amount, render_memo_for_review, resolve_recipient,
            submit_transfer, wait_for_finalization,
        },
        input::InputMode,
    },
};
use anyhow::{Result, bail};
use cliclack::log;
use rusqlite::Connection;

/// Run `ccd transfer`.
pub(crate) async fn run(conn: &Connection, args: CcdTransferArgs) -> Result<()> {
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
    let amount = resolve_amount(args.amount, input_mode)?;
    let memo = match args.memo {
        Some(memo) => parse_memo_option(Some(memo))?,
        None if input_mode.prompts_allowed() => shared::prompt_optional_memo()?,
        None => None,
    };
    let resolved = ResolvedTransfer {
        recipient,
        amount,
        memo,
    };

    let mut review = vec![
        "CCD transfer".to_owned(),
        format!(
            "network: {} ({})",
            context.network_name, context.endpoint_label
        ),
        format!("sender: {}", context.sender_address),
        format!("recipient: {}", resolved.recipient),
        format!("amount: {}", resolved.amount),
    ];
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
            "Approve and submit this CCD transfer?",
            "CCD transfer declined by user",
        )?
    {
        return Ok(());
    }

    let transaction_hash = submit_transfer(conn, &mut context, resolved).await?;
    log::success(format!(
        "Submitted CCD transfer on {} ({}): {transaction_hash}",
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

fn resolve_amount(
    amount: Option<concordium_rust_sdk::common::types::Amount>,
    input_mode: InputMode,
) -> Result<concordium_rust_sdk::common::types::Amount> {
    match amount {
        Some(amount) => Ok(amount),
        None if input_mode.prompts_allowed() => prompt_amount("CCD amount:"),
        None => bail!("missing required command-line value: amount"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use concordium_rust_sdk::common::types::Amount;

    #[test]
    fn resolve_amount_requires_explicit_value_in_non_interactive_mode() {
        let err = resolve_amount(None, InputMode::non_interactive()).unwrap_err();

        assert!(err.to_string().contains("amount"));
    }

    #[test]
    fn resolve_amount_preserves_explicit_value() -> Result<()> {
        let amount = Amount::from_micro_ccd(12_500_000);

        assert_eq!(
            resolve_amount(Some(amount), InputMode::interactive())?,
            amount
        );
        Ok(())
    }
}
