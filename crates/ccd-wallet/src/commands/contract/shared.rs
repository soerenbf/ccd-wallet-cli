//! Shared prepared-input helpers for contract submission commands.

use crate::commands::{
    account::{
        build_export_wallet_account, local_account_context_lines, resolve_signing_account_context,
    },
    input::{AccountLabel, FinalizationPolicy, InputMode, NetworkName, Promptable},
    ui::{ContextLine, log_resolved_context},
};
use anyhow::Result;
use ccd_wallet_core::{config as node_config, store::accounts};
use concordium_rust_sdk::{types::WalletAccount, v2};
use rusqlite::Connection;

/// Prepared common signing and submission inputs for contract-submitting commands.
#[derive(Clone, Debug)]
pub(super) struct PreparedContractSubmission {
    account: Promptable<AccountLabel>,
    network: Option<NetworkName>,
    node: Option<v2::Endpoint>,
    input_mode: InputMode,
    finalization: FinalizationPolicy,
}

impl PreparedContractSubmission {
    /// Build prepared contract submission inputs from raw clap fields.
    pub(super) fn from_raw(
        account: Option<&str>,
        network: Option<&str>,
        node: Option<v2::Endpoint>,
        non_interactive: bool,
        no_defaults: bool,
        no_wait: bool,
    ) -> Result<Self> {
        Ok(Self {
            account: Promptable::from_option(account.map(str::parse).transpose()?, "account"),
            network: network.map(str::parse).transpose()?,
            node,
            input_mode: InputMode::from_flags(non_interactive, no_defaults),
            finalization: FinalizationPolicy::from_no_wait(no_wait),
        })
    }

    /// Return the input mode for command-specific promptable values.
    pub(super) fn input_mode(&self) -> InputMode {
        self.input_mode
    }

    /// Return whether the command should wait for transaction finalization.
    pub(super) fn should_wait_for_finalization(&self) -> bool {
        self.finalization.should_wait()
    }
}

/// Resolved signing context for contract submission commands.
pub(super) struct ContractSubmissionContext {
    pub(super) network_name: String,
    pub(super) endpoint_label: String,
    pub(super) client: v2::Client,
    pub(super) wallet: WalletAccount,
}

/// Resolve prepared contract submission inputs into network, client, and wallet context.
pub(super) async fn resolve_prepared_submission_context(
    conn: &Connection,
    prepared: &PreparedContractSubmission,
) -> Result<ContractSubmissionContext> {
    let account = match &prepared.account {
        Promptable::Provided(account) => Some(account.as_str()),
        Promptable::Missing { .. } => None,
    };
    let (network_context, selection) = resolve_signing_account_context(
        conn,
        account,
        prepared.network.as_ref().map(|network| network.as_str()),
        prepared.node.clone(),
        !prepared.input_mode.prompts_allowed(),
        !prepared.input_mode.defaults_allowed(),
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
    let account: accounts::AccountRecord = selection.record;
    let network_name = network_context.network_name;
    let network_entry = network_context.network_entry;
    let endpoint = network_context.endpoint;
    let endpoint_label = network_context.endpoint_label;
    let wallet = build_export_wallet_account(conn, &network_name, &network_entry, &account)?;
    let client = node_config::connect_v2_client(endpoint.clone()).await?;
    Ok(ContractSubmissionContext {
        network_name,
        endpoint_label,
        client,
        wallet,
    })
}
