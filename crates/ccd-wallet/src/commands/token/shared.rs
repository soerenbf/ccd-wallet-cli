//! Shared helpers for protocol-level token commands.

use crate::commands::{
    account::{
        AccountReferenceContext, AccountReferenceUnlocks, build_export_wallet_account_with_unlocks,
        local_account_context_lines, resolve_account_network_context, resolve_account_reference,
        resolve_account_references, resolve_signing_account_context,
    },
    input::{AccountLabel, Defaultable, FinalizationPolicy, InputMode, NetworkName, Promptable},
    transaction::render::render_finalized_summary,
    ui::{
        ContextLine, FuzzySelectItem, SelectItem, fuzzy_multiselect_or_single,
        log_resolved_context, select_always,
    },
};
use anyhow::{Context, Result, bail};
use ccd_wallet_core::config as node_config;
use chrono::{DateTime, SecondsFormat, Utc};
use cliclack::{confirm, input, spinner};
use concordium_rust_sdk::{
    base::{
        common::types::TransactionTime,
        contracts_common::AccountAddress,
        hashes::Hash,
        protocol_level_locks::{
            LockConfig, LockController, LockControllerSimpleV0, LockControllerSimpleV0Capability,
            LockControllerSimpleV0Grant, LockId, LockInfo,
        },
        protocol_level_tokens::{
            MetadataUrl, TokenAdminRole, TokenAmount, TokenId, TokenModuleState,
        },
    },
    protocol_level_tokens::{LockInfoResponse, TokenInfo, token_client::TokenClient},
    types::WalletAccount,
    v2::{self, AccountIdentifier, BlockIdentifier},
};
use rusqlite::Connection;
use std::{collections::HashMap, str::FromStr};

/// Resolved context for a token mutation command.
pub(super) struct MutationContext {
    pub(super) network_name: String,
    pub(super) network_genesis_hash: String,
    pub(super) endpoint_label: String,
    pub(super) client: v2::Client,
    pub(super) wallet: WalletAccount,
    pub(super) account_unlocks: AccountReferenceUnlocks,
}

/// Prepared shared context inputs for a token mutation command.
#[derive(Clone, Debug)]
pub(super) struct PreparedTokenMutationContext {
    account: Promptable<AccountLabel>,
    network: Defaultable<NetworkName>,
    node: Option<v2::Endpoint>,
    input_mode: InputMode,
    finalization: FinalizationPolicy,
    always_prompt_account: bool,
}

impl PreparedTokenMutationContext {
    /// Build prepared token mutation context inputs from common raw command fields.
    pub(super) fn from_raw(
        account: Option<&str>,
        network: Option<&str>,
        node: Option<v2::Endpoint>,
        non_interactive: bool,
        no_defaults: bool,
        no_wait: bool,
        always_prompt_account: bool,
    ) -> Result<Self> {
        Ok(Self {
            account: Promptable::from_option(account.map(str::parse).transpose()?, "account"),
            network: Defaultable::from_option(network.map(str::parse).transpose()?, "network"),
            node,
            input_mode: InputMode::from_flags(non_interactive, no_defaults),
            finalization: FinalizationPolicy::from_no_wait(no_wait),
            always_prompt_account,
        })
    }

    /// Return the shared input mode for resolving command-specific prepared values.
    pub(super) fn input_mode(&self) -> InputMode {
        self.input_mode
    }

    /// Return whether the command should wait for finalization after submission.
    pub(super) fn should_wait_for_finalization(&self) -> bool {
        self.finalization.should_wait()
    }
}

/// Resolve a client for a read-only token or lock query.
pub(super) async fn resolve_query_client(
    conn: &Connection,
    network: Option<&str>,
    node: Option<v2::Endpoint>,
    no_defaults: bool,
) -> Result<(String, String, v2::Client)> {
    let (network_name, _network_entry, endpoint, endpoint_label, network_source) =
        resolve_account_network_context(conn, network, node, false, no_defaults).await?;
    log_resolved_context(&[ContextLine {
        label: "network:",
        value: format!("{network_name} @ {endpoint_label}"),
        source: network_source,
    }])?;
    let client = node_config::connect_v2_client(endpoint.clone())
        .await
        .with_context(|| format!("failed to connect to Concordium node at {endpoint_label}"))?;
    Ok((network_name, endpoint_label, client))
}

