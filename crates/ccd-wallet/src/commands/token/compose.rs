//! Interactive protocol-level token composition planning and submission.

use crate::{
    cli::{TokenComposeArgs, TokenComposePreviewArgs, TokenComposeSubmitArgs},
    commands::{
        account::{
            AccountReferenceContext, AccountReferenceUnlocks, resolve_account_network_context,
            resolve_account_reference,
        },
        input::{AccountLabel, FinalizationPolicy, InputMode, NetworkName, Promptable},
        token::shared,
        ui::{SelectItem, select_always},
    },
};
use anyhow::{Context, Result, bail, ensure};
use ccd_wallet_core::{config as node_config, store::config as app_config};
use cliclack::{confirm, input};
use concordium_rust_sdk::{
    base::{
        common::types::TransactionTime,
        contracts_common::AccountAddress,
        protocol_level_locks::{LockId, LockRecipients},
        protocol_level_tokens::{
            TokenAmount, TokenId, meta_operations, meta_operations::MetaUpdateOperations,
        },
        transactions::{BlockItem, ExactSizeTransactionSigner, construct},
    },
    protocol_level_tokens::lock_client,
};
use reedline::{
    Completer, DefaultPrompt, EditCommand, Emacs, IdeMenu, KeyCode, KeyModifiers, Keybindings,
    MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu, Signal, Span, Suggestion,
    default_emacs_keybindings,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, fmt, fs, path::Path, str::FromStr};

const PLAN_VERSION: u32 = 1;
const DEFAULT_EXPIRY_SECS: u32 = 300;

/// Run the interactive token composer for a plan file.
pub(super) async fn compose(conn: &Connection, args: TokenComposeArgs) -> Result<()> {
    match args.command {
        Some(crate::cli::TokenComposeSubcommand::Preview(args)) => preview(*args).await,
        Some(crate::cli::TokenComposeSubcommand::Submit(args)) => submit(conn, *args).await,
        None => {
            let plan_path = args
                .plan
                .as_deref()
                .context("token compose requires a plan path")?;
            run_composer(conn, plan_path).await
        }
    }
}

/// Render the operations recorded in a token composition plan.
pub(super) async fn preview(args: TokenComposePreviewArgs) -> Result<()> {
    let plan = read_plan(&args.plan)?;
    println!("{}", render_plan_preview(&plan, Some(args.plan.as_path()))?);
    Ok(())
}

#[derive(Clone, Debug)]
struct PreparedTokenComposeSubmit {
    plan: std::path::PathBuf,
    sender: Option<AccountLabel>,
    network: Option<NetworkName>,
    node: Option<concordium_rust_sdk::v2::Endpoint>,
    input_mode: InputMode,
    finalization: FinalizationPolicy,
}

impl PreparedTokenComposeSubmit {
    fn from_cli(args: TokenComposeSubmitArgs) -> Result<Self> {
        Self::from_parts(
            args.plan,
            args.sender,
            args.network,
            args.node,
            args.non_interactive,
            args.no_defaults,
            args.no_wait,
        )
    }

    fn from_repl(plan: std::path::PathBuf, args: SubmitCommandArgs) -> Result<Self> {
        Self::from_parts(
            plan,
            args.sender,
            args.network,
            args.node,
            args.non_interactive,
            args.no_defaults,
            args.no_wait,
        )
    }

    fn from_parts(
        plan: std::path::PathBuf,
        sender: Option<String>,
        network: Option<String>,
        node: Option<concordium_rust_sdk::v2::Endpoint>,
        non_interactive: bool,
        no_defaults: bool,
        no_wait: bool,
    ) -> Result<Self> {
        let input_mode = InputMode::from_flags(non_interactive, no_defaults);
        if !input_mode.prompts_allowed() && sender.is_none() {
            bail!("sender must be provided with --sender in --non-interactive mode");
        }
        Ok(Self {
            plan,
            sender: sender.map(|sender| sender.parse()).transpose()?,
            network: network.map(|network| network.parse()).transpose()?,
            node,
            input_mode,
            finalization: FinalizationPolicy::from_no_wait(no_wait),
        })
    }

    fn plan_path(&self) -> &Path {
        &self.plan
    }
}

/// Submit a token composition plan.
pub(super) async fn submit(conn: &Connection, args: TokenComposeSubmitArgs) -> Result<()> {
    let prepared = PreparedTokenComposeSubmit::from_cli(args)?;
    submit_prepared(conn, prepared).await
}

async fn submit_prepared(conn: &Connection, prepared: PreparedTokenComposeSubmit) -> Result<()> {
    let plan = read_plan(prepared.plan_path())?;
    validate_lock_references(&plan)?;
    if plan.operations.is_empty() {
        bail!("token composition plan has no operations to submit");
    }

    let inferred_network = if prepared.network.is_none() && prepared.node.is_none() {
        Some(plan_network_name(&plan)?.parse()?)
    } else {
        None
    };
    let mutation_context = shared::PreparedTokenMutationContext::from_parts(
        prepared.sender.clone(),
        prepared.network.clone().or(inferred_network),
        prepared.node.clone(),
        prepared.input_mode,
        prepared.finalization,
        true,
    );
    let mut context = shared::resolve_prepared_mutation_context(conn, &mutation_context).await?;

    validate_plan_network_matches_context(&plan, &context)?;
    let operations = resolve_meta_update_operations(conn, &mut context, &plan).await?;
    let summary = render_plan_preview(&plan, Some(prepared.plan_path()))?;
    cliclack::log::info(format!(
        "Token composition transaction\nnetwork: {} ({})\naccount: {}\n\n{}",
        context.network_name, context.endpoint_label, context.wallet.address, summary
    ))?;
    if !shared::confirm_submission(
        "Approve and submit this token composition? Type y to approve:",
        "token composition declined by user",
    )? {
        return Ok(());
    }

    let spin = cliclack::spinner();
    spin.start("Submitting token composition...");
    let transaction_hash = sign_and_send_meta_update(&mut context, &operations).await?;
    spin.clear();
    cliclack::log::success(format!(
        "Submitted token composition on {} ({}): {transaction_hash}",
        context.network_name, context.endpoint_label
    ))?;
    if prepared.finalization.should_wait() {
        shared::wait_for_finalization(
            &mut context.client,
            &transaction_hash,
            &context.network_name,
            &context.endpoint_label,
        )
        .await?;
    }
    Ok(())
}

fn plan_network_name(plan: &Plan) -> Result<String> {
    let genesis_hash = plan
        .network_genesis_hash
        .as_deref()
        .context("token composition plan is missing network genesis hash")?;
    let app_config = app_config::load()?;
    let mut matches = app_config
        .networks
        .iter()
        .filter(|(_, entry)| entry.genesis_hash == genesis_hash)
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    matches.sort();
    matches.into_iter().next().with_context(|| {
        format!("no configured network matches token composition genesis hash {genesis_hash}")
    })
}

fn validate_plan_network_matches_context(
    plan: &Plan,
    context: &shared::MutationContext,
) -> Result<()> {
    let Some(plan_genesis_hash) = plan.network_genesis_hash.as_deref() else {
        bail!(
            "token composition plan is missing network genesis hash; reopen it with `token compose <PLAN>` and add or re-add an operation"
        );
    };
    if plan_genesis_hash != context.network_genesis_hash {
        bail!(
            "token composition plan is for genesis hash {}, but selected network '{}' has genesis hash {}",
            plan_genesis_hash,
            context.network_name,
            context.network_genesis_hash
        );
    }
    Ok(())
}

async fn resolve_meta_update_operations(
    conn: &Connection,
    context: &mut shared::MutationContext,
    plan: &Plan,
) -> Result<MetaUpdateOperations> {
    let predicted_locks = predict_created_locks(context, plan).await?;
    let mut operations = Vec::with_capacity(plan.operations.len());
    for operation in &plan.operations {
        match operation {
            PlanOperation::Transfer {
                token,
                recipient,
                amount,
            } => {
                let token_id = parse_token_id(token)?;
                let amount = resolve_amount(context, token_id.clone(), amount).await?;
                let recipient = resolve_account(conn, context, recipient, "recipient")?;
                operations.push(meta_operations::transfer_tokens(
                    token_id, recipient, amount,
                ));
            }
            PlanOperation::Mint { token, amount } => {
                let token_id = parse_token_id(token)?;
                let amount = resolve_amount(context, token_id.clone(), amount).await?;
                operations.push(meta_operations::mint_tokens(token_id, amount));
            }
            PlanOperation::Burn { token, amount } => {
                let token_id = parse_token_id(token)?;
                let amount = resolve_amount(context, token_id.clone(), amount).await?;
                operations.push(meta_operations::burn_tokens(token_id, amount));
            }
            PlanOperation::Pause { token } => {
                operations.push(meta_operations::pause(parse_token_id(token)?));
            }
            PlanOperation::Unpause { token } => {
                operations.push(meta_operations::unpause(parse_token_id(token)?));
            }
            PlanOperation::AllowListAdd { token, targets } => {
                let token_id = parse_token_id(token)?;
                for target in resolve_accounts(conn, context, targets, "target")? {
                    operations.push(meta_operations::add_token_allow_list(
                        token_id.clone(),
                        target,
                    ));
                }
            }
            PlanOperation::AllowListRemove { token, targets } => {
                let token_id = parse_token_id(token)?;
                for target in resolve_accounts(conn, context, targets, "target")? {
                    operations.push(meta_operations::remove_token_allow_list(
                        token_id.clone(),
                        target,
                    ));
                }
            }
            PlanOperation::DenyListAdd { token, targets } => {
                let token_id = parse_token_id(token)?;
                for target in resolve_accounts(conn, context, targets, "target")? {
                    operations.push(meta_operations::add_token_deny_list(
                        token_id.clone(),
                        target,
                    ));
                }
            }
            PlanOperation::DenyListRemove { token, targets } => {
                let token_id = parse_token_id(token)?;
                for target in resolve_accounts(conn, context, targets, "target")? {
                    operations.push(meta_operations::remove_token_deny_list(
                        token_id.clone(),
                        target,
                    ));
                }
            }
            PlanOperation::AdminRolesAssign {
                token,
                target,
                roles,
            } => {
                let token_id = parse_token_id(token)?;
                let target = resolve_account(conn, context, target, "target")?;
                let roles = parse_admin_roles(roles)?;
                operations.push(meta_operations::assign_admin_roles(token_id, target, roles));
            }
            PlanOperation::AdminRolesRevoke {
                token,
                target,
                roles,
            } => {
                let token_id = parse_token_id(token)?;
                let target = resolve_account(conn, context, target, "target")?;
                let roles = parse_admin_roles(roles)?;
                operations.push(meta_operations::revoke_admin_roles(token_id, target, roles));
            }
            PlanOperation::MetadataUpdate {
                token,
                url,
                checksum_sha256,
            } => {
                let token_id = parse_token_id(token)?;
                let metadata = shared::build_metadata_url(url.clone(), checksum_sha256.as_deref())?;
                operations.push(meta_operations::update_metadata(token_id, metadata));
            }
            PlanOperation::LockCreate {
                recipients,
                expiry,
                grants,
                tokens,
                keep_alive,
            } => {
                let recipients = recipients.resolve_accounts(conn, context)?;
                let expiry = shared::parse_expiry_time(expiry)?;
                let grants = grants
                    .iter()
                    .map(|grant| resolve_plan_lock_grant(conn, context, grant))
                    .collect::<Result<Vec<_>>>()?;
                let tokens = tokens
                    .iter()
                    .map(|token| parse_token_id(token))
                    .collect::<Result<Vec<_>>>()?;
                let config =
                    shared::build_lock_config(recipients, expiry, grants, tokens, *keep_alive);
                operations.push(meta_operations::lock_create(config));
            }
            PlanOperation::LockFund {
                lock,
                token,
                amount,
            } => {
                let token_id = parse_token_id(token)?;
                let lock_id = resolve_lock_reference(lock, &predicted_locks)?;
                let amount = resolve_amount(context, token_id.clone(), amount).await?;
                operations.push(meta_operations::lock_fund(token_id, lock_id, amount, None));
            }
            PlanOperation::LockSend {
                lock,
                token,
                source,
                recipient,
                amount,
            } => {
                let token_id = parse_token_id(token)?;
                let lock_id = resolve_lock_reference(lock, &predicted_locks)?;
                let source = resolve_account(conn, context, source, "source")?;
                let recipient = resolve_account(conn, context, recipient, "recipient")?;
                let amount = resolve_amount(context, token_id.clone(), amount).await?;
                operations.push(meta_operations::lock_send(
                    token_id, lock_id, source, recipient, amount, None,
                ));
            }
            PlanOperation::LockReturn {
                lock,
                token,
                source,
                amount,
            } => {
                let token_id = parse_token_id(token)?;
                let lock_id = resolve_lock_reference(lock, &predicted_locks)?;
                let source = resolve_account(conn, context, source, "source")?;
                let amount = resolve_amount(context, token_id.clone(), amount).await?;
                operations.push(meta_operations::lock_return(
                    token_id, lock_id, source, amount, None,
                ));
            }
            PlanOperation::LockCancel { lock } => {
                let lock_id = resolve_lock_reference(lock, &predicted_locks)?;
                operations.push(meta_operations::lock_cancel(lock_id, None));
            }
        }
    }
    Ok(MetaUpdateOperations::new(operations))
}

