//! Interactive protocol-level token composition planning and submission.

use crate::{
    cli::{TokenComposeArgs, TokenComposePreviewArgs, TokenComposeSubmitArgs},
    commands::token::shared,
};
use anyhow::{Context, Result, bail};
use cliclack::input;
use concordium_rust_sdk::{
    base::{
        common::types::TransactionTime,
        contracts_common::AccountAddress,
        protocol_level_locks::LockId,
        protocol_level_tokens::{
            TokenAmount, TokenId, meta_operations, meta_operations::MetaUpdateOperations,
        },
        transactions::{BlockItem, ExactSizeTransactionSigner, construct},
    },
    protocol_level_tokens::lock_client,
};
use reedline::{DefaultPrompt, Reedline, Signal};
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

/// Submit a token composition plan.
pub(super) async fn submit(conn: &Connection, args: TokenComposeSubmitArgs) -> Result<()> {
    validate_submit_args(&args)?;
    let plan = read_plan(&args.plan)?;
    validate_lock_references(&plan)?;
    if plan.operations.is_empty() {
        bail!("token composition plan has no operations to submit");
    }

    let mut context = shared::resolve_mutation_context(
        conn,
        args.sender.as_deref(),
        args.network.as_deref(),
        args.node,
        args.non_interactive,
        args.no_defaults,
        false,
    )
    .await?;

    let operations = resolve_meta_update_operations(conn, &mut context, &plan).await?;
    let summary = render_plan_preview(&plan, Some(args.plan.as_path()))?;
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
    if !args.no_wait {
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

fn validate_submit_args(args: &TokenComposeSubmitArgs) -> Result<()> {
    if args.non_interactive && args.sender.is_none() {
        bail!("sender must be provided with --sender in --non-interactive mode");
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
                let recipients = resolve_accounts(conn, context, recipients, "recipient")?;
                let expiry = shared::parse_expiry_time(expiry)?;
                let grants = grants
                    .iter()
                    .map(|grant| grant.to_unresolved())
                    .map(|grant| {
                        grant.and_then(|grant| shared::resolve_lock_grant(conn, context, &grant))
                    })
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

fn resolve_lock_reference(input: &str, predicted_locks: &[LockId]) -> Result<LockId> {
    match ParsedLockReference::parse(input)? {
        ParsedLockReference::Latest => bail!("saved plans must use explicit lock references"),
        ParsedLockReference::Created(index) => predicted_locks
            .get(index - 1)
            .cloned()
            .with_context(|| format!("same-plan lock reference '@{index}' could not be resolved")),
        ParsedLockReference::Existing(lock) => lock.parse().context("invalid lock identifier"),
    }
}

fn parse_token_id(input: &str) -> Result<TokenId> {
    TokenId::from_str(input).context("invalid token identifier")
}

async fn resolve_amount(
    context: &mut shared::MutationContext,
    token_id: TokenId,
    amount: &str,
) -> Result<TokenAmount> {
    let mut client = context.client.clone();
    let token_info = shared::query_token_info(&mut client, token_id).await?;
    shared::parse_token_amount(amount, token_info.token_state.decimals)
}

fn resolve_account(
    conn: &Connection,
    context: &mut shared::MutationContext,
    value: &str,
    label: &str,
) -> Result<AccountAddress> {
    shared::resolve_account_address(
        conn,
        context,
        Some(value),
        "Account address or local label:",
        label,
        true,
    )
}

fn resolve_accounts(
    conn: &Connection,
    context: &mut shared::MutationContext,
    values: &[String],
    label: &str,
) -> Result<Vec<AccountAddress>> {
    shared::parse_account_addresses(conn, context, values, label)
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
    cliclack::log::info(format!(
        "Token composer loaded {} operation(s) from {}.",
        plan.operations.len(),
        plan_path.display()
    ))?;
    println!("Type 'help' or '?' for commands. Type 'exit' or press Ctrl-C to quit.");

    let mut line_editor = Reedline::create();
    let prompt = DefaultPrompt::default();
    loop {
        match line_editor.read_line(&prompt)? {
            Signal::Success(line) => {
                match handle_repl_line(conn, plan_path, &mut plan, &line).await {
                    Ok(ReplControl::Continue) => {}
                    Ok(ReplControl::Exit) => break,
                    Err(error) => cliclack::log::error(format!("{error:#}"))?,
                }
            }
            Signal::CtrlC | Signal::CtrlD => break,
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplControl {
    Continue,
    Exit,
}

async fn handle_repl_line(
    conn: &Connection,
    plan_path: &Path,
    plan: &mut Plan,
    line: &str,
) -> Result<ReplControl> {
    match parse_repl_command(line)? {
        ReplCommand::Empty => {}
        ReplCommand::Help => println!("{}", help_text()),
        ReplCommand::Exit => return Ok(ReplControl::Exit),
        ReplCommand::Preview => println!("{}", render_plan_preview(plan, Some(plan_path))?),
        ReplCommand::Add(operation) => {
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
            submit(
                conn,
                TokenComposeSubmitArgs {
                    plan: plan_path.to_owned(),
                    sender: args.sender,
                    network: args.network,
                    node: args.node,
                    no_wait: args.no_wait,
                    non_interactive: args.non_interactive,
                    no_defaults: args.no_defaults,
                },
            )
            .await?;
        }
    }
    Ok(ReplControl::Continue)
}

#[derive(Debug, Clone)]
enum ReplCommand {
    Empty,
    Help,
    Exit,
    Preview,
    Add(PlanOperation),
    Submit(SubmitCommandArgs),
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

fn parse_repl_command(line: &str) -> Result<ReplCommand> {
    let Some(words) = shlex::split(line) else {
        bail!("failed to parse command line; check quoting")
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
        "add" => parse_add_command(words).map(ReplCommand::Add),
        "submit" => parse_submit_command(words).map(ReplCommand::Submit),
        other => bail!("unknown token compose command '{other}'. Type 'help' for usage."),
    }
}

fn parse_add_command(mut words: WordCursor) -> Result<PlanOperation> {
    let family = words.required_word("operation family")?;
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
            recipients: args.take_many_or_prompt("recipient", "Recipients (comma-separated):")?,
            expiry: args.take_or_prompt("expiry", "Lock expiry:")?,
            grants: args.take_lock_grants()?,
            tokens: args.take_many_or_prompt("token", "Tokens (comma-separated):")?,
            keep_alive: args.flag("keep-alive"),
        }),
        "fund" => Ok(PlanOperation::LockFund {
            lock: args.take_positional_or_prompt("lock", "Lock id or @ reference:")?,
            token: args.take_or_prompt("token", "Token identifier:")?,
            amount: args.take_or_prompt("amount", "Token amount:")?,
        }),
        "send" => Ok(PlanOperation::LockSend {
            lock: args.take_positional_or_prompt("lock", "Lock id or @ reference:")?,
            token: args.take_or_prompt("token", "Token identifier:")?,
            source: args.take_or_prompt("source", "Source account address or label:")?,
            recipient: args.take_or_prompt("recipient", "Recipient account address or label:")?,
            amount: args.take_or_prompt("amount", "Token amount:")?,
        }),
        "return" => Ok(PlanOperation::LockReturn {
            lock: args.take_positional_or_prompt("lock", "Lock id or @ reference:")?,
            token: args.take_or_prompt("token", "Token identifier:")?,
            source: args.take_or_prompt("source", "Source account address or label:")?,
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
                    "keep-alive" => flags.push(name.to_owned()),
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

    fn take_or_prompt(&mut self, name: &str, prompt: &str) -> Result<String> {
        if let Some(value) = self.take_optional(name) {
            return Ok(value);
        }
        let value: String = input(prompt).interact()?;
        Ok(value)
    }

    fn take_positional_or_prompt(&mut self, name: &str, prompt: &str) -> Result<String> {
        if let Some(value) = self.positionals.pop_front() {
            return Ok(value);
        }
        self.take_or_prompt(name, prompt)
    }

    fn take_many_or_prompt(&mut self, name: &str, prompt: &str) -> Result<Vec<String>> {
        let values = self.take_all(name);
        if !values.is_empty() {
            return Ok(values);
        }
        let value: String = input(prompt).interact()?;
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
    r#"Commands:
  add transfer --token TOKEN --recipient ACCOUNT --amount AMOUNT
  add mint --token TOKEN --amount AMOUNT
  add burn --token TOKEN --amount AMOUNT
  add pause --token TOKEN
  add unpause --token TOKEN
  add allow-list add --token TOKEN --target ACCOUNT [--target ACCOUNT...]
  add allow-list remove --token TOKEN --target ACCOUNT [--target ACCOUNT...]
  add deny-list add --token TOKEN --target ACCOUNT [--target ACCOUNT...]
  add deny-list remove --token TOKEN --target ACCOUNT [--target ACCOUNT...]
  add admin-roles assign --token TOKEN --target ACCOUNT --role ROLE [--role ROLE...]
  add admin-roles revoke --token TOKEN --target ACCOUNT --role ROLE [--role ROLE...]
  add metadata update --token TOKEN --url URL [--checksum-sha256 HEX]
  add lock create --recipient ACCOUNT --expiry TIME --grant GRANT --token TOKEN [--keep-alive]
  add lock fund LOCK --token TOKEN --amount AMOUNT
  add lock send LOCK --source ACCOUNT --recipient ACCOUNT --token TOKEN --amount AMOUNT
  add lock return LOCK --source ACCOUNT --token TOKEN --amount AMOUNT
  add lock cancel LOCK
  preview
  submit [--sender LABEL] [--network NAME] [--node ENDPOINT] [--no-wait] [--non-interactive] [--no-defaults]
  help | ?
  exit

Lock references:
  @   most recent preceding lock-create, canonicalized to @N on save
  @1  first lock created in this plan
  @2  second lock created in this plan
"#
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Plan {
    version: u32,
    #[serde(default)]
    operations: Vec<PlanOperation>,
}

impl Plan {
    fn empty() -> Self {
        Self {
            version: PLAN_VERSION,
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
        recipients: Vec<String>,
        expiry: String,
        grants: Vec<PlanLockGrant>,
        tokens: Vec<String>,
        #[serde(default, skip_serializing_if = "is_false")]
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
                recipients.join(", "),
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

fn is_false(value: &bool) -> bool {
    !*value
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
            operations: vec![
                PlanOperation::Mint {
                    token: "CCD".to_owned(),
                    amount: "100".to_owned(),
                },
                PlanOperation::LockCreate {
                    recipients: vec!["bob".to_owned()],
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
        assert!(rendered.contains("1. Mint 100 CCD"));
        assert!(rendered.contains("2. Create lock @1"));
        assert!(rendered.contains("3. Fund lock @1 with 100 CCD"));
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
            recipients: vec!["carol".to_owned()],
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
    fn rejects_out_of_range_numbered_reference() {
        let plan = Plan {
            version: PLAN_VERSION,
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
        let command =
            parse_repl_command("add mint --token CCD --amount 100").expect("parse succeeds");
        assert!(matches!(
            command,
            ReplCommand::Add(PlanOperation::Mint { token, amount }) if token == "CCD" && amount == "100"
        ));
    }

    #[test]
    fn parses_add_lock_create_with_inline_structured_grant() {
        let command = parse_repl_command(
            "add lock create --recipient bob --expiry 1d --grant alice:fund,send --token CCD",
        )
        .expect("parse succeeds");
        assert!(matches!(
            command,
            ReplCommand::Add(PlanOperation::LockCreate { grants, .. })
                if grants == vec![PlanLockGrant {
                    account: "alice".to_owned(),
                    capabilities: vec!["fund".to_owned(), "send".to_owned()],
                }]
        ));
    }

    #[test]
    fn rejects_add_lock_create_with_unknown_inline_grant_capability() {
        let err = parse_repl_command(
            "add lock create --recipient bob --expiry 1d --grant alice:fund,nonsense --token CCD",
        )
        .expect_err("parse must fail");
        assert!(err.to_string().contains("nonsense"));
    }

    #[test]
    fn parses_add_lock_fund_command() {
        let command =
            parse_repl_command("add lock fund @ --token CCD --amount 100").expect("parse succeeds");
        assert!(matches!(
            command,
            ReplCommand::Add(PlanOperation::LockFund { lock, token, amount })
                if lock == "@" && token == "CCD" && amount == "100"
        ));
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
        let err = validate_submit_args(&args).expect_err("missing sender must fail");
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
        assert!(rendered.contains("Fund lock @1 with 100 CCD"));
    }

    #[test]
    fn saved_plan_uses_structured_lock_grants() {
        let plan = sample_plan();
        let serialized = toml::to_string_pretty(&plan).expect("serialize succeeds");
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