/// Resolve the signer wallet, network, and node client for prepared token mutation inputs.
pub(super) async fn resolve_prepared_mutation_context(
    conn: &Connection,
    prepared: &PreparedTokenMutationContext,
) -> Result<MutationContext> {
    let account = match &prepared.account {
        Promptable::Provided(account) => Some(account.as_str()),
        Promptable::Missing { .. } => None,
    };
    let network = match &prepared.network {
        Defaultable::Provided(network) => Some(network.as_str()),
        Defaultable::Missing { .. } => None,
    };
    let (network_context, selection) = resolve_signing_account_context(
        conn,
        account,
        network,
        prepared.node.clone(),
        !prepared.input_mode.prompts_allowed(),
        !prepared.input_mode.defaults_allowed(),
        prepared.always_prompt_account,
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
    let account = selection.record;
    let network_name = network_context.network_name;
    let network_entry = network_context.network_entry;
    let endpoint = network_context.endpoint;
    let endpoint_label = network_context.endpoint_label;
    let mut account_unlocks = AccountReferenceUnlocks::new();
    let wallet = build_export_wallet_account_with_unlocks(
        conn,
        &network_name,
        &network_entry,
        &account,
        &mut account_unlocks,
    )?;
    let client = node_config::connect_v2_client(endpoint.clone())
        .await
        .with_context(|| format!("failed to connect to Concordium node at {endpoint_label}"))?;
    Ok(MutationContext {
        network_name,
        network_genesis_hash: network_entry.genesis_hash,
        endpoint_label,
        client,
        wallet,
        account_unlocks,
    })
}

/// Resolve the signer wallet, network, and node client for a token mutation.
pub(super) async fn resolve_mutation_context(
    conn: &Connection,
    account: Option<&str>,
    network: Option<&str>,
    node: Option<v2::Endpoint>,
    non_interactive: bool,
    no_defaults: bool,
    always_prompt_account: bool,
) -> Result<MutationContext> {
    let prepared = PreparedTokenMutationContext::from_raw(
        account,
        network,
        node,
        non_interactive,
        no_defaults,
        false,
        always_prompt_account,
    )?;
    resolve_prepared_mutation_context(conn, &prepared).await
}

/// Construct a token client for the supplied token.
pub(super) async fn init_token_client(
    client: v2::Client,
    token_id: TokenId,
) -> Result<TokenClient> {
    TokenClient::init_from_token_id(client, token_id)
        .await
        .context("failed to query token information")
}

/// Query token information directly.
pub(super) async fn query_token_info(
    client: &mut v2::Client,
    token_id: TokenId,
) -> Result<TokenInfo> {
    Ok(client
        .get_token_info(token_id, BlockIdentifier::LastFinal)
        .await
        .context("failed to query token information")?
        .response)
}

/// Query lock information directly.
pub(super) async fn query_lock_info(client: &mut v2::Client, lock_id: LockId) -> Result<LockInfo> {
    let response: LockInfoResponse = client
        .get_lock_info(lock_id, BlockIdentifier::LastFinal)
        .await
        .context("failed to query lock information")?
        .response;
    response
        .decode_lock_info()
        .context("failed to decode lock information")
}

/// Select a token identifier from available account balances.
pub(super) fn select_token_from_balances(
    available_balances: &HashMap<TokenId, TokenAmount>,
) -> Result<TokenId> {
    if available_balances.is_empty() {
        bail!("account has no available token balances to transfer");
    }
    let mut items: Vec<SelectItem<TokenId>> = available_balances
        .iter()
        .map(|(token_id, amount)| SelectItem {
            value: token_id.clone(),
            label: token_id.to_string(),
            hint: format!("available: {amount}"),
        })
        .collect();
    items.sort_by_key(|item| item.label.clone());
    select_always("Select token", &items, None)
}

/// Prompt for a token identifier.
pub(super) fn prompt_token_id() -> Result<TokenId> {
    let value: String = input("Token ID:").interact()?;
    value.parse().context("invalid token identifier")
}

/// Prompt for a required string value.
pub(super) fn prompt_required_string(prompt: &str) -> Result<String> {
    Ok(input(prompt).interact()?)
}

/// Resolve token admin roles from explicit CLI input or an interactive multi-select.
pub(super) fn resolve_admin_roles(
    explicit: &[String],
    non_interactive: bool,
) -> Result<Vec<TokenAdminRole>> {
    if !explicit.is_empty() {
        return explicit
            .iter()
            .map(String::as_str)
            .map(parse_token_admin_role)
            .collect();
    }
    if non_interactive {
        bail!("at least one --role must be provided in --non-interactive mode");
    }
    let items = [
        ("update-admin-roles", TokenAdminRole::UpdateAdminRoles),
        ("mint", TokenAdminRole::Mint),
        ("burn", TokenAdminRole::Burn),
        ("update-allow-list", TokenAdminRole::UpdateAllowList),
        ("update-deny-list", TokenAdminRole::UpdateDenyList),
        ("pause", TokenAdminRole::Pause),
        ("update-metadata", TokenAdminRole::UpdateMetadata),
    ];
    let select_items = items
        .iter()
        .map(|(name, role)| FuzzySelectItem {
            value: *role,
            text: (*name).to_owned(),
        })
        .collect::<Vec<_>>();
    fuzzy_multiselect_or_single("Select admin roles", &select_items)
}

/// Resolve one or more target addresses from explicit CLI input or an interactive prompt.
pub(super) fn resolve_target_addresses(
    conn: &Connection,
    context: &mut MutationContext,
    explicit: &[String],
    non_interactive: bool,
) -> Result<Vec<AccountAddress>> {
    if !explicit.is_empty() {
        return resolve_account_references(
            conn,
            AccountReferenceContext {
                network_name: &context.network_name,
                network_genesis_hash: &context.network_genesis_hash,
            },
            explicit,
            "target",
            &mut context.account_unlocks,
        );
    }
    if non_interactive {
        bail!("at least one --target must be provided in --non-interactive mode");
    }
    Ok(vec![resolve_account_reference(
        conn,
        AccountReferenceContext {
            network_name: &context.network_name,
            network_genesis_hash: &context.network_genesis_hash,
        },
        None,
        "Target account address or local label:",
        "target",
        false,
        &mut context.account_unlocks,
    )?])
}

/// Prompt for a lock identifier.
pub(super) fn prompt_lock_id() -> Result<LockId> {
    let value: String = input("Lock ID:").interact()?;
    value.parse().context("invalid lock identifier")
}

/// Select a token configured for a lock.
pub(super) fn select_lock_token(
    lock_info: &LockInfo,
    balance_hints: &HashMap<TokenId, TokenAmount>,
) -> Result<TokenId> {
    let configured = configured_lock_tokens(lock_info);
    if configured.is_empty() {
        bail!("lock '{}' has no configured tokens", lock_info.lock);
    }
    let items = configured
        .iter()
        .map(|token_id| SelectItem {
            value: token_id.clone(),
            label: token_id.to_string(),
            hint: balance_hints
                .get(token_id)
                .map(|amount| format!("balance: {amount}"))
                .unwrap_or_else(|| "balance: unavailable".to_owned()),
        })
        .collect::<Vec<_>>();
    select_always("Select token", &items, None)
}

pub(super) fn configured_lock_tokens(lock_info: &LockInfo) -> Vec<TokenId> {
    match &lock_info.controller {
        LockController::SimpleV0(controller) => controller.tokens.clone(),
    }
}

/// Ensure that a token identifier is configured for the lock.
pub(super) fn ensure_lock_token(lock_info: &LockInfo, token_id: &TokenId) -> Result<()> {
    if configured_lock_tokens(lock_info)
        .iter()
        .any(|candidate| candidate == token_id)
    {
        Ok(())
    } else {
        bail!(
            "token '{}' is not configured for lock '{}'",
            token_id,
            lock_info.lock
        )
    }
}

/// Resolve an amount from explicit input or an interactive prompt with a balance hint.
pub(super) fn resolve_token_amount(
    explicit: Option<&str>,
    decimals: u8,
    balance_hint: Option<TokenAmount>,
    label: &str,
    non_interactive: bool,
) -> Result<TokenAmount> {
    match explicit {
        Some(value) => parse_token_amount(value, decimals),
        None if non_interactive => bail!("--amount is required in --non-interactive mode"),
        None => {
            let prompt = match balance_hint {
                Some(balance) => format!("Amount ({label}: {balance}):"),
                None => "Amount:".to_owned(),
            };
            let value: String = input(prompt).interact()?;
            parse_token_amount(&value, decimals)
        }
    }
}

/// Query available token balances for an account.
pub(super) async fn account_available_balances(
    client: &mut v2::Client,
    account: AccountAddress,
) -> Result<HashMap<TokenId, TokenAmount>> {
    let info = client
        .get_account_info(
            &AccountIdentifier::from(account),
            BlockIdentifier::LastFinal,
        )
        .await
        .context("failed to query account token balances")?
        .response;
    let mut balances = HashMap::new();
    for token in info.tokens {
        let module_state = token
            .state
            .decode_module_state()
            .context("failed to decode token account state")?;
        balances.insert(
            token.token_id,
            module_state.available.unwrap_or(token.state.balance),
        );
    }
    Ok(balances)
}

/// Return locked balances for a source account under a lock.
pub(super) fn locked_balances_for_source(
    lock_info: &LockInfo,
    source: AccountAddress,
) -> HashMap<TokenId, TokenAmount> {
    let mut balances = HashMap::new();
    for funds in &lock_info.funds {
        if funds.account.address == source {
            for amount in &funds.amounts {
                balances.insert(amount.token.clone(), amount.amount);
            }
        }
    }
    balances
}

/// Parse a decimal token amount according to the token's configured decimals.
pub(super) fn parse_token_amount(input: &str, decimals: u8) -> Result<TokenAmount> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        bail!("token amount cannot be empty");
    }
    if trimmed.starts_with('-') {
        bail!("token amount must not be negative");
    }

    let mut parts = trimmed.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next();
    if parts.next().is_some() {
        bail!("invalid token amount '{trimmed}'");
    }
    if whole.is_empty() && fraction.is_none() {
        bail!("invalid token amount '{trimmed}'");
    }
    if !whole.is_empty() && !whole.chars().all(|ch| ch.is_ascii_digit()) {
        bail!("invalid token amount '{trimmed}'");
    }
    let fraction = fraction.unwrap_or_default();
    if !fraction.chars().all(|ch| ch.is_ascii_digit()) {
        bail!("invalid token amount '{trimmed}'");
    }
    let fraction = fraction.trim_end_matches('0');
    if fraction.len() > usize::from(decimals) {
        bail!(
            "token amount '{}' has more fractional digits than the token supports ({})",
            trimmed,
            decimals
        );
    }

    let scale = usize::from(decimals);
    let whole_digits = if whole.is_empty() { "0" } else { whole };
    let raw_text = format!("{whole_digits}{:0<scale$}", fraction, scale = scale);
    let raw = raw_text
        .parse::<u64>()
        .with_context(|| format!("token amount '{trimmed}' is too large"))?;
    Ok(TokenAmount::from_raw(raw, decimals))
}