async fn predict_created_locks(
    context: &mut shared::MutationContext,
    plan: &Plan,
) -> Result<Vec<LockId>> {
    let count = plan
        .operations
        .iter()
        .filter(|operation| operation.is_lock_create())
        .count();
    let mut locks = Vec::with_capacity(count);
    for index in 0..count {
        locks.push(
            lock_client::get_next_lock_id(
                &mut context.client,
                context.wallet.address,
                index as u64,
            )
            .await
            .context("failed to predict same-transaction lock id")?,
        );
    }
    Ok(locks)
}

fn resolve_plan_lock_grant(
    conn: &Connection,
    context: &mut shared::MutationContext,
    grant: &PlanLockGrant,
) -> Result<concordium_rust_sdk::base::protocol_level_locks::LockControllerSimpleV0Grant> {
    if grant.account == "@sender" {
        let unresolved = grant.to_unresolved()?;
        return Ok(
            concordium_rust_sdk::base::protocol_level_locks::LockControllerSimpleV0Grant {
                account: concordium_rust_sdk::protocol_level_tokens::CborHolderAccount::from(
                    context.wallet.address,
                ),
                roles: unresolved.roles,
            },
        );
    }
    let unresolved = grant.to_unresolved()?;
    shared::resolve_lock_grant(conn, context, &unresolved)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlanTokenIdInput(String);

impl PlanTokenIdInput {
    fn parse(input: &str) -> Result<Self> {
        TokenId::from_str(input).with_context(|| format!("invalid token identifier '{input}'"))?;
        Ok(Self(input.to_owned()))
    }

    fn resolve(&self) -> Result<TokenId> {
        TokenId::from_str(&self.0).with_context(|| format!("invalid token identifier '{}'", self.0))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlanTokenAmountInput(String);

impl PlanTokenAmountInput {
    fn parse(input: &str) -> Result<Self> {
        if input.trim().is_empty() {
            bail!("token amount must not be empty");
        }
        if input.trim().starts_with('-') {
            bail!("token amount must not be negative");
        }
        Ok(Self(input.to_owned()))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PlanAccountReferenceInput {
    Sender,
    AddressOrLabel(String),
}

impl PlanAccountReferenceInput {
    fn parse(input: &str) -> Self {
        if input == "@sender" {
            Self::Sender
        } else {
            Self::AddressOrLabel(input.to_owned())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PlanLockReferenceInput {
    Latest,
    Created(usize),
    Existing(String),
}

impl PlanLockReferenceInput {
    fn parse(input: &str) -> Result<Self> {
        match ParsedLockReference::parse(input)? {
            ParsedLockReference::Latest => Ok(Self::Latest),
            ParsedLockReference::Created(index) => Ok(Self::Created(index)),
            ParsedLockReference::Existing(lock) => Ok(Self::Existing(lock.to_owned())),
        }
    }
}

fn resolve_lock_reference(input: &str, predicted_locks: &[LockId]) -> Result<LockId> {
    match PlanLockReferenceInput::parse(input)? {
        PlanLockReferenceInput::Latest => bail!("saved plans must use explicit lock references"),
        PlanLockReferenceInput::Created(index) => predicted_locks
            .get(index - 1)
            .cloned()
            .with_context(|| format!("same-plan lock reference '@{index}' could not be resolved")),
        PlanLockReferenceInput::Existing(lock) => lock.parse().context("invalid lock identifier"),
    }
}

fn parse_token_id(input: &str) -> Result<TokenId> {
    PlanTokenIdInput::parse(input)?.resolve()
}

async fn query_plan_token_info(
    client: &mut concordium_rust_sdk::v2::Client,
    token_id: TokenId,
) -> Result<concordium_rust_sdk::protocol_level_tokens::TokenInfo> {
    shared::query_token_info(client, token_id.clone())
        .await
        .with_context(|| format!("token '{}' was not found on the plan network", token_id))
}

async fn resolve_amount(
    context: &mut shared::MutationContext,
    token_id: TokenId,
    amount: &str,
) -> Result<TokenAmount> {
    let amount = PlanTokenAmountInput::parse(amount)?;
    let mut client = context.client.clone();
    let token_info = query_plan_token_info(&mut client, token_id.clone()).await?;
    shared::parse_token_amount(amount.as_str(), token_info.token_state.decimals).with_context(
        || {
            format!(
                "invalid amount '{}' for token '{}'",
                amount.as_str(),
                token_id
            )
        },
    )
}

fn resolve_account(
    conn: &Connection,
    context: &mut shared::MutationContext,
    value: &str,
    label: &str,
) -> Result<AccountAddress> {
    match PlanAccountReferenceInput::parse(value) {
        PlanAccountReferenceInput::Sender => Ok(context.wallet.address),
        PlanAccountReferenceInput::AddressOrLabel(value) => shared::resolve_account_address(
            conn,
            context,
            Some(&value),
            "Account address or local label:",
            label,
            true,
        ),
    }
}

fn resolve_accounts(
    conn: &Connection,
    context: &mut shared::MutationContext,
    values: &[String],
    label: &str,
) -> Result<Vec<AccountAddress>> {
    values
        .iter()
        .map(|value| resolve_account(conn, context, value, label))
        .collect()
}

fn parse_admin_roles(
    roles: &[String],
) -> Result<Vec<concordium_rust_sdk::base::protocol_level_tokens::TokenAdminRole>> {
    roles
        .iter()
        .map(|role| shared::parse_token_admin_role(role))
        .collect()
}

async fn sign_and_send_meta_update(
    context: &mut shared::MutationContext,
    operations: &MetaUpdateOperations,
) -> Result<concordium_rust_sdk::base::hashes::TransactionHash> {
    let expiry = TransactionTime::seconds_after(DEFAULT_EXPIRY_SECS);
    let nonce = context
        .client
        .get_next_account_sequence_number(&context.wallet.address)
        .await
        .context("failed to query next account sequence number")?
        .nonce;
    let transaction = construct::meta_update_operations(
        context.wallet.num_keys(),
        context.wallet.address,
        nonce,
        expiry,
        operations,
    )
    .sign(&context.wallet);
    let block_item = BlockItem::AccountTransaction(transaction);
    context
        .client
        .send_block_item(&block_item)
        .await
        .context("failed to submit token composition transaction")
}

async fn run_composer(conn: &Connection, plan_path: &Path) -> Result<()> {
    let mut plan = load_or_new_plan(plan_path)?;
    if bind_plan_network(conn, &mut plan, true).await? {
        save_plan_atomic(plan_path, &plan)?;
    }
    println!("{}", render_plan_preview(&plan, Some(plan_path))?);
    println!();
    println!("Type 'help' or '?' for commands. Type 'exit' or press Ctrl-C to quit.");

    let mut line_editor = Reedline::create()
        .with_completer(Box::new(ComposeCompleter {
            plan_path: plan_path.to_owned(),
        }))
        .with_menu(ReedlineMenu::EngineCompleter(Box::new(
            IdeMenu::default().with_name("completion_menu"),
        )))
        .with_edit_mode(Box::new(Emacs::new(composer_keybindings())));
    let prompt = DefaultPrompt::default();
    while let Signal::Success(line) = line_editor.read_line(&prompt)? {
        match handle_repl_line(conn, plan_path, &mut plan, &line).await {
            Ok(ReplControl::Continue) => {}
            Ok(ReplControl::Exit) => break,
            Err(error) => cliclack::log::error(format!("{error:#}"))?,
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplControl {
    Continue,
    Exit,
}

async fn prepare_added_operation(
    conn: &Connection,
    plan: &mut Plan,
    operation: PlanOperation,
) -> Result<PlanOperation> {
    bind_plan_network(conn, plan, false).await?;
    let genesis_hash = plan
        .network_genesis_hash
        .as_deref()
        .context("token composition plan is missing network genesis hash")?;
    let mut unlocks = AccountReferenceUnlocks::new();
    let operation =
        resolve_operation_account_references(conn, genesis_hash, operation, &mut unlocks)?;
    validate_added_operation_against_network(plan, &operation).await?;
    Ok(operation)
}

async fn validate_added_operation_against_network(
    plan: &Plan,
    operation: &PlanOperation,
) -> Result<()> {
    let mut client = plan_network_client(plan).await?;
    validate_operation_tokens_and_amounts(&mut client, plan, operation).await
}

async fn plan_network_client(plan: &Plan) -> Result<concordium_rust_sdk::v2::Client> {
    let genesis_hash = plan
        .network_genesis_hash
        .as_deref()
        .context("token composition plan is missing network genesis hash")?;
    let app_config = app_config::load()?;
    let (network_name, entry) = app_config
        .networks
        .iter()
        .find(|(_, entry)| entry.genesis_hash == genesis_hash)
        .with_context(|| {
            format!("no configured network matches token composition genesis hash {genesis_hash}")
        })?;
    let endpoint = entry
        .node_endpoint
        .parse()
        .with_context(|| format!("invalid node endpoint for network '{network_name}'"))?;
    node_config::connect_v2_client(endpoint)
        .await
        .with_context(|| format!("failed to connect to node for network '{network_name}'"))
}

async fn validate_operation_tokens_and_amounts(
    client: &mut concordium_rust_sdk::v2::Client,
    plan: &Plan,
    operation: &PlanOperation,
) -> Result<()> {
    match operation {
        PlanOperation::Transfer { token, amount, .. }
        | PlanOperation::Mint { token, amount }
        | PlanOperation::Burn { token, amount } => {
            let token_id = parse_token_id(token)?;
            validate_token_amount(client, token_id, amount).await?;
        }
        PlanOperation::Pause { token }
        | PlanOperation::Unpause { token }
        | PlanOperation::MetadataUpdate { token, .. } => {
            let token_id = parse_token_id(token)?;
            query_plan_token_info(client, token_id).await?;
        }
        PlanOperation::AllowListAdd { token, .. }
        | PlanOperation::AllowListRemove { token, .. }
        | PlanOperation::DenyListAdd { token, .. }
        | PlanOperation::DenyListRemove { token, .. }
        | PlanOperation::AdminRolesAssign { token, .. }
        | PlanOperation::AdminRolesRevoke { token, .. } => {
            let token_id = parse_token_id(token)?;
            query_plan_token_info(client, token_id).await?;
        }
        PlanOperation::LockCreate { tokens, .. } => {
            for token in tokens {
                let token_id = parse_token_id(token)?;
                query_plan_token_info(client, token_id).await?;
            }
        }
        PlanOperation::LockFund {
            lock,
            token,
            amount,
        }
        | PlanOperation::LockReturn {
            lock,
            token,
            amount,
            ..
        } => {
            let token_id = parse_token_id(token)?;
            validate_lock_token(client, plan, lock, &token_id).await?;
            validate_token_amount(client, token_id, amount).await?;
        }
        PlanOperation::LockSend {
            lock,
            token,
            recipient,
            amount,
            ..
        } => {
            let token_id = parse_token_id(token)?;
            validate_lock_token(client, plan, lock, &token_id).await?;
            validate_lock_recipient(client, plan, lock, recipient).await?;
            validate_token_amount(client, token_id, amount).await?;
        }
        PlanOperation::LockCancel { lock } => {
            if matches!(
                ParsedLockReference::parse(lock)?,
                ParsedLockReference::Existing(_)
            ) {
                let lock_id = lock.parse().context("invalid lock identifier")?;
                shared::query_lock_info(client, lock_id).await?;
            }
        }
    }
    Ok(())
}

async fn validate_token_amount(
    client: &mut concordium_rust_sdk::v2::Client,
    token_id: TokenId,
    amount: &str,
) -> Result<()> {
    let token_info = query_plan_token_info(client, token_id.clone()).await?;
    shared::parse_token_amount(amount, token_info.token_state.decimals)
        .with_context(|| format!("invalid amount '{amount}' for token '{}'", token_id))?;
    Ok(())
}

async fn validate_lock_token(
    client: &mut concordium_rust_sdk::v2::Client,
    plan: &Plan,
    lock: &str,
    token_id: &TokenId,
) -> Result<()> {
    let configured = configured_tokens_for_lock_reference(Some(client), plan, lock).await?;
    ensure!(
        configured.iter().any(|configured| configured == token_id),
        "token '{}' is not configured for lock '{}'",
        token_id,
        lock
    );
    Ok(())
}

async fn select_token_for_lock_reference(plan: &Plan, lock: &str) -> Result<String> {
    let mut client = if matches!(
        ParsedLockReference::parse(lock)?,
        ParsedLockReference::Existing(_)
    ) {
        Some(plan_network_client(plan).await?)
    } else {
        None
    };
    let tokens = configured_tokens_for_lock_reference(client.as_mut(), plan, lock).await?;
    if tokens.is_empty() {
        bail!("lock '{lock}' has no configured tokens");
    }
    let items = tokens
        .into_iter()
        .map(|token| SelectItem {
            value: token.to_string(),
            label: token.to_string(),
            hint: String::new(),
        })
        .collect::<Vec<_>>();
    select_always("Select token", &items, None)
}

async fn validate_lock_recipient(
    client: &mut concordium_rust_sdk::v2::Client,
    plan: &Plan,
    lock: &str,
    recipient: &str,
) -> Result<()> {
    if recipient == "@sender"
        && matches!(
            ParsedLockReference::parse(lock)?,
            ParsedLockReference::Existing(_)
        )
    {
        return Ok(());
    }
    let configured = configured_recipients_for_lock_reference(Some(client), plan, lock).await?;
    match configured {
        shared::LockRecipientMode::Any => Ok(()),
        shared::LockRecipientMode::Limited(configured) => {
            ensure!(
                configured.iter().any(|configured| configured == recipient),
                "recipient '{}' is not configured for lock '{}'",
                recipient,
                lock
            );
            Ok(())
        }
    }
}

async fn select_recipient_for_lock_reference(plan: &Plan, lock: &str) -> Result<String> {
    let mut client = if matches!(
        ParsedLockReference::parse(lock)?,
        ParsedLockReference::Existing(_)
    ) {
        Some(plan_network_client(plan).await?)
    } else {
        None
    };
    let recipients = configured_recipients_for_lock_reference(client.as_mut(), plan, lock).await?;
    match recipients {
        shared::LockRecipientMode::Any => {
            Ok(input("Recipient account address or @sender:").interact()?)
        }
        shared::LockRecipientMode::Limited(recipients) => {
            if recipients.is_empty() {
                bail!("lock '{lock}' has no configured recipients");
            }
            let items = recipients
                .into_iter()
                .map(|recipient| SelectItem {
                    value: recipient.clone(),
                    label: recipient,
                    hint: String::new(),
                })
                .collect::<Vec<_>>();
            select_always("Select recipient", &items, None)
        }
    }
}

async fn configured_tokens_for_lock_reference(
    client: Option<&mut concordium_rust_sdk::v2::Client>,
    plan: &Plan,
    lock: &str,
) -> Result<Vec<TokenId>> {
    match ParsedLockReference::parse(lock)? {
        ParsedLockReference::Latest => latest_lock_create_tokens(plan),
        ParsedLockReference::Created(index) => nth_lock_create_tokens(plan, index),
        ParsedLockReference::Existing(_) => {
            let Some(client) = client else {
                bail!("node client is required to resolve existing lock '{lock}'");
            };
            let lock_id = lock.parse().context("invalid lock identifier")?;
            let info = shared::query_lock_info(client, lock_id).await?;
            Ok(shared::configured_lock_tokens(&info))
        }
    }
}

async fn configured_recipients_for_lock_reference(
    client: Option<&mut concordium_rust_sdk::v2::Client>,
    plan: &Plan,
    lock: &str,
) -> Result<shared::LockRecipientMode<String>> {
    match ParsedLockReference::parse(lock)? {
        ParsedLockReference::Latest => latest_lock_create_recipients(plan),
        ParsedLockReference::Created(index) => nth_lock_create_recipients(plan, index),
        ParsedLockReference::Existing(_) => {
            let Some(client) = client else {
                bail!("node client is required to resolve existing lock '{lock}'");
            };
            let lock_id = lock.parse().context("invalid lock identifier")?;
            let info = shared::query_lock_info(client, lock_id).await?;
            match info.recipients {
                LockRecipients::Any => Ok(shared::LockRecipientMode::Any),
                LockRecipients::Limited(recipients) => Ok(shared::LockRecipientMode::Limited(
                    recipients
                        .into_iter()
                        .map(|recipient| recipient.address.to_string())
                        .collect(),
                )),
            }
        }
    }
}

fn latest_lock_create_recipients(plan: &Plan) -> Result<shared::LockRecipientMode<String>> {
    let index = plan
        .operations
        .iter()
        .filter(|operation| operation.is_lock_create())
        .count();
    if index == 0 {
        bail!("lock reference '@' requires a preceding lock-create operation")
    }
    nth_lock_create_recipients(plan, index)
}

fn nth_lock_create_recipients(
    plan: &Plan,
    target: usize,
) -> Result<shared::LockRecipientMode<String>> {
    let mut index = 0usize;
    for operation in &plan.operations {
        let PlanOperation::LockCreate { recipients, .. } = operation else {
            continue;
        };
        index += 1;
        if index == target {
            return Ok(recipients.to_shared_mode());
        }
    }
    bail!("same-plan lock reference '@{target}' could not be resolved")
}

fn latest_lock_create_tokens(plan: &Plan) -> Result<Vec<TokenId>> {
    let index = plan
        .operations
        .iter()
        .filter(|operation| operation.is_lock_create())
        .count();
    if index == 0 {
        bail!("lock reference '@' requires a preceding lock-create operation")
    }
    nth_lock_create_tokens(plan, index)
}

fn nth_lock_create_tokens(plan: &Plan, target: usize) -> Result<Vec<TokenId>> {
    let mut index = 0usize;
    for operation in &plan.operations {
        let PlanOperation::LockCreate { tokens, .. } = operation else {
            continue;
        };
        index += 1;
        if index == target {
            return tokens.iter().map(|token| parse_token_id(token)).collect();
        }
    }
    bail!("same-plan lock reference '@{target}' could not be resolved")
}

async fn bind_plan_network(conn: &Connection, plan: &mut Plan, force_prompt: bool) -> Result<bool> {
    if plan.network_genesis_hash.is_some() {
        return Ok(false);
    }
    let (network_name, network_entry, _endpoint, _endpoint_label, _source) =
        resolve_account_network_context(conn, None, None, false, force_prompt).await?;
    let genesis_hash = network_entry.genesis_hash;
    plan.network_genesis_hash = Some(genesis_hash.clone());
    cliclack::log::info(format!(
        "Token composition plan bound to network {network_name} ({genesis_hash})."
    ))?;
    Ok(true)
}

fn resolve_operation_account_references(
    conn: &Connection,
    genesis_hash: &str,
    operation: PlanOperation,
    unlocks: &mut AccountReferenceUnlocks,
) -> Result<PlanOperation> {
    Ok(match operation {
        PlanOperation::Transfer {
            token,
            recipient,
            amount,
        } => PlanOperation::Transfer {
            token,
            recipient: resolve_plan_account(conn, genesis_hash, &recipient, "recipient", unlocks)?,
            amount,
        },
        PlanOperation::AllowListAdd { token, targets } => PlanOperation::AllowListAdd {
            token,
            targets: resolve_plan_accounts(conn, genesis_hash, &targets, "target", unlocks)?,
        },
        PlanOperation::AllowListRemove { token, targets } => PlanOperation::AllowListRemove {
            token,
            targets: resolve_plan_accounts(conn, genesis_hash, &targets, "target", unlocks)?,
        },
        PlanOperation::DenyListAdd { token, targets } => PlanOperation::DenyListAdd {
            token,
            targets: resolve_plan_accounts(conn, genesis_hash, &targets, "target", unlocks)?,
        },
        PlanOperation::DenyListRemove { token, targets } => PlanOperation::DenyListRemove {
            token,
            targets: resolve_plan_accounts(conn, genesis_hash, &targets, "target", unlocks)?,
        },
        PlanOperation::AdminRolesAssign {
            token,
            target,
            roles,
        } => PlanOperation::AdminRolesAssign {
            token,
            target: resolve_plan_account(conn, genesis_hash, &target, "target", unlocks)?,
            roles,
        },
        PlanOperation::AdminRolesRevoke {
            token,
            target,
            roles,
        } => PlanOperation::AdminRolesRevoke {
            token,
            target: resolve_plan_account(conn, genesis_hash, &target, "target", unlocks)?,
            roles,
        },
        PlanOperation::LockCreate {
            recipients,
            expiry,
            grants,
            tokens,
            keep_alive,
        } => PlanOperation::LockCreate {
            recipients: recipients.resolve_plan_accounts(conn, genesis_hash, unlocks)?,
            expiry,
            grants: grants
                .into_iter()
                .map(|grant| grant.resolve_account(conn, genesis_hash, unlocks))
                .collect::<Result<Vec<_>>>()?,
            tokens,
            keep_alive,
        },
        PlanOperation::LockSend {
            lock,
            token,
            source,
            recipient,
            amount,
        } => PlanOperation::LockSend {
            lock,
            token,
            source: resolve_plan_account(conn, genesis_hash, &source, "source", unlocks)?,
            recipient: resolve_plan_account(conn, genesis_hash, &recipient, "recipient", unlocks)?,
            amount,
        },
        PlanOperation::LockReturn {
            lock,
            token,
            source,
            amount,
        } => PlanOperation::LockReturn {
            lock,
            token,
            source: resolve_plan_account(conn, genesis_hash, &source, "source", unlocks)?,
            amount,
        },
        other => other,
    })
}

fn resolve_plan_accounts(
    conn: &Connection,
    genesis_hash: &str,
    values: &[String],
    label: &str,
    unlocks: &mut AccountReferenceUnlocks,
) -> Result<Vec<String>> {
    values
        .iter()
        .map(|value| resolve_plan_account(conn, genesis_hash, value, label, unlocks))
        .collect()
}

fn resolve_plan_account(
    conn: &Connection,
    genesis_hash: &str,
    value: &str,
    label: &str,
    unlocks: &mut AccountReferenceUnlocks,
) -> Result<String> {
    if value == "@sender" {
        return Ok(value.to_owned());
    }
    let address = resolve_account_reference(
        conn,
        AccountReferenceContext {
            network_name: genesis_hash,
            network_genesis_hash: genesis_hash,
        },
        Some(value),
        "Account address or local label:",
        label,
        true,
        unlocks,
    )?;
    Ok(address.to_string())
}

async fn handle_repl_line(
    conn: &Connection,
    plan_path: &Path,
    plan: &mut Plan,
    line: &str,
) -> Result<ReplControl> {
    let trimmed = line.trim();
    if !trimmed.is_empty() {
        *plan = read_plan(plan_path)?;
    }
    match parse_repl_command_for_plan(plan, line).await? {
        ReplCommand::Empty => {}
        ReplCommand::Help => println!("{}", help_text()),
        ReplCommand::Exit => return Ok(ReplControl::Exit),
        ReplCommand::Preview => println!("{}", render_plan_preview(plan, Some(plan_path))?),
        ReplCommand::Add(operation) => {
            let operation = prepare_added_operation(conn, plan, operation).await?;
            let mut candidate = plan.clone();
            candidate.operations.push(operation);
            canonicalize_lock_references(&mut candidate)?;
            save_plan_atomic(plan_path, &candidate)?;
            *plan = candidate;
            cliclack::log::success(format!(
                "Added operation {} and saved {}.",
                plan.operations.len(),
                plan_path.display()
            ))?;
        }
        ReplCommand::Submit(args) => {
            let prepared = PreparedTokenComposeSubmit::from_repl(plan_path.to_owned(), *args)?;
            submit_prepared(conn, prepared).await?;
            return Ok(ReplControl::Exit);
        }
    }
    Ok(ReplControl::Continue)
}

fn composer_keybindings() -> Keybindings {
    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_owned()),
            ReedlineEvent::MenuNext,
        ]),
    );
    keybindings.add_binding(
        KeyModifiers::ALT,
        KeyCode::Enter,
        ReedlineEvent::Edit(vec![EditCommand::InsertNewline]),
    );
    keybindings
}

#[derive(Debug, Clone)]
struct ComposeCompleter {
    plan_path: std::path::PathBuf,
}

impl Completer for ComposeCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        complete_compose_line(line, pos, Some(self.plan_path.as_path()))
    }
}

fn complete_compose_line(line: &str, pos: usize, plan_path: Option<&Path>) -> Vec<Suggestion> {
    let pos = pos.min(line.len());
    let prefix = &line[..pos];
    let (word_start, current_word) = completion_word(prefix);
    let words = shlex::split(prefix).unwrap_or_else(|| {
        prefix
            .split_whitespace()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
    });
    let candidates = completion_candidates(&words, current_word, plan_path);
    candidates
        .into_iter()
        .filter(|candidate| candidate.starts_with(current_word))
        .map(|candidate| Suggestion {
            value: candidate,
            description: None,
            style: None,
            extra: None,
            span: Span {
                start: word_start,
                end: pos,
            },
            append_whitespace: true,
        })
        .collect()
}

fn completion_word(prefix: &str) -> (usize, &str) {
    let start = prefix
        .char_indices()
        .rev()
        .find(|(_, ch)| ch.is_whitespace())
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    (start, &prefix[start..])
}

fn completion_candidates(
    words: &[String],
    current_word: &str,
    plan_path: Option<&Path>,
) -> Vec<String> {
    if current_word.starts_with('@') {
        return account_and_lock_reference_candidates(plan_path);
    }
    if current_word.starts_with("--") {
        return flag_candidates(words);
    }
    match words {
        [] => top_level_command_candidates(),
        [first] if !current_word.is_empty() && first == current_word => {
            top_level_command_candidates()
        }
        [first] => nested_command_candidates(first),
        [first, second] if second == current_word => nested_command_candidates(first),
        _ => Vec::new(),
    }
}

fn top_level_command_candidates() -> Vec<String> {
    [
        "transfer",
        "mint",
        "burn",
        "pause",
        "unpause",
        "allow-list",
        "deny-list",
        "admin-roles",
        "metadata",
        "lock",
        "preview",
        "submit",
        "help",
        "exit",
    ]
    .into_iter()
    .map(ToOwned::to_owned)
    .collect()
}

fn nested_command_candidates(first: &str) -> Vec<String> {
    match first {
        "allow-list" | "deny-list" => vec!["add".to_owned(), "remove".to_owned()],
        "admin-roles" => vec!["assign".to_owned(), "revoke".to_owned()],
        "metadata" => vec!["update".to_owned()],
        "lock" => vec![
            "create".to_owned(),
            "fund".to_owned(),
            "send".to_owned(),
            "return".to_owned(),
            "cancel".to_owned(),
        ],
        _ => Vec::new(),
    }
}

fn flag_candidates(words: &[String]) -> Vec<String> {
    let first = words.first().map(String::as_str);
    let second = words.get(1).map(String::as_str);
    let flags: &[&str] = match (first, second) {
        (Some("transfer"), _) => &["--token", "--recipient", "--amount"],
        (Some("mint" | "burn"), _) => &["--token", "--amount"],
        (Some("pause" | "unpause"), _) => &["--token"],
        (Some("allow-list" | "deny-list"), Some("add" | "remove")) => &["--token", "--target"],
        (Some("admin-roles"), Some("assign" | "revoke")) => &["--token", "--target", "--role"],
        (Some("metadata"), Some("update")) => &["--token", "--url", "--checksum-sha256"],
        (Some("lock"), Some("create")) => &[
            "--recipient",
            "--any-recipient",
            "--expiry",
            "--grant",
            "--token",
            "--keep-alive",
        ],
        (Some("lock"), Some("fund")) => &["--token", "--amount"],
        (Some("lock"), Some("send")) => &["--source", "--recipient", "--token", "--amount"],
        (Some("lock"), Some("return")) => &["--source", "--token", "--amount"],
        (Some("submit"), _) => &[
            "--sender",
            "--network",
            "--node",
            "--no-wait",
            "--non-interactive",
            "--no-defaults",
        ],
        _ => &[],
    };
    flags.iter().map(|flag| (*flag).to_owned()).collect()
}

fn account_and_lock_reference_candidates(plan_path: Option<&Path>) -> Vec<String> {
    let mut candidates = vec!["@sender".to_owned(), "@".to_owned()];
    if let Some(plan_path) = plan_path
        && let Ok(plan) = read_plan(plan_path)
    {
        let lock_count = plan
            .operations
            .iter()
            .filter(|operation| operation.is_lock_create())
            .count();
        candidates.extend((1..=lock_count).map(|index| format!("@{index}")));
    }
    candidates.sort();
    candidates.dedup();
    candidates
}

#[derive(Debug, Clone)]
enum ReplCommand {
    Empty,
    Help,
    Exit,
    Preview,
    Add(PlanOperation),
    Submit(Box<SubmitCommandArgs>),
}

#[derive(Debug, Clone, Default)]
struct SubmitCommandArgs {
    sender: Option<String>,
    network: Option<String>,
    node: Option<concordium_rust_sdk::v2::Endpoint>,
    no_wait: bool,
    non_interactive: bool,
    no_defaults: bool,
}

async fn parse_repl_command_for_plan(plan: &Plan, line: &str) -> Result<ReplCommand> {
    let Some(words) = shlex::split(line) else {
        bail!("failed to parse command line; check quoting")
    };
    if matches!(words.first().map(String::as_str), Some("lock"))
        && matches!(
            words.get(1).map(String::as_str),
            Some("fund" | "send" | "return")
        )
    {
        return parse_plan_aware_lock_op(plan, &words)
            .await
            .map(ReplCommand::Add);
    }
    parse_repl_words(words)
}

#[cfg(test)]
fn parse_repl_command(line: &str) -> Result<ReplCommand> {
    let Some(words) = shlex::split(line) else {
        bail!("failed to parse command line; check quoting")
    };
    parse_repl_words(words)
}

fn parse_repl_words(words: Vec<String>) -> Result<ReplCommand> {
    let Some(_first) = words.first() else {
        return Ok(ReplCommand::Empty);
    };
    let mut words = WordCursor::new(words);
    let Some(command) = words.pop() else {
        return Ok(ReplCommand::Empty);
    };
    match command.as_str() {
        "help" | "?" => Ok(ReplCommand::Help),
        "exit" | "quit" => Ok(ReplCommand::Exit),
        "preview" => {
            words.ensure_empty()?;
            Ok(ReplCommand::Preview)
        }
        "submit" => parse_submit_command(words)
            .map(Box::new)
            .map(ReplCommand::Submit),
        "transfer" | "mint" | "burn" | "pause" | "unpause" | "allow-list" | "deny-list"
        | "admin-roles" | "metadata" | "lock" => {
            parse_operation_command(command, words).map(ReplCommand::Add)
        }
        other => bail!("unknown token compose command '{other}'. Type 'help' for usage."),
    }
}

async fn parse_plan_aware_lock_op(plan: &Plan, words: &[String]) -> Result<PlanOperation> {
    let action = words
        .get(1)
        .map(String::as_str)
        .context("missing lock action")?;
    let tail = words.iter().skip(2).cloned().collect::<Vec<_>>();
    let mut args = OperationArgs::from_words(WordCursor::new(tail))?;
    match action {
        "fund" => {
            let lock = args.take_positional_or_prompt("lock", "Lock id or @ reference:")?;
            let token = args.take_token_or_select_for_lock(plan, &lock).await?;
            Ok(PlanOperation::LockFund {
                lock,
                token,
                amount: args.take_or_prompt("amount", "Token amount:")?,
            })
        }
        "send" => {
            let lock = args.take_positional_or_prompt("lock", "Lock id or @ reference:")?;
            let token = args.take_token_or_select_for_lock(plan, &lock).await?;
            let source = args.take_or_prompt("source", "Source account address or @sender:")?;
            let recipient = args.take_recipient_or_select_for_lock(plan, &lock).await?;
            Ok(PlanOperation::LockSend {
                lock,
                token,
                source,
                recipient,
                amount: args.take_or_prompt("amount", "Token amount:")?,
            })
        }
        "return" => {
            let lock = args.take_positional_or_prompt("lock", "Lock id or @ reference:")?;
            let token = args.take_token_or_select_for_lock(plan, &lock).await?;
            Ok(PlanOperation::LockReturn {
                lock,
                token,
                source: args.take_or_prompt("source", "Source account address or @sender:")?,
                amount: args.take_or_prompt("amount", "Token amount:")?,
            })
        }
        other => bail!("unknown lock action '{other}'"),
    }
}

fn parse_operation_command(family: String, words: WordCursor) -> Result<PlanOperation> {
    match family.as_str() {
        "transfer" => parse_transfer(words),
        "mint" => parse_amount_op(words, AmountOperation::Mint),
        "burn" => parse_amount_op(words, AmountOperation::Burn),
        "pause" => parse_token_only_op(words, TokenOnlyOperation::Pause),
        "unpause" => parse_token_only_op(words, TokenOnlyOperation::Unpause),
        "allow-list" => parse_list_op(words, ListKind::Allow),
        "deny-list" => parse_list_op(words, ListKind::Deny),
        "admin-roles" => parse_admin_roles_op(words),
        "metadata" => parse_metadata_op(words),
        "lock" => parse_lock_op(words),
        other => bail!("unknown token operation family '{other}'"),
    }
}

fn parse_submit_command(mut words: WordCursor) -> Result<SubmitCommandArgs> {
    let mut args = SubmitCommandArgs::default();
    while let Some(word) = words.pop() {
        match word.as_str() {
            "--sender" | "--account" => args.sender = Some(words.required_word("sender")?),
            "--network" => args.network = Some(words.required_word("network")?),
            "--node" => {
                let endpoint = words.required_word("node endpoint")?;
                args.node = Some(endpoint.parse().context("invalid node endpoint")?);
            }
            "--no-wait" => args.no_wait = true,
            "--non-interactive" => args.non_interactive = true,
            "--no-defaults" => args.no_defaults = true,
            other => bail!("unknown submit option '{other}'"),
        }
    }
    Ok(args)
}

#[derive(Debug, Clone, Copy)]
enum AmountOperation {
    Mint,
    Burn,
}

#[derive(Debug, Clone, Copy)]
enum TokenOnlyOperation {
    Pause,
    Unpause,
}

#[derive(Debug, Clone, Copy)]
enum ListKind {
    Allow,
    Deny,
}

fn parse_transfer(words: WordCursor) -> Result<PlanOperation> {
    let mut args = OperationArgs::from_words(words)?;
    Ok(PlanOperation::Transfer {
        token: args.take_or_prompt("token", "Token identifier:")?,
        recipient: args.take_or_prompt("recipient", "Recipient account address or label:")?,
        amount: args.take_or_prompt("amount", "Token amount:")?,
    })
}

fn parse_amount_op(words: WordCursor, kind: AmountOperation) -> Result<PlanOperation> {
    let mut args = OperationArgs::from_words(words)?;
    let token = args.take_or_prompt("token", "Token identifier:")?;
    let amount = args.take_or_prompt("amount", "Token amount:")?;
    Ok(match kind {
        AmountOperation::Mint => PlanOperation::Mint { token, amount },
        AmountOperation::Burn => PlanOperation::Burn { token, amount },
    })
}

fn parse_token_only_op(words: WordCursor, kind: TokenOnlyOperation) -> Result<PlanOperation> {
    let mut args = OperationArgs::from_words(words)?;
    let token = args.take_or_prompt("token", "Token identifier:")?;
    Ok(match kind {
        TokenOnlyOperation::Pause => PlanOperation::Pause { token },
        TokenOnlyOperation::Unpause => PlanOperation::Unpause { token },
    })
}

fn parse_list_op(mut words: WordCursor, kind: ListKind) -> Result<PlanOperation> {
    let action = words.required_word("list action")?;
    let mut args = OperationArgs::from_words(words)?;
    let token = args.take_or_prompt("token", "Token identifier:")?;
    let targets = args.take_many_or_prompt(
        "target",
        "Target account addresses or labels (comma-separated):",
    )?;
    match (kind, action.as_str()) {
        (ListKind::Allow, "add") => Ok(PlanOperation::AllowListAdd { token, targets }),
        (ListKind::Allow, "remove") => Ok(PlanOperation::AllowListRemove { token, targets }),
        (ListKind::Deny, "add") => Ok(PlanOperation::DenyListAdd { token, targets }),
        (ListKind::Deny, "remove") => Ok(PlanOperation::DenyListRemove { token, targets }),
        (_, other) => bail!("unknown list action '{other}'; expected add or remove"),
    }
}

fn parse_admin_roles_op(mut words: WordCursor) -> Result<PlanOperation> {
    let action = words.required_word("admin-roles action")?;
    let mut args = OperationArgs::from_words(words)?;
    let token = args.take_or_prompt("token", "Token identifier:")?;
    let target = args.take_or_prompt("target", "Target account address or label:")?;
    let roles = args.take_many_or_prompt("role", "Admin roles (comma-separated):")?;
    match action.as_str() {
        "assign" => Ok(PlanOperation::AdminRolesAssign {
            token,
            target,
            roles,
        }),
        "revoke" => Ok(PlanOperation::AdminRolesRevoke {
            token,
            target,
            roles,
        }),
        other => bail!("unknown admin-roles action '{other}'; expected assign or revoke"),
    }
}

fn parse_metadata_op(mut words: WordCursor) -> Result<PlanOperation> {
    let action = words.required_word("metadata action")?;
    if action != "update" {
        bail!("unknown metadata action '{action}'; expected update");
    }
    let mut args = OperationArgs::from_words(words)?;
    Ok(PlanOperation::MetadataUpdate {
        token: args.take_or_prompt("token", "Token identifier:")?,
        url: args.take_or_prompt("url", "Metadata URL:")?,
        checksum_sha256: args.take_optional("checksum-sha256"),
    })
}

fn parse_lock_op(mut words: WordCursor) -> Result<PlanOperation> {
    let action = words.required_word("lock action")?;
    let mut args = OperationArgs::from_words(words)?;
    match action.as_str() {
        "create" => Ok(PlanOperation::LockCreate {
            recipients: args.take_lock_recipients()?,
            expiry: args.take_or_prompt("expiry", "Lock expiry:")?,
            grants: args.take_lock_grants()?,
            tokens: args.take_many_or_prompt("token", "Tokens (comma-separated):")?,
            keep_alive: args.take_keep_alive()?,
        }),
        "fund" => Ok(PlanOperation::LockFund {
            lock: args.take_positional_or_prompt("lock", "Lock id or @ reference:")?,
            token: args.take_or_prompt("token", "Token identifier:")?,
            amount: args.take_or_prompt("amount", "Token amount:")?,
        }),
        "send" => Ok(PlanOperation::LockSend {
            lock: args.take_positional_or_prompt("lock", "Lock id or @ reference:")?,
            token: args.take_or_prompt("token", "Token identifier:")?,
            source: args.take_or_prompt("source", "Source account address or @sender:")?,
            recipient: args.take_or_prompt("recipient", "Recipient account address or @sender:")?,
            amount: args.take_or_prompt("amount", "Token amount:")?,
        }),
        "return" => Ok(PlanOperation::LockReturn {
            lock: args.take_positional_or_prompt("lock", "Lock id or @ reference:")?,
            token: args.take_or_prompt("token", "Token identifier:")?,
            source: args.take_or_prompt("source", "Source account address or @sender:")?,
            amount: args.take_or_prompt("amount", "Token amount:")?,
        }),
        "cancel" => Ok(PlanOperation::LockCancel {
            lock: args.take_positional_or_prompt("lock", "Lock id or @ reference:")?,
        }),
        other => bail!("unknown lock action '{other}'"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OperationArgs {
    positionals: VecDeque<String>,
    options: Vec<(String, String)>,
    flags: Vec<String>,
}

impl OperationArgs {
    fn from_words(mut words: WordCursor) -> Result<Self> {
        let mut positionals = VecDeque::new();
        let mut options = Vec::new();
        let mut flags = Vec::new();
        while let Some(word) = words.pop() {
            if let Some(name) = word.strip_prefix("--") {
                match name {
                    "keep-alive" | "any-recipient" => flags.push(name.to_owned()),
                    _ => options.push((name.to_owned(), words.required_word(name)?)),
                }
            } else {
                positionals.push_back(word);
            }
        }
        Ok(Self {
            positionals,
            options,
            flags,
        })
    }

    fn flag(&self, name: &str) -> bool {
        self.flags.iter().any(|flag| flag == name)
    }

    fn take_keep_alive(&self) -> Result<bool> {
        if self.flag("keep-alive") {
            return Ok(true);
        }
        Ok(confirm("Keep the lock alive after funds are returned?")
            .initial_value(false)
            .interact()?)
    }

    fn take_lock_recipients(&mut self) -> Result<PlanLockRecipients> {
        let recipients = self.take_all("recipient");
        if self.flag("any-recipient") {
            if !recipients.is_empty() {
                bail!("--any-recipient cannot be combined with --recipient");
            }
            return Ok(PlanLockRecipients::any());
        }
        if !recipients.is_empty() {
            return Ok(PlanLockRecipients::limited(recipients));
        }
        let allow_any = confirm("Allow any eligible account to receive funds from this lock?")
            .initial_value(false)
            .interact()?;
        if allow_any {
            Ok(PlanLockRecipients::any())
        } else {
            Ok(PlanLockRecipients::limited(self.take_many_or_prompt(
                "recipient",
                "Recipients (comma-separated):",
            )?))
        }
    }

    fn take_optional(&mut self, name: &str) -> Option<String> {
        let index = self
            .options
            .iter()
            .position(|(candidate, _)| candidate == name)?;
        Some(self.options.remove(index).1)
    }

    fn take_all(&mut self, name: &str) -> Vec<String> {
        let mut values = Vec::new();
        let mut index = 0;
        while index < self.options.len() {
            if self.options[index].0 == name {
                values.push(self.options.remove(index).1);
            } else {
                index += 1;
            }
        }
        values
    }

    fn take_promptable(&mut self, name: &'static str) -> Promptable<String> {
        Promptable::from_option(self.take_optional(name), name)
    }

    fn take_or_prompt(&mut self, name: &'static str, prompt: &str) -> Result<String> {
        self.take_promptable(name)
            .resolve_with(InputMode::interactive(), || prompt_string(prompt))
            .map(|resolved| resolved.into_value())
    }

    fn take_positional_or_prompt(&mut self, name: &'static str, prompt: &str) -> Result<String> {
        let input = Promptable::from_option(self.positionals.pop_front(), name);
        input
            .resolve_with(InputMode::interactive(), || {
                self.take_or_prompt(name, prompt)
            })
            .map(|resolved| resolved.into_value())
    }

    fn take_many_or_prompt(&mut self, name: &'static str, prompt: &str) -> Result<Vec<String>> {
        let values = self.take_all(name);
        let raw = Promptable::from_option((!values.is_empty()).then(|| values.join(",")), name)
            .resolve_with(InputMode::interactive(), || prompt_string(prompt))?
            .into_value();
        let value = raw;
        let values = value
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if values.is_empty() {
            bail!("{name} requires at least one value");
        }
        Ok(values)
    }

    fn take_lock_grants(&mut self) -> Result<Vec<PlanLockGrant>> {
        let inline = self.take_all("grant");
        if !inline.is_empty() {
            return inline
                .iter()
                .map(String::as_str)
                .map(shared::parse_unresolved_lock_grant)
                .map(|grant| grant.map(PlanLockGrant::from_unresolved))
                .collect();
        }
        shared::prompt_unresolved_lock_grants().map(|grants| {
            grants
                .into_iter()
                .map(PlanLockGrant::from_unresolved)
                .collect()
        })
    }

    async fn take_token_or_select_for_lock(&mut self, plan: &Plan, lock: &str) -> Result<String> {
        match self.take_promptable("token") {
            Promptable::Provided(token) => Ok(token),
            Promptable::Missing { .. } => select_token_for_lock_reference(plan, lock).await,
        }
    }

    async fn take_recipient_or_select_for_lock(
        &mut self,
        plan: &Plan,
        lock: &str,
    ) -> Result<String> {
        match self.take_promptable("recipient") {
            Promptable::Provided(recipient) => Ok(recipient),
            Promptable::Missing { .. } => select_recipient_for_lock_reference(plan, lock).await,
        }
    }
}

fn prompt_string(prompt: &str) -> Result<String> {
    Ok(input(prompt).interact()?)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WordCursor {
    words: VecDeque<String>,
}

impl WordCursor {
    fn new(words: Vec<String>) -> Self {
        Self {
            words: VecDeque::from(words),
        }
    }

    fn pop(&mut self) -> Option<String> {
        self.words.pop_front()
    }

    fn required_word(&mut self, label: &str) -> Result<String> {
        self.pop()
            .with_context(|| format!("missing required {label}"))
    }

    fn ensure_empty(&self) -> Result<()> {
        if self.words.is_empty() {
            Ok(())
        } else {
            bail!("unexpected argument '{}'", self.words[0])
        }
    }
}

fn help_text() -> &'static str {
    r#"Operation commands:
  transfer --token TOKEN --recipient ACCOUNT --amount AMOUNT
  mint --token TOKEN --amount AMOUNT
  burn --token TOKEN --amount AMOUNT
  pause --token TOKEN
  unpause --token TOKEN
  allow-list add --token TOKEN --target ACCOUNT [--target ACCOUNT...]
  allow-list remove --token TOKEN --target ACCOUNT [--target ACCOUNT...]
  deny-list add --token TOKEN --target ACCOUNT [--target ACCOUNT...]
  deny-list remove --token TOKEN --target ACCOUNT [--target ACCOUNT...]
  admin-roles assign --token TOKEN --target ACCOUNT --role ROLE [--role ROLE...]
  admin-roles revoke --token TOKEN --target ACCOUNT --role ROLE [--role ROLE...]
  metadata update --token TOKEN --url URL [--checksum-sha256 HEX]
  lock create (--recipient ACCOUNT [--recipient ACCOUNT...] | --any-recipient) --expiry TIME --grant GRANT --token TOKEN [--keep-alive]
  lock fund LOCK --token TOKEN --amount AMOUNT
  lock send LOCK --source ACCOUNT --recipient ACCOUNT --token TOKEN --amount AMOUNT
  lock return LOCK --source ACCOUNT --token TOKEN --amount AMOUNT
  lock cancel LOCK

Plan commands:
  preview
  submit [--sender LABEL] [--network NAME] [--node ENDPOINT] [--no-wait] [--non-interactive] [--no-defaults]

Session commands:
  help | ?
  exit

Saved plan syntax:
  lock-create recipients = ["ACCOUNT", ...] for limited-recipient locks
  lock-create recipients = "any" for any-recipient locks, previewed as any eligible account

Lock references:
  @          most recent preceding lock-create, canonicalized to @N on save
  @N         Nth lock created in this plan; for example @2 references the second lock-create
  <lock-id>  base58check lock id for an existing on-chain lock

Account references:
  <address>  concrete account address saved directly
  <label>    finalized local account label resolved to an address before save
  @sender    selected submit sender, resolved when submitting
"#
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Plan {
    version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    network_genesis_hash: Option<String>,
    #[serde(default)]
    operations: Vec<PlanOperation>,
}

impl Plan {
    fn empty() -> Self {
        Self {
            version: PLAN_VERSION,
            network_genesis_hash: None,
            operations: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum PlanOperation {
    Transfer {
        token: String,
        recipient: String,
        amount: String,
    },
    Mint {
        token: String,
        amount: String,
    },
    Burn {
        token: String,
        amount: String,
    },
    Pause {
        token: String,
    },
    Unpause {
        token: String,
    },
    AllowListAdd {
        token: String,
        targets: Vec<String>,
    },
    AllowListRemove {
        token: String,
        targets: Vec<String>,
    },
    DenyListAdd {
        token: String,
        targets: Vec<String>,
    },
    DenyListRemove {
        token: String,
        targets: Vec<String>,
    },
    AdminRolesAssign {
        token: String,
        target: String,
        roles: Vec<String>,
    },
    AdminRolesRevoke {
        token: String,
        target: String,
        roles: Vec<String>,
    },
    MetadataUpdate {
        token: String,
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        checksum_sha256: Option<String>,
    },
    LockCreate {
        recipients: PlanLockRecipients,
        expiry: String,
        grants: Vec<PlanLockGrant>,
        tokens: Vec<String>,
        #[serde(default)]
        keep_alive: bool,
    },
    LockFund {
        lock: String,
        token: String,
        amount: String,
    },
    LockSend {
        lock: String,
        token: String,
        source: String,
        recipient: String,
        amount: String,
    },
    LockReturn {
        lock: String,
        token: String,
        source: String,
        amount: String,
    },
    LockCancel {
        lock: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
enum PlanLockRecipients {
    Any(String),
    Limited(Vec<String>),
}

impl PlanLockRecipients {
    fn any() -> Self {
        Self::Any("any".to_owned())
    }

    fn limited(recipients: Vec<String>) -> Self {
        Self::Limited(recipients)
    }

    fn validate(&self) -> Result<()> {
        match self {
            Self::Any(value) if value == "any" => Ok(()),
            Self::Any(value) => bail!(
                "unsupported lock recipients value '{value}'; use recipients = \"any\" or an array of accounts"
            ),
            Self::Limited(_) => Ok(()),
        }
    }

    fn preview(&self) -> String {
        match self {
            Self::Any(_) => "any eligible account".to_owned(),
            Self::Limited(recipients) => recipients.join(", "),
        }
    }

    fn to_shared_mode(&self) -> shared::LockRecipientMode<String> {
        match self {
            Self::Any(_) => shared::LockRecipientMode::Any,
            Self::Limited(recipients) => shared::LockRecipientMode::Limited(recipients.clone()),
        }
    }

    fn resolve_accounts(
        &self,
        conn: &Connection,
        context: &mut shared::MutationContext,
    ) -> Result<shared::LockRecipientMode<AccountAddress>> {
        self.validate()?;
        match self {
            Self::Any(_) => Ok(shared::LockRecipientMode::Any),
            Self::Limited(recipients) => Ok(shared::LockRecipientMode::Limited(resolve_accounts(
                conn,
                context,
                recipients,
                "recipient",
            )?)),
        }
    }

    fn resolve_plan_accounts(
        self,
        conn: &Connection,
        genesis_hash: &str,
        unlocks: &mut AccountReferenceUnlocks,
    ) -> Result<Self> {
        self.validate()?;
        match self {
            Self::Any(_) => Ok(Self::any()),
            Self::Limited(recipients) => Ok(Self::Limited(resolve_plan_accounts(
                conn,
                genesis_hash,
                &recipients,
                "recipient",
                unlocks,
            )?)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PlanLockGrant {
    account: String,
    capabilities: Vec<String>,
}

impl PlanLockGrant {
    fn from_unresolved(grant: shared::UnresolvedLockGrant) -> Self {
        Self {
            account: grant.account,
            capabilities: grant
                .roles
                .into_iter()
                .map(shared::lock_capability_cli_name)
                .map(ToOwned::to_owned)
                .collect(),
        }
    }

    fn to_unresolved(&self) -> Result<shared::UnresolvedLockGrant> {
        if self.capabilities.is_empty() {
            bail!(
                "lock grant for '{}' must include at least one capability",
                self.account
            );
        }
        let roles = self
            .capabilities
            .iter()
            .map(|capability| shared::parse_lock_capability(capability))
            .collect::<Result<Vec<_>>>()?;
        Ok(shared::UnresolvedLockGrant {
            account: self.account.clone(),
            roles,
        })
    }

    fn preview(&self) -> String {
        format!("{}:{}", self.account, self.capabilities.join(","))
    }

    fn resolve_account(
        mut self,
        conn: &Connection,
        genesis_hash: &str,
        unlocks: &mut AccountReferenceUnlocks,
    ) -> Result<Self> {
        self.account = resolve_plan_account(conn, genesis_hash, &self.account, "grant", unlocks)?;
        Ok(self)
    }
}

impl PlanOperation {
    fn lock_reference_mut(&mut self) -> Option<&mut String> {
        match self {
            Self::LockFund { lock, .. }
            | Self::LockSend { lock, .. }
            | Self::LockReturn { lock, .. }
            | Self::LockCancel { lock } => Some(lock),
            _ => None,
        }
    }

    fn is_lock_create(&self) -> bool {
        matches!(self, Self::LockCreate { .. })
    }

    fn preview_line(&self, lock_create_index: Option<usize>) -> String {
        match self {
            Self::Transfer {
                token,
                recipient,
                amount,
            } => format!("Transfer {amount} {token} to {recipient}"),
            Self::Mint { token, amount } => format!("Mint {amount} {token}"),
            Self::Burn { token, amount } => format!("Burn {amount} {token}"),
            Self::Pause { token } => format!("Pause {token}"),
            Self::Unpause { token } => format!("Unpause {token}"),
            Self::AllowListAdd { token, targets } => {
                format!("Add {} to {token} allow-list", targets.join(", "))
            }
            Self::AllowListRemove { token, targets } => {
                format!("Remove {} from {token} allow-list", targets.join(", "))
            }
            Self::DenyListAdd { token, targets } => {
                format!("Add {} to {token} deny-list", targets.join(", "))
            }
            Self::DenyListRemove { token, targets } => {
                format!("Remove {} from {token} deny-list", targets.join(", "))
            }
            Self::AdminRolesAssign {
                token,
                target,
                roles,
            } => format!(
                "Assign {token} admin roles [{}] to {target}",
                roles.join(", ")
            ),
            Self::AdminRolesRevoke {
                token,
                target,
                roles,
            } => format!(
                "Revoke {token} admin roles [{}] from {target}",
                roles.join(", ")
            ),
            Self::MetadataUpdate {
                token,
                url,
                checksum_sha256,
            } => match checksum_sha256 {
                Some(checksum) => format!("Update {token} metadata to {url} ({checksum})"),
                None => format!("Update {token} metadata to {url}"),
            },
            Self::LockCreate {
                recipients,
                expiry,
                grants,
                tokens,
                keep_alive,
            } => format!(
                "Create lock @{} for tokens [{}], recipients [{}], expiry {expiry}, grants [{}], keep alive {}",
                lock_create_index.unwrap_or_default(),
                tokens.join(", "),
                recipients.preview(),
                grants
                    .iter()
                    .map(PlanLockGrant::preview)
                    .collect::<Vec<_>>()
                    .join("; "),
                if *keep_alive { "yes" } else { "no" }
            ),
            Self::LockFund {
                lock,
                token,
                amount,
            } => format!("Fund lock {lock} with {amount} {token}"),
            Self::LockSend {
                lock,
                token,
                source,
                recipient,
                amount,
            } => format!("Send {amount} {token} from {source} in lock {lock} to {recipient}"),
            Self::LockReturn {
                lock,
                token,
                source,
                amount,
            } => format!("Return {amount} {token} from {source} in lock {lock}"),
            Self::LockCancel { lock } => format!("Cancel lock {lock}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParsedLockReference<'a> {
    Latest,
    Created(usize),
    Existing(&'a str),
}

impl<'a> ParsedLockReference<'a> {
    fn parse(input: &'a str) -> Result<Self> {
        if input == "@" {
            return Ok(Self::Latest);
        }
        if let Some(number) = input.strip_prefix('@') {
            if number.is_empty() {
                bail!("lock reference '@' must be canonicalized before validation");
            }
            let index = number
                .parse::<usize>()
                .with_context(|| format!("invalid same-plan lock reference '{input}'"))?;
            if index == 0 {
                bail!("same-plan lock references are one-based; use @1 for the first lock")
            }
            return Ok(Self::Created(index));
        }
        Ok(Self::Existing(input))
    }
}

impl fmt::Display for ParsedLockReference<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Latest => f.write_str("@"),
            Self::Created(index) => write!(f, "@{index}"),
            Self::Existing(lock) => f.write_str(lock),
        }
    }
}

fn load_or_new_plan(path: &Path) -> Result<Plan> {
    if path.exists() {
        read_plan(path)
    } else {
        Ok(Plan::empty())
    }
}

fn read_plan(path: &Path) -> Result<Plan> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read token composition plan {}", path.display()))?;
    let plan: Plan = toml::from_str(&content)
        .with_context(|| format!("failed to parse token composition plan {}", path.display()))?;
    validate_plan_version(&plan)?;
    validate_lock_references(&plan)?;
    Ok(plan)
}

fn validate_plan_version(plan: &Plan) -> Result<()> {
    if plan.version != PLAN_VERSION {
        bail!(
            "unsupported token composition plan version {}; expected {PLAN_VERSION}",
            plan.version
        );
    }
    Ok(())
}

fn save_plan_atomic(path: &Path, plan: &Plan) -> Result<()> {
    validate_plan_version(plan)?;
    validate_lock_references(plan)?;
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create plan directory {}", parent.display()))?;
    }
    let content =
        toml::to_string_pretty(plan).context("failed to serialize token composition plan")?;
    let temporary = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("toml")
    ));
    fs::write(&temporary, content)
        .with_context(|| format!("failed to write temporary plan {}", temporary.display()))?;
    fs::rename(&temporary, path).with_context(|| {
        format!(
            "failed to replace token composition plan {} with {}",
            path.display(),
            temporary.display()
        )
    })?;
    Ok(())
}

fn render_plan_preview(plan: &Plan, path: Option<&Path>) -> Result<String> {
    validate_plan_version(plan)?;
    validate_lock_references(plan)?;
    let mut lines = Vec::new();
    match path {
        Some(path) => lines.push(format!("Token composition plan: {}", path.display())),
        None => lines.push("Token composition plan".to_owned()),
    }
    if let Some(network) = plan_preview_network(plan)? {
        lines.push(format!("Network: {network}"));
    }
    lines.push(String::new());
    lines.push("Operations:".to_owned());
    if plan.operations.is_empty() {
        lines.push("  <none>".to_owned());
        return Ok(lines.join("\n"));
    }

    let mut lock_create_count = 0usize;
    for (index, operation) in plan.operations.iter().enumerate() {
        let lock_index = if operation.is_lock_create() {
            lock_create_count += 1;
            Some(lock_create_count)
        } else {
            None
        };
        lines.push(format!(
            "  {}. {}",
            index + 1,
            operation.preview_line(lock_index)
        ));
    }
    Ok(lines.join("\n"))
}

fn plan_preview_network(plan: &Plan) -> Result<Option<String>> {
    let Some(genesis_hash) = plan.network_genesis_hash.as_deref() else {
        return Ok(None);
    };
    let app_config = app_config::load()?;
    let aliases = app_config
        .networks
        .iter()
        .filter(|(_, entry)| entry.genesis_hash == genesis_hash)
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    if aliases.is_empty() {
        Ok(Some(genesis_hash.to_owned()))
    } else {
        Ok(Some(format!("{} ({genesis_hash})", aliases.join(", "))))
    }
}

fn canonicalize_lock_references(plan: &mut Plan) -> Result<()> {
    validate_plan_version(plan)?;
    let mut lock_create_count = 0usize;
    for operation in &mut plan.operations {
        if operation.is_lock_create() {
            lock_create_count += 1;
        }
        let Some(lock) = operation.lock_reference_mut() else {
            continue;
        };
        match ParsedLockReference::parse(lock)? {
            ParsedLockReference::Latest => {
                if lock_create_count == 0 {
                    bail!("lock reference '@' requires a preceding lock-create operation")
                }
                *lock = format!("@{lock_create_count}");
            }
            ParsedLockReference::Created(index) => {
                if index > lock_create_count {
                    bail!(
                        "lock reference '@{index}' requires at least {index} preceding lock-create operation(s)"
                    );
                }
            }
            ParsedLockReference::Existing(_) => {}
        }
    }
    Ok(())
}

fn validate_lock_references(plan: &Plan) -> Result<()> {
    validate_plan_version(plan)?;
    let mut lock_create_count = 0usize;
    for operation in &plan.operations {
        if operation.is_lock_create() {
            lock_create_count += 1;
            if let PlanOperation::LockCreate { recipients, .. } = operation {
                recipients.validate()?;
            }
        }
        let lock = match operation {
            PlanOperation::LockFund { lock, .. }
            | PlanOperation::LockSend { lock, .. }
            | PlanOperation::LockReturn { lock, .. }
            | PlanOperation::LockCancel { lock } => lock,
            _ => continue,
        };
        match ParsedLockReference::parse(lock)? {
            ParsedLockReference::Latest => {
                bail!("saved plans must use explicit same-plan lock references such as @1")
            }
            ParsedLockReference::Created(index) => {
                if index > lock_create_count {
                    bail!(
                        "lock reference '@{index}' requires at least {index} preceding lock-create operation(s)"
                    );
                }
            }
            ParsedLockReference::Existing(_) => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn sample_plan() -> Plan {
        Plan {
            version: PLAN_VERSION,
            network_genesis_hash: Some("genesis-a".to_owned()),
            operations: vec![
                PlanOperation::Mint {
                    token: "CCD".to_owned(),
                    amount: "100".to_owned(),
                },
                PlanOperation::LockCreate {
                    recipients: PlanLockRecipients::limited(vec!["bob".to_owned()]),
                    expiry: "30d".to_owned(),
                    grants: vec![PlanLockGrant {
                        account: "alice".to_owned(),
                        capabilities: vec!["fund".to_owned()],
                    }],
                    tokens: vec!["CCD".to_owned()],
                    keep_alive: false,
                },
                PlanOperation::LockFund {
                    lock: "@1".to_owned(),
                    token: "CCD".to_owned(),
                    amount: "100".to_owned(),
                },
            ],
        }
    }

    #[test]
    fn parses_and_renders_plan_preview() {
        let plan: Plan = toml::from_str(
            r#"
version = 1
network_genesis_hash = "genesis-a"

[[operations]]
type = "mint"
token = "CCD"
amount = "100"

[[operations]]
type = "lock-create"
recipients = ["bob"]
expiry = "30d"
tokens = ["CCD"]

[[operations.grants]]
account = "alice"
capabilities = ["fund"]

[[operations]]
type = "lock-fund"
lock = "@1"
token = "CCD"
amount = "100"
"#,
        )
        .expect("valid plan");
        assert_eq!(plan, sample_plan());
        let rendered = render_plan_preview(&plan, None).expect("preview renders");
        assert!(rendered.contains("Network:"));
        assert!(rendered.contains("1. Mint 100 CCD"));
        assert!(rendered.contains("2. Create lock @1"));
        assert!(rendered.contains("3. Fund lock @1 with 100 CCD"));
    }

    #[test]
    fn parses_and_serializes_any_recipient_lock_create() {
        let plan: Plan = toml::from_str(
            r#"
version = 1

[[operations]]
type = "lock-create"
recipients = "any"
expiry = "30d"
tokens = ["CCD"]

[[operations.grants]]
account = "alice"
capabilities = ["fund"]
"#,
        )
        .expect("valid any-recipient plan");
        let rendered = render_plan_preview(&plan, None).expect("preview renders");
        assert!(rendered.contains("any eligible account"));
        let serialized = toml::to_string_pretty(&plan).expect("serialize succeeds");
        assert!(serialized.contains("recipients = \"any\""));
        assert!(!serialized.contains("recipients = []"));
    }

    #[test]
    fn rejects_unknown_string_recipient_sentinel() {
        let plan: Plan = toml::from_str(
            r#"
version = 1

[[operations]]
type = "lock-create"
recipients = "everyone"
expiry = "30d"
tokens = ["CCD"]

[[operations.grants]]
account = "alice"
capabilities = ["fund"]
"#,
        )
        .expect("string recipients parse before semantic validation");
        let err = validate_lock_references(&plan).expect_err("unknown sentinel must fail");
        assert!(err.to_string().contains("recipients"));
    }

    #[test]
    fn canonicalizes_latest_lock_reference_to_numbered_reference() {
        let mut plan = sample_plan();
        if let PlanOperation::LockFund { lock, .. } = &mut plan.operations[2] {
            *lock = "@".to_owned();
        }
        canonicalize_lock_references(&mut plan).expect("canonicalize succeeds");
        assert!(matches!(
            &plan.operations[2],
            PlanOperation::LockFund { lock, .. } if lock == "@1"
        ));
    }

    #[test]
    fn rejects_latest_reference_without_preceding_lock_create() {
        let mut plan = Plan {
            version: PLAN_VERSION,
            network_genesis_hash: Some("genesis-a".to_owned()),
            operations: vec![PlanOperation::LockFund {
                lock: "@".to_owned(),
                token: "CCD".to_owned(),
                amount: "1".to_owned(),
            }],
        };
        let err = canonicalize_lock_references(&mut plan).expect_err("must fail");
        assert!(err.to_string().contains("requires a preceding lock-create"));
    }

    #[test]
    fn explicit_second_lock_reference_targets_second_prior_lock_create() {
        let mut plan = sample_plan();
        plan.operations.push(PlanOperation::LockCreate {
            recipients: PlanLockRecipients::limited(vec!["carol".to_owned()]),
            expiry: "60d".to_owned(),
            grants: vec![PlanLockGrant {
                account: "alice".to_owned(),
                capabilities: vec!["fund".to_owned()],
            }],
            tokens: vec!["CCD".to_owned()],
            keep_alive: false,
        });
        plan.operations.push(PlanOperation::LockFund {
            lock: "@2".to_owned(),
            token: "CCD".to_owned(),
            amount: "50".to_owned(),
        });
        validate_lock_references(&plan).expect("@2 resolves after second lock-create");
    }

    #[test]
    fn local_lock_recipient_lookup_returns_recipients() {
        let recipients = nth_lock_create_recipients(&sample_plan(), 1).expect("recipients resolve");
        assert_eq!(
            recipients,
            shared::LockRecipientMode::Limited(vec!["bob".to_owned()])
        );
    }

    #[test]
    fn completes_top_level_commands_and_context_flags() {
        let commands = complete_compose_line("mi", 2, None)
            .into_iter()
            .map(|suggestion| suggestion.value)
            .collect::<Vec<_>>();
        assert!(commands.contains(&"mint".to_owned()));

        let flags = complete_compose_line("lock create --", 14, None)
            .into_iter()
            .map(|suggestion| suggestion.value)
            .collect::<Vec<_>>();
        assert!(flags.contains(&"--recipient".to_owned()));
        assert!(flags.contains(&"--any-recipient".to_owned()));
        assert!(flags.contains(&"--keep-alive".to_owned()));
    }

    #[test]
    fn completes_plan_lock_references() {
        let mut path = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos();
        path.push(format!("ccd-wallet-compose-complete-test-{unique}.toml"));
        let mut plan = sample_plan();
        plan.operations.push(PlanOperation::LockCreate {
            recipients: PlanLockRecipients::limited(vec!["carol".to_owned()]),
            expiry: "60d".to_owned(),
            grants: vec![PlanLockGrant {
                account: "alice".to_owned(),
                capabilities: vec!["fund".to_owned()],
            }],
            tokens: vec!["CCD".to_owned()],
            keep_alive: false,
        });
        save_plan_atomic(&path, &plan).expect("save succeeds");
        let refs = complete_compose_line("lock fund @", 11, Some(path.as_path()))
            .into_iter()
            .map(|suggestion| suggestion.value)
            .collect::<Vec<_>>();
        fs::remove_file(&path).expect("cleanup succeeds");
        assert!(refs.contains(&"@sender".to_owned()));
        assert!(refs.contains(&"@1".to_owned()));
        assert!(refs.contains(&"@2".to_owned()));
    }

    #[test]
    fn help_text_includes_reference_legends() {
        let help = help_text();
        assert!(help.contains("Operation commands:"));
        assert!(help.contains("Plan commands:"));
        assert!(help.contains("Session commands:"));
        assert!(!help.contains("add mint"));
        assert!(help.contains("Lock references:"));
        assert!(help.contains("@N"));
        assert!(help.contains("@2 references the second lock-create"));
        assert!(help.contains("<lock-id>"));
        assert!(help.contains("base58check lock id"));
        assert!(help.contains("Account references:"));
        assert!(help.contains("@sender"));
    }

    #[test]
    fn preserves_sender_account_reference_without_local_lookup() {
        let mut unlocks = AccountReferenceUnlocks::new();
        let conn = Connection::open_in_memory().expect("in-memory db opens");
        let resolved = resolve_plan_account(&conn, "genesis-a", "@sender", "source", &mut unlocks)
            .expect("@sender resolves symbolically");
        assert_eq!(resolved, "@sender");
    }

    #[test]
    fn rejects_out_of_range_numbered_reference() {
        let plan = Plan {
            version: PLAN_VERSION,
            network_genesis_hash: Some("genesis-a".to_owned()),
            operations: vec![PlanOperation::LockFund {
                lock: "@2".to_owned(),
                token: "CCD".to_owned(),
                amount: "1".to_owned(),
            }],
        };
        let err = validate_lock_references(&plan).expect_err("must fail");
        assert!(err.to_string().contains("requires at least 2"));
    }

    #[test]
    fn parses_add_mint_command() {
        let command = parse_repl_command("mint --token CCD --amount 100").expect("parse succeeds");
        assert!(matches!(
            command,
            ReplCommand::Add(PlanOperation::Mint { token, amount }) if token == "CCD" && amount == "100"
        ));
    }

    #[test]
    fn parses_add_lock_create_with_inline_structured_grant() {
        let command = parse_repl_command(
            "lock create --recipient bob --expiry 1d --grant alice:fund,send --token CCD --keep-alive",
        )
        .expect("parse succeeds");
        assert!(matches!(
            command,
            ReplCommand::Add(PlanOperation::LockCreate { grants, keep_alive: true, .. })
                if grants == vec![PlanLockGrant {
                    account: "alice".to_owned(),
                    capabilities: vec!["fund".to_owned(), "send".to_owned()],
                }]
        ));
    }

    #[test]
    fn parses_add_lock_create_with_any_recipient() {
        let command = parse_repl_command(
            "lock create --any-recipient --expiry 1d --grant alice:fund --token CCD --keep-alive",
        )
        .expect("parse succeeds");
        assert!(matches!(
            command,
            ReplCommand::Add(PlanOperation::LockCreate { recipients: PlanLockRecipients::Any(value), .. })
                if value == "any"
        ));
    }

    #[test]
    fn rejects_add_lock_create_with_mixed_recipient_modes() {
        let err = parse_repl_command(
            "lock create --any-recipient --recipient bob --expiry 1d --grant alice:fund --token CCD --keep-alive",
        )
        .expect_err("mixed recipient modes must fail");
        assert!(err.to_string().contains("--any-recipient"));
    }

    #[test]
    fn rejects_add_lock_create_with_unknown_inline_grant_capability() {
        let err = parse_repl_command(
            "lock create --recipient bob --expiry 1d --grant alice:fund,nonsense --token CCD --keep-alive",
        )
        .expect_err("parse must fail");
        assert!(err.to_string().contains("nonsense"));
    }

    #[test]
    fn parses_add_lock_fund_command() {
        let command =
            parse_repl_command("lock fund @ --token CCD --amount 100").expect("parse succeeds");
        assert!(matches!(
            command,
            ReplCommand::Add(PlanOperation::LockFund { lock, token, amount })
                if lock == "@" && token == "CCD" && amount == "100"
        ));
    }

    #[test]
    fn parses_plan_specific_unresolved_domain_inputs() {
        assert!(PlanTokenIdInput::parse("CCD").is_ok());
        assert!(PlanTokenIdInput::parse("").is_err());
        assert_eq!(
            PlanAccountReferenceInput::parse("@sender"),
            PlanAccountReferenceInput::Sender
        );
        assert_eq!(
            PlanAccountReferenceInput::parse("alice"),
            PlanAccountReferenceInput::AddressOrLabel("alice".to_owned())
        );
        assert!(matches!(
            PlanLockReferenceInput::parse("@2").unwrap(),
            PlanLockReferenceInput::Created(2)
        ));
        assert_eq!(
            PlanTokenAmountInput::parse("1.25").unwrap().as_str(),
            "1.25"
        );
        assert!(PlanTokenAmountInput::parse("-1").is_err());
    }

    #[test]
    fn parses_help_alias_and_preview_commands() {
        assert!(matches!(
            parse_repl_command("?").unwrap(),
            ReplCommand::Help
        ));
        assert!(matches!(
            parse_repl_command("preview").unwrap(),
            ReplCommand::Preview
        ));
    }

    #[test]
    fn non_interactive_submit_requires_sender_before_prompting() {
        let args = TokenComposeSubmitArgs {
            plan: std::path::PathBuf::from("plan.toml"),
            sender: None,
            network: Some("testnet".to_owned()),
            node: None,
            no_wait: true,
            non_interactive: true,
            no_defaults: false,
        };
        let err = PreparedTokenComposeSubmit::from_cli(args).expect_err("missing sender must fail");
        assert!(err.to_string().contains("--sender"));
    }

    #[test]
    fn preview_includes_saved_plan_path_and_operations() {
        let mut path = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos();
        path.push(format!("ccd-wallet-compose-preview-test-{unique}.toml"));
        let plan = sample_plan();
        save_plan_atomic(&path, &plan).expect("save succeeds");
        let loaded = read_plan(&path).expect("load succeeds");
        let rendered =
            render_plan_preview(&loaded, Some(path.as_path())).expect("preview succeeds");
        fs::remove_file(&path).expect("cleanup succeeds");
        assert!(rendered.contains(&format!("Token composition plan: {}", path.display())));
        assert!(rendered.contains("Network:"));
        assert!(rendered.contains("Fund lock @1 with 100 CCD"));
    }

    #[test]
    fn saved_plan_uses_structured_lock_grants() {
        let plan = sample_plan();
        let serialized = toml::to_string_pretty(&plan).expect("serialize succeeds");
        assert!(serialized.contains("keep_alive = false"));
        assert!(serialized.contains("[[operations.grants]]"));
        assert!(serialized.contains("account = \"alice\""));
        assert!(serialized.contains("capabilities = [\"fund\"]"));
        assert!(!serialized.contains("grants = [\"alice:fund\"]"));
    }

    #[test]
    fn saves_and_loads_plan_atomically() {
        let mut path = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock ok")
            .as_nanos();
        path.push(format!("ccd-wallet-compose-test-{unique}.toml"));
        let plan = sample_plan();
        save_plan_atomic(&path, &plan).expect("save succeeds");
        let loaded = read_plan(&path).expect("load succeeds");
        fs::remove_file(&path).expect("cleanup succeeds");
        assert_eq!(loaded, plan);
    }
}