/// Parse a token admin role from its CLI spelling.
pub(super) fn parse_token_admin_role(input: &str) -> Result<TokenAdminRole> {
    match input {
        "update-admin-roles" | "updateAdminRoles" => Ok(TokenAdminRole::UpdateAdminRoles),
        "mint" => Ok(TokenAdminRole::Mint),
        "burn" => Ok(TokenAdminRole::Burn),
        "update-allow-list" | "updateAllowList" => Ok(TokenAdminRole::UpdateAllowList),
        "update-deny-list" | "updateDenyList" => Ok(TokenAdminRole::UpdateDenyList),
        "pause" => Ok(TokenAdminRole::Pause),
        "update-metadata" | "updateMetadata" => Ok(TokenAdminRole::UpdateMetadata),
        other => bail!("unknown token admin role '{other}'"),
    }
}

/// Parse a metadata checksum from hex.
pub(super) fn parse_checksum_sha_256(input: Option<&str>) -> Result<Option<Hash>> {
    input
        .map(Hash::from_str)
        .transpose()
        .context("invalid SHA-256 checksum hex")
}

/// Build token metadata from CLI inputs.
pub(super) fn build_metadata_url(url: String, checksum: Option<&str>) -> Result<MetadataUrl> {
    Ok(MetadataUrl {
        url,
        checksum_sha_256: parse_checksum_sha_256(checksum)?,
        additional: HashMap::new(),
    })
}

/// Resolve an account address or local account label from explicit input or a prompt.
pub(super) fn resolve_account_address(
    conn: &Connection,
    context: &mut MutationContext,
    explicit: Option<&str>,
    prompt: &str,
    label: &str,
    non_interactive: bool,
) -> Result<AccountAddress> {
    resolve_account_reference(
        conn,
        AccountReferenceContext {
            network_name: &context.network_name,
            network_genesis_hash: &context.network_genesis_hash,
        },
        explicit,
        prompt,
        label,
        non_interactive,
        &mut context.account_unlocks,
    )
}

/// Resolve a list of account addresses or local account labels.
pub(super) fn parse_account_addresses(
    conn: &Connection,
    context: &mut MutationContext,
    values: &[String],
    label: &str,
) -> Result<Vec<AccountAddress>> {
    resolve_account_references(
        conn,
        AccountReferenceContext {
            network_name: &context.network_name,
            network_genesis_hash: &context.network_genesis_hash,
        },
        values,
        label,
        &mut context.account_unlocks,
    )
}

/// Prompt for an approval confirmation.
pub(super) fn confirm_submission(prompt: &str, declined_message: &str) -> Result<bool> {
    let confirmation: String = input(prompt).default_input("n").interact()?;
    if confirmation.eq_ignore_ascii_case("y") || confirmation.eq_ignore_ascii_case("yes") {
        Ok(true)
    } else {
        cliclack::log::warning(declined_message)?;
        Ok(false)
    }
}

/// Wait for transaction finalization and render a summary.
pub(super) async fn wait_for_finalization(
    client: &mut v2::Client,
    transaction_hash: &concordium_rust_sdk::base::hashes::TransactionHash,
    network_name: &str,
    endpoint_label: &str,
) -> Result<()> {
    let spin = spinner();
    spin.start("Waiting for transaction finalization...");
    let (block_hash, summary) = client
        .wait_until_finalized(transaction_hash)
        .await
        .context("failed while waiting for transaction finalization")?;
    spin.clear();
    let block_time = client.get_block_info(block_hash).await.ok().map(|info| {
        info.response
            .block_slot_time
            .to_rfc3339_opts(SecondsFormat::Secs, true)
    });
    println!(
        "{}",
        render_finalized_summary(
            transaction_hash,
            &format!("{network_name} @ {endpoint_label}"),
            &block_hash,
            &summary,
            block_time.as_ref(),
        )?
    );
    Ok(())
}

/// Render a human-readable token summary.
pub(super) fn render_token_info(info: &TokenInfo) -> Result<String> {
    let state = info
        .token_state
        .decode_module_state()
        .context("failed to decode token module state")?;
    let mut lines = vec![
        format!("Token ID: {}", info.token_id),
        format!("Token module: {}", info.token_state.token_module_ref),
        format!("Decimals: {}", info.token_state.decimals),
        format!("Total supply: {}", info.token_state.total_supply),
        String::new(),
        "Module state:".to_owned(),
    ];
    lines.extend(render_token_module_state_lines(&state)?);
    Ok(lines.join("\n"))
}

fn render_token_module_state_lines(state: &TokenModuleState) -> Result<Vec<String>> {
    let lines = vec![
        format_optional_string("Name", state.name.as_deref()),
        format_metadata_url(state.metadata.as_ref())?,
        format_optional_account("Governance account", state.governance_account.as_ref()),
        format_optional_flag("Allow list", state.allow_list),
        format_optional_flag("Deny list", state.deny_list),
        format_optional_flag("Mintable", state.mintable),
        format_optional_flag("Burnable", state.burnable),
        format_optional_flag("Paused", state.paused),
    ];
    Ok(lines)
}

fn format_optional_string(label: &str, value: Option<&str>) -> String {
    match value {
        Some(value) => format!("  {label}: {value}"),
        None => format!("  {label}: none"),
    }
}

fn format_metadata_url(metadata: Option<&MetadataUrl>) -> Result<String> {
    Ok(match metadata {
        Some(metadata) => {
            let checksum = metadata
                .checksum_sha_256
                .as_ref()
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_owned());
            let mut line = format!("  Metadata: {} (checksum: {})", metadata.url, checksum);
            if !metadata.additional.is_empty() {
                line.push_str(&format!(", additional: {:?}", metadata.additional));
            }
            line
        }
        None => "  Metadata: none".to_owned(),
    })
}

fn format_optional_account(
    label: &str,
    value: Option<&concordium_rust_sdk::protocol_level_tokens::CborHolderAccount>,
) -> String {
    match value {
        Some(value) => format!("  {label}: {}", value.address),
        None => format!("  {label}: none"),
    }
}

fn format_optional_flag(label: &str, value: Option<bool>) -> String {
    match value {
        Some(true) => format!("  {label}: enabled"),
        Some(false) => format!("  {label}: disabled"),
        None => format!("  {label}: unsupported"),
    }
}

/// Render a human-readable lock summary.
pub(super) fn render_lock_info(info: &LockInfo) -> Result<String> {
    let mut lines = vec![
        format!("Lock ID: {}", info.lock),
        format!("Expiry: {}", format_transaction_time(info.expiry)?),
        String::new(),
        "Recipients:".to_owned(),
    ];
    if info.recipients.is_empty() {
        lines.push("  none".to_owned());
    } else {
        for recipient in &info.recipients {
            lines.push(format!("  - {}", recipient.address));
        }
    }

    lines.push(String::new());
    lines.push("Controller:".to_owned());
    match &info.controller {
        LockController::SimpleV0(controller) => {
            lines.push("  Type: simple-v0".to_owned());
            lines.push(format!("  Keep alive: {}", yes_no(controller.keep_alive)));
            lines.push(format!(
                "  Tokens: {}",
                controller
                    .tokens
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            if let Some(memo) = &controller.memo {
                lines.push(format!("  Memo: {:?}", memo));
            }
            lines.push("  Grants:".to_owned());
            if controller.grants.is_empty() {
                lines.push("    none".to_owned());
            } else {
                for grant in &controller.grants {
                    let roles = grant
                        .roles
                        .iter()
                        .map(lock_capability_name)
                        .collect::<Vec<_>>()
                        .join(", ");
                    lines.push(format!("    - {}: {}", grant.account.address, roles));
                }
            }
        }
    }

    lines.push(String::new());
    lines.push("Funds:".to_owned());
    if info.funds.is_empty() {
        lines.push("  none".to_owned());
    } else {
        for funds in &info.funds {
            lines.push(format!("  - Account: {}", funds.account.address));
            if funds.amounts.is_empty() {
                lines.push("    Amounts: none".to_owned());
            } else {
                for amount in &funds.amounts {
                    lines.push(format!("    {}: {}", amount.token, amount.amount));
                }
            }
        }
    }

    Ok(lines.join("\n"))
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn lock_capability_name(value: &LockControllerSimpleV0Capability) -> &'static str {
    match value {
        LockControllerSimpleV0Capability::Fund => "fund",
        LockControllerSimpleV0Capability::Return => "return",
        LockControllerSimpleV0Capability::Send => "send",
        LockControllerSimpleV0Capability::Cancel => "cancel",
    }
}

fn format_transaction_time(value: TransactionTime) -> Result<String> {
    let seconds = i64::try_from(value.seconds).context("timestamp is too large to display")?;
    let datetime = DateTime::<Utc>::from_timestamp(seconds, 0)
        .with_context(|| format!("timestamp {seconds} cannot be formatted"))?;
    Ok(datetime.to_rfc3339_opts(SecondsFormat::Secs, true))
}

/// A lock grant before account labels have been resolved to addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct UnresolvedLockGrant {
    /// Raw account address or finalized local account label.
    pub(super) account: String,
    /// Capabilities granted to the account.
    pub(super) roles: Vec<LockControllerSimpleV0Capability>,
}

/// Return the user-facing spelling for a lock capability.
pub(super) fn lock_capability_cli_name(value: LockControllerSimpleV0Capability) -> &'static str {
    match value {
        LockControllerSimpleV0Capability::Fund => "fund",
        LockControllerSimpleV0Capability::Return => "return",
        LockControllerSimpleV0Capability::Send => "send",
        LockControllerSimpleV0Capability::Cancel => "cancel",
    }
}

/// Parse an unresolved lock create grant in the form `<ACCOUNT:ROLE[,ROLE...]>`.
pub(super) fn parse_unresolved_lock_grant(input: &str) -> Result<UnresolvedLockGrant> {
    let (account, roles) = input
        .split_once(':')
        .with_context(|| format!("invalid grant '{input}'; expected <ACCOUNT:ROLE[,ROLE...]>"))?;
    let account = account.trim();
    if account.is_empty() {
        bail!("grant '{input}' must include an account before ':'");
    }
    let roles = parse_lock_grant_roles(input, roles)?;
    Ok(UnresolvedLockGrant {
        account: account.to_owned(),
        roles,
    })
}

/// Resolve an unresolved lock grant against local account labels in the current network context.
pub(super) fn resolve_lock_grant(
    conn: &Connection,
    context: &mut MutationContext,
    grant: &UnresolvedLockGrant,
) -> Result<LockControllerSimpleV0Grant> {
    let account = resolve_account_reference(
        conn,
        AccountReferenceContext {
            network_name: &context.network_name,
            network_genesis_hash: &context.network_genesis_hash,
        },
        Some(&grant.account),
        "Grant account address or local label:",
        "grant",
        true,
        &mut context.account_unlocks,
    )?;
    Ok(LockControllerSimpleV0Grant {
        account: concordium_rust_sdk::protocol_level_tokens::CborHolderAccount::from(account),
        roles: grant.roles.clone(),
    })
}

/// Prompt for one or more unresolved lock grants.
pub(super) fn prompt_unresolved_lock_grants() -> Result<Vec<UnresolvedLockGrant>> {
    let mut grants = Vec::new();
    loop {
        let account: String = input("Grant account address or local label:").interact()?;
        let roles = prompt_lock_capabilities()?;
        grants.push(UnresolvedLockGrant { account, roles });
        let another = confirm("Add another grant?")
            .initial_value(false)
            .interact()?;
        if !another {
            break;
        }
    }
    Ok(grants)
}

fn prompt_lock_capabilities() -> Result<Vec<LockControllerSimpleV0Capability>> {
    let items = lock_capability_items();
    fuzzy_multiselect_or_single("Select grant capabilities", &items)
}

fn lock_capability_items() -> Vec<FuzzySelectItem<LockControllerSimpleV0Capability>> {
    [
        LockControllerSimpleV0Capability::Fund,
        LockControllerSimpleV0Capability::Send,
        LockControllerSimpleV0Capability::Return,
        LockControllerSimpleV0Capability::Cancel,
    ]
    .into_iter()
    .map(|capability| FuzzySelectItem {
        text: lock_capability_cli_name(capability.clone()).to_owned(),
        value: capability,
    })
    .collect()
}

pub(super) fn parse_lock_grant_roles(
    input: &str,
    roles: &str,
) -> Result<Vec<LockControllerSimpleV0Capability>> {
    let roles = roles
        .split(',')
        .map(str::trim)
        .filter(|role| !role.is_empty())
        .map(parse_lock_capability)
        .collect::<Result<Vec<_>>>()?;
    if roles.is_empty() {
        bail!("grant '{input}' must include at least one capability");
    }
    Ok(roles)
}

pub(super) fn parse_lock_capability(input: &str) -> Result<LockControllerSimpleV0Capability> {
    match input {
        "fund" => Ok(LockControllerSimpleV0Capability::Fund),
        "return" => Ok(LockControllerSimpleV0Capability::Return),
        "send" => Ok(LockControllerSimpleV0Capability::Send),
        "cancel" => Ok(LockControllerSimpleV0Capability::Cancel),
        other => {
            bail!("unknown lock capability '{other}'; expected one of: fund, send, return, cancel")
        }
    }
}

/// Resolve whether a lock should be kept alive after funds are returned.
pub(super) fn resolve_lock_keep_alive(
    explicit_keep_alive: bool,
    non_interactive: bool,
) -> Result<bool> {
    if explicit_keep_alive || non_interactive {
        return Ok(explicit_keep_alive);
    }
    Ok(confirm("Keep the lock alive after funds are returned?")
        .initial_value(false)
        .interact()?)
}

/// Build a simple-v0 lock configuration from CLI inputs.
pub(super) fn build_lock_config(
    recipients: Vec<AccountAddress>,
    expiry: TransactionTime,
    grants: Vec<LockControllerSimpleV0Grant>,
    tokens: Vec<TokenId>,
    keep_alive: bool,
) -> LockConfig {
    LockConfig {
        recipients: recipients
            .into_iter()
            .map(concordium_rust_sdk::protocol_level_tokens::CborHolderAccount::from)
            .collect(),
        expiry,
        controller: LockController::SimpleV0(LockControllerSimpleV0 {
            grants,
            tokens,
            keep_alive,
            memo: None,
        }),
    }
}

/// Parse a lock expiry time input.
pub(super) fn parse_expiry_time(input: &str) -> Result<TransactionTime> {
    let seconds = parse_time_input(input, now_unix_seconds()?)?;
    Ok(TransactionTime::from_seconds(seconds))
}

fn parse_time_input(input: &str, now: u64) -> Result<u64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        bail!("time input cannot be empty");
    }
    if let Some(duration_seconds) = parse_relative_duration_seconds(trimmed)? {
        return now
            .checked_add(duration_seconds)
            .with_context(|| "relative time overflows unix seconds");
    }
    if let Ok(datetime) = DateTime::parse_from_rfc3339(trimmed) {
        let timestamp = datetime.with_timezone(&Utc).timestamp();
        if timestamp < 0 {
            bail!("time input must not be before the unix epoch");
        }
        return Ok(timestamp as u64);
    }
    trimmed.parse::<u64>().with_context(|| {
        format!("invalid time input '{trimmed}'; use relative duration, RFC3339, or unix seconds")
    })
}

fn parse_relative_duration_seconds(input: &str) -> Result<Option<u64>> {
    let Some(unit) = input.chars().last() else {
        return Ok(None);
    };
    let multiplier = match unit {
        's' | 'S' => 1,
        'm' | 'M' => 60,
        'h' | 'H' => 60 * 60,
        'd' | 'D' => 24 * 60 * 60,
        _ => return Ok(None),
    };
    let number = &input[..input.len() - unit.len_utf8()];
    if number.trim().is_empty() {
        bail!("relative duration '{input}' is missing a number");
    }
    let value = number
        .trim()
        .parse::<u64>()
        .with_context(|| format!("invalid relative duration '{input}'"))?;
    value
        .checked_mul(multiplier)
        .map(Some)
        .with_context(|| format!("relative duration '{input}' overflows seconds"))
}

fn now_unix_seconds() -> Result<u64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .context("system clock is before the unix epoch")?
        .as_secs())
}

#[cfg(test)]
mod tests {
    use super::{
        parse_lock_grant_roles, parse_token_admin_role, parse_token_amount,
        parse_unresolved_lock_grant,
    };
    use concordium_rust_sdk::base::protocol_level_locks::LockControllerSimpleV0Capability;
    use concordium_rust_sdk::base::protocol_level_tokens::TokenAdminRole;

    #[test]
    fn parses_decimal_token_amount_with_padding() {
        let amount = parse_token_amount("1.25", 4).unwrap();
        assert_eq!(amount.to_string(), "1.2500");
    }

    #[test]
    fn rejects_too_many_fractional_digits() {
        let err = parse_token_amount("1.234", 2).unwrap_err();
        assert!(err.to_string().contains("more fractional digits"));
    }

    #[test]
    fn parses_protocol_near_admin_role_names() {
        assert_eq!(
            parse_token_admin_role("update-admin-roles").unwrap(),
            TokenAdminRole::UpdateAdminRoles
        );
        assert_eq!(
            parse_token_admin_role("update-metadata").unwrap(),
            TokenAdminRole::UpdateMetadata
        );
    }

    #[test]
    fn parses_lock_grant_roles() {
        let roles = parse_lock_grant_roles("account:fund,send", "fund,send").unwrap();
        assert_eq!(roles.len(), 2);
        assert_eq!(roles[0], LockControllerSimpleV0Capability::Fund);
        assert_eq!(roles[1], LockControllerSimpleV0Capability::Send);
    }

    #[test]
    fn parses_unresolved_lock_grant() {
        let grant = parse_unresolved_lock_grant("alice:fund,send").unwrap();
        assert_eq!(grant.account, "alice");
        assert_eq!(grant.roles.len(), 2);
        assert_eq!(grant.roles[0], LockControllerSimpleV0Capability::Fund);
        assert_eq!(grant.roles[1], LockControllerSimpleV0Capability::Send);
    }

    #[test]
    fn rejects_unknown_lock_capability() {
        let err = parse_unresolved_lock_grant("alice:fund,nonsense").unwrap_err();
        assert!(err.to_string().contains("nonsense"));
    }
}
