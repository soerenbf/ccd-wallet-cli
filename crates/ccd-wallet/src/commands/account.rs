use crate::{
    cli::{
        AccountExportArgs, AccountImportGenesisArgs, AccountImportSubcommand, AccountListArgs,
        AccountNewArgs, AccountRenameArgs, AccountShowArgs, AccountSubcommand,
    },
    commands::ui::{
        ContextLine, FuzzySelectItem, ResolutionSource, SelectItem, fuzzy_select_always,
        fuzzy_select_or_single, log_resolved_context, select_or_single,
    },
};
use anyhow::{Context, Result, bail};
use ccd_wallet_core::{
    account_creation::{
        CredentialDeploymentInput, build_credential_deployment, credential_counter_to_u8,
        parse_identity_object,
    },
    config as node_config,
    store::{
        accounts,
        config::{AppConfig, NetworkEntry, load},
        crypto::KEY_LEN,
        identities::{self, IdentityRecord, IdentityStatus},
        seeds, wallet_state,
    },
    wallet::{ConcordiumHdWallet, Net},
};
use ccd_wallet_identity_provider::client::{self, PollResult};
use cliclack::{input, password, select, spinner};
use concordium_rust_sdk::{
    common::types::{AccountAddress, KeyIndex, KeyPair},
    id::{
        constants::{ArCurve, IpPairing},
        types::{AccountKeys, ArIdentity, ArInfo, CredentialData, IpInfo, SignatureThreshold},
    },
    protocol_level_tokens::{AccountToken, TokenAmount},
    types::{
        AccountInfo, BlockItemSummaryDetails, WalletAccount,
        transactions::{BlockItem, Payload},
    },
    v2::{self, AccountIdentifier, BlockIdentifier},
};
use futures_util::StreamExt;
use rusqlite::Connection;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};
use zeroize::Zeroizing;

#[derive(Clone, Debug, Eq, PartialEq)]
enum ScopeSelection {
    All,
    One(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AccountListStatus {
    Pending,
    Finalized,
}

impl AccountListStatus {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "finalized" => Ok(Self::Finalized),
            other => bail!("unsupported account status '{other}'"),
        }
    }
}

pub async fn run(conn: &mut Connection, command: AccountSubcommand) -> Result<()> {
    match command {
        AccountSubcommand::Export(args) => export_account(conn, args).await,
        AccountSubcommand::Import(command) => match command.command {
            AccountImportSubcommand::Genesis(args) => import_genesis(conn, args).await,
        },
        AccountSubcommand::List(args) => list_accounts(conn, args).await,
        AccountSubcommand::New(args) => new(conn, *args).await,
        AccountSubcommand::Show(args) => show_account(conn, *args).await,
        AccountSubcommand::Rename(args) => rename_account(conn, args).await,
    }
}

async fn list_accounts(conn: &mut Connection, args: AccountListArgs) -> Result<()> {
    let seed_scope = resolve_account_list_seed_scope(conn, args.seed.as_deref())?;
    let network_scope = resolve_network_scope(
        conn,
        args.network.as_deref(),
        args.non_interactive,
        args.no_defaults,
        true,
    )?;
    let status_filter = args
        .status
        .as_deref()
        .map(AccountListStatus::parse)
        .transpose()?;

    log_scope_context(&seed_scope, &network_scope)?;

    let seeds_by_id = seed_labels_by_id(conn)?;
    let networks_by_hash = network_names_by_genesis_hash()?;
    let mut accounts = accounts::list(conn)?
        .into_iter()
        .filter(|record| matches_seed_scope(record, &seed_scope, &seeds_by_id))
        .filter(|record| matches_network_scope(record, &network_scope, &networks_by_hash))
        .filter(|record| matches_account_status(record, status_filter))
        .collect::<Vec<_>>();
    accounts.sort_by(|a, b| a.label.cmp(&b.label));

    let address_map = if args.show_addresses {
        load_account_addresses(conn, &accounts, &seeds_by_id, &seed_scope)?
    } else {
        BTreeMap::new()
    };

    for record in accounts {
        let seed_label = seeds_by_id
            .get(&record.seed_id)
            .cloned()
            .unwrap_or_else(|| "<unknown-seed>".to_owned());
        let network_name = networks_by_hash
            .get(&record.network_genesis_hash)
            .cloned()
            .unwrap_or_else(|| record.network_genesis_hash.clone());
        println!(
            "{}",
            render_account_fuzzy_text(
                &record,
                &seed_label,
                &network_name,
                address_map.get(&record.id).map(String::as_str),
                true,
            )
        );
    }
    Ok(())
}

async fn show_account(conn: &mut Connection, args: AccountShowArgs) -> Result<()> {
    let context = resolve_account_show_network_context(
        conn,
        args.network.as_deref(),
        args.node,
        args.non_interactive,
        args.no_defaults,
    )
    .await?;
    let block = crate::smart_contracts::shared::parse_block_identifier(args.block.as_deref())?;
    let target = resolve_account_show_target(conn, &context, &args.account)?;
    let view = match target {
        AccountShowTarget::RawAddress(address) => {
            let info = query_account_info(
                context.endpoint.clone(),
                &context.endpoint_label,
                address,
                block,
            )
            .await?;
            AccountShowView::from_info(context.display_name, None, &info)?
        }
        AccountShowTarget::LocalPending(metadata, transaction_hash) => {
            AccountShowView::pending(context.display_name, metadata, transaction_hash)
        }
        AccountShowTarget::LocalFinalized(record, metadata) => {
            let address = decrypt_local_account_address(conn, &context.display_name, &record)?;
            let info = query_account_info(
                context.endpoint.clone(),
                &context.endpoint_label,
                address,
                block,
            )
            .await?;
            AccountShowView::from_info(context.display_name, Some(metadata), &info)?
        }
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&view)?);
    } else {
        println!("{}", render_account_show(&view, args.verbose));
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct AccountShowNetworkContext {
    endpoint: v2::Endpoint,
    endpoint_label: String,
    display_name: String,
    genesis_hash: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountShowLocalMetadata {
    label: String,
    seed: Option<String>,
    source: String,
}

#[derive(Debug)]
enum AccountShowTarget {
    RawAddress(AccountAddress),
    LocalFinalized(accounts::AccountRecord, AccountShowLocalMetadata),
    LocalPending(AccountShowLocalMetadata, Option<String>),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountShowView {
    network: String,
    address: Option<String>,
    local: Option<AccountShowLocalMetadata>,
    status: String,
    transaction_hash: Option<String>,
    ccd: Option<AccountShowCcdView>,
    tokens: Vec<AccountShowTokenView>,
    protocol: Option<AccountShowProtocolView>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountShowCcdView {
    balance: String,
    available: String,
    locked: String,
    release_schedule: Vec<AccountShowReleaseView>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountShowReleaseView {
    amount: String,
    timestamp: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountShowTokenView {
    id: String,
    balance: String,
    available: String,
    locked: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountShowProtocolView {
    account_index: String,
    nonce: String,
    credential_count: usize,
    threshold: String,
    staking: String,
}

impl AccountShowView {
    fn pending(
        network: String,
        local: AccountShowLocalMetadata,
        transaction_hash: Option<String>,
    ) -> Self {
        Self {
            network,
            address: None,
            local: Some(local),
            status: "pending".to_owned(),
            transaction_hash,
            ccd: None,
            tokens: Vec::new(),
            protocol: None,
        }
    }

    fn from_info(
        network: String,
        local: Option<AccountShowLocalMetadata>,
        info: &AccountInfo,
    ) -> Result<Self> {
        let ccd = AccountShowCcdView {
            balance: format!("{} CCD", info.account_amount),
            available: format!("{} CCD", info.available_balance),
            locked: format!("{} CCD", ccd_locked_amount(info)),
            release_schedule: info
                .account_release_schedule
                .schedule
                .iter()
                .map(|release| AccountShowReleaseView {
                    amount: format!("{} CCD", release.amount),
                    timestamp: release
                        .timestamp
                        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                })
                .collect(),
        };
        let tokens = info
            .tokens
            .iter()
            .map(account_show_token_view)
            .collect::<Result<Vec<_>>>()?;
        let protocol = Some(AccountShowProtocolView {
            account_index: format!("{}", info.account_index),
            nonce: format!("{}", info.account_nonce),
            credential_count: info.account_credentials.len(),
            threshold: u8::from(info.account_threshold).to_string(),
            staking: if info.account_stake.is_some() {
                "configured".to_owned()
            } else {
                "none".to_owned()
            },
        });
        Ok(Self {
            network,
            address: Some(format!("{}", info.account_address)),
            local,
            status: "finalized".to_owned(),
            transaction_hash: None,
            ccd: Some(ccd),
            tokens,
            protocol,
        })
    }
}

async fn resolve_account_show_network_context(
    conn: &Connection,
    network: Option<&str>,
    node: Option<v2::Endpoint>,
    non_interactive: bool,
    no_defaults: bool,
) -> Result<AccountShowNetworkContext> {
    if let Some(endpoint) = node {
        let endpoint_label = node_config::endpoint_label(&endpoint);
        let genesis_hash = fetch_node_genesis_hash(endpoint.clone(), &endpoint_label).await?;
        if let Some(network_name) = network {
            let app_config = load()?;
            let entry = app_config
                .networks
                .get(network_name)
                .with_context(|| format!("network '{}' is not registered", network_name))?;
            if entry.genesis_hash != genesis_hash {
                bail!(
                    "node at {} belongs to genesis hash {}, which does not match configured network '{}' ({})",
                    endpoint_label,
                    genesis_hash,
                    network_name,
                    entry.genesis_hash
                );
            }
            return Ok(AccountShowNetworkContext {
                endpoint,
                endpoint_label,
                display_name: network_name.to_owned(),
                genesis_hash,
            });
        }
        let display_name =
            network_name_for_genesis(&genesis_hash)?.unwrap_or(endpoint_label.clone());
        return Ok(AccountShowNetworkContext {
            endpoint,
            endpoint_label,
            display_name,
            genesis_hash,
        });
    }

    let (network_name, network_entry, endpoint, endpoint_label, _source) =
        resolve_account_network_context(conn, network, None, non_interactive, no_defaults).await?;
    Ok(AccountShowNetworkContext {
        endpoint,
        endpoint_label,
        display_name: network_name,
        genesis_hash: network_entry.genesis_hash,
    })
}

fn resolve_account_show_target(
    conn: &Connection,
    context: &AccountShowNetworkContext,
    target: &str,
) -> Result<AccountShowTarget> {
    if let Ok(address) = AccountAddress::from_str(target) {
        return Ok(AccountShowTarget::RawAddress(address));
    }

    let record = accounts::find_by_network_and_label(conn, &context.genesis_hash, target)?
        .with_context(|| {
            format!(
                "account '{}' is not configured for network '{}'",
                target, context.display_name
            )
        })?;
    let metadata = local_metadata_for_record(conn, &record)?;
    match record.status {
        accounts::AccountStatus::Pending => Ok(AccountShowTarget::LocalPending(
            metadata,
            record.transaction_hash.clone(),
        )),
        accounts::AccountStatus::Finalized => {
            Ok(AccountShowTarget::LocalFinalized(record, metadata))
        }
    }
}

fn local_metadata_for_record(
    conn: &Connection,
    record: &accounts::AccountRecord,
) -> Result<AccountShowLocalMetadata> {
    let source = match record.source_kind {
        accounts::AccountSourceKind::Derived => "derived",
        accounts::AccountSourceKind::Imported => "imported",
    };
    let seed = if record.source_kind == accounts::AccountSourceKind::Derived {
        Some(
            seeds::list(conn)?
                .into_iter()
                .find(|seed| seed.id == record.seed_id)
                .map(|seed| seed.label)
                .context("selected account references unknown seed")?,
        )
    } else {
        None
    };
    Ok(AccountShowLocalMetadata {
        label: record.label.clone(),
        seed,
        source: source.to_owned(),
    })
}

fn decrypt_local_account_address(
    conn: &Connection,
    network_name: &str,
    record: &accounts::AccountRecord,
) -> Result<AccountAddress> {
    let address = match record.source_kind {
        accounts::AccountSourceKind::Derived => {
            let seed_label = seeds::list(conn)?
                .into_iter()
                .find(|seed| seed.id == record.seed_id)
                .map(|seed| seed.label)
                .context("selected account references unknown seed")?;
            let password: String = password(format!("Password for seed '{}':", seed_label))
                .allow_empty()
                .interact()?;
            let unlocked = seeds::unlock_context(conn, &seed_label, &password)?;
            accounts::decrypt_private_payload(conn, record.id, &unlocked.dek)?.account_address
        }
        accounts::AccountSourceKind::Imported => {
            let vault_password: String = password(format!(
                "Imported accounts vault password for '{}':",
                network_name
            ))
            .allow_empty()
            .interact()?;
            let unlocked = accounts::unlock_imported_vault(
                conn,
                &record.network_genesis_hash,
                &vault_password,
            )?;
            accounts::decrypt_imported_payload(conn, record.id, &unlocked.dek)?.account_address
        }
    };
    AccountAddress::from_str(&address).context("stored account address is invalid")
}

async fn query_account_info(
    endpoint: v2::Endpoint,
    endpoint_label: &str,
    address: AccountAddress,
    block: BlockIdentifier,
) -> Result<AccountInfo> {
    let mut client = node_config::connect_v2_client(endpoint)
        .await
        .with_context(|| format!("failed to connect to Concordium node at {endpoint_label}"))?;
    Ok(client
        .get_account_info(&AccountIdentifier::from(address), block)
        .await
        .with_context(|| format!("failed to query account information from {endpoint_label}"))?
        .response)
}

fn account_show_token_view(token: &AccountToken) -> Result<AccountShowTokenView> {
    let module_state = token
        .state
        .decode_module_state()
        .context("failed to decode token account state")?;
    let available = module_state.available.unwrap_or(token.state.balance);
    let locked = token_locked_amount(token.state.balance, available)?;
    Ok(AccountShowTokenView {
        id: token.token_id.to_string(),
        balance: token.state.balance.to_string(),
        available: available.to_string(),
        locked: locked.to_string(),
    })
}

fn ccd_locked_amount(info: &AccountInfo) -> concordium_rust_sdk::common::types::Amount {
    ccd_locked_from_amounts(info.account_amount, info.available_balance)
}

fn ccd_locked_from_amounts(
    balance: concordium_rust_sdk::common::types::Amount,
    available: concordium_rust_sdk::common::types::Amount,
) -> concordium_rust_sdk::common::types::Amount {
    balance
        .checked_sub(available)
        .unwrap_or_else(concordium_rust_sdk::common::types::Amount::zero)
}

fn token_locked_amount(balance: TokenAmount, available: TokenAmount) -> Result<TokenAmount> {
    if balance.decimals() != available.decimals() {
        bail!("token balance and available balance use different decimal precision")
    }
    let locked = balance
        .value()
        .checked_sub(available.value())
        .with_context(|| "token available balance exceeds total balance")?;
    Ok(TokenAmount::from_raw(locked, balance.decimals()))
}

fn network_name_for_genesis(genesis_hash: &str) -> Result<Option<String>> {
    Ok(load()?
        .networks
        .into_iter()
        .find(|(_, entry)| entry.genesis_hash == genesis_hash)
        .map(|(name, _)| name))
}

fn render_account_show(view: &AccountShowView, verbose: bool) -> String {
    if view.status == "pending" {
        return render_pending_account_show(view);
    }

    let mut lines = vec![render_account_show_header(view), String::new()];
    if let Some(ccd) = &view.ccd {
        lines.push(format!("CCD balance: {}", ccd.balance));
        lines.push(format!("  available: {}", ccd.available));
        if ccd.locked != "0.0 CCD" {
            lines.push(format!("  locked: {}", ccd.locked));
        }
        if !ccd.release_schedule.is_empty() {
            lines.push("  release schedule:".to_owned());
            lines.extend(
                ccd.release_schedule.iter().map(|release| {
                    format!("    {} unlocks at {}", release.amount, release.timestamp)
                }),
            );
        }
    }

    for token in &view.tokens {
        lines.push(String::new());
        lines.push(format!("{} balance: {}", token.id, token.balance));
        if token.locked != token_zero_with_same_decimals(&token.locked) {
            lines.push(format!("  available: {}", token.available));
            lines.push(format!("  locked: {}", token.locked));
        }
    }

    if verbose && let Some(protocol) = &view.protocol {
        lines.push(String::new());
        lines.push("Protocol details:".to_owned());
        lines.push(format!("  account index: {}", protocol.account_index));
        lines.push(format!("  next nonce: {}", protocol.nonce));
        lines.push(format!("  credentials: {}", protocol.credential_count));
        lines.push(format!("  signature threshold: {}", protocol.threshold));
        lines.push(format!("  staking: {}", protocol.staking));
    }

    lines.join("\n")
}

fn render_pending_account_show(view: &AccountShowView) -> String {
    let mut lines = vec![
        render_account_show_header(view),
        String::new(),
        "Status: pending".to_owned(),
    ];
    if let Some(transaction_hash) = &view.transaction_hash {
        lines.push(format!("Transaction: {transaction_hash}"));
        lines.push(String::new());
        lines.push(format!(
            "Account has not finalized yet. Run `ccd-wallet transaction show {transaction_hash}` for submission status."
        ));
    } else {
        lines.push(String::new());
        lines.push("Finalized on-chain account state is not available yet.".to_owned());
    }
    lines.join("\n")
}

fn render_account_show_header(view: &AccountShowView) -> String {
    let subject = view.address.as_deref().unwrap_or(&view.network);
    let base = if view.address.is_some() {
        format!("{} @ {}", subject, view.network)
    } else {
        view.network.clone()
    };
    match &view.local {
        Some(local) => match local.seed.as_deref() {
            Some(seed) => format!("[{seed} : {}] {base}", local.label),
            None => format!("[{}] {base}", local.label),
        },
        None => base,
    }
}

fn token_zero_with_same_decimals(value: &str) -> String {
    match value.split_once('.') {
        Some((_, fraction)) => format!("0.{:0<width$}", "", width = fraction.len()),
        None => "0".to_owned(),
    }
}

async fn export_account(conn: &mut Connection, args: AccountExportArgs) -> Result<()> {
    let (network_name, network_entry, network_source) = resolve_export_network(
        conn,
        args.network.as_deref(),
        args.non_interactive,
        args.no_defaults,
    )?;
    log_resolved_context(&[ContextLine {
        label: "network:",
        value: network_name.clone(),
        source: network_source,
    }])?;

    let account = resolve_export_account(
        conn,
        &network_name,
        &network_entry.genesis_hash,
        args.label.as_deref(),
        args.non_interactive,
        false,
    )?;
    let output_path =
        resolve_export_output_path(args.output, &account.label, args.non_interactive)?;
    let signer = build_export_wallet_account(conn, &network_name, &network_entry, &account)?;
    let signer_json = serialize_wallet_account_minimal(&signer)?;
    write_export_file(&output_path, &signer_json)?;
    cliclack::log::success(format!(
        "Exported account '{}' to {}.",
        account.label,
        output_path.display()
    ))?;
    Ok(())
}

async fn import_genesis(conn: &mut Connection, args: AccountImportGenesisArgs) -> Result<()> {
    validate_genesis_import_file(&args.file)?;

    let (network_name, network_entry, _endpoint, endpoint_label, network_source) =
        resolve_import_network_context(conn, args.network.as_deref(), args.non_interactive).await?;
    log_resolved_context(&[ContextLine {
        label: "network:",
        value: format!("{network_name} @ {endpoint_label}"),
        source: network_source,
    }])?;

    let label = resolve_import_label(
        args.label,
        &args.file,
        args.non_interactive,
        &network_entry.genesis_hash,
        conn,
    )?;
    let json = fs::read_to_string(&args.file).with_context(|| {
        format!(
            "failed to read genesis account file {}",
            args.file.display()
        )
    })?;
    let original_filename = args
        .file
        .file_name()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned);
    let payload = accounts::parse_genesis_account_json(&json, original_filename)?;

    let vault_password =
        prompt_imported_vault_password(conn, &network_name, &network_entry.genesis_hash)?;
    let vault = accounts::create_or_unlock_imported_vault(
        conn,
        &network_entry.genesis_hash,
        &vault_password,
    )?;
    let source_metadata_json = serde_json::to_string(&payload.source)
        .context("failed to serialise imported account source metadata")?;
    accounts::import_imported_account(
        conn,
        &vault.dek,
        &vault.record,
        accounts::ImportedAccount {
            network_genesis_hash: &network_entry.genesis_hash,
            label: &label,
            import_kind: "genesis",
            source_metadata_json: Some(&source_metadata_json),
            payload: &payload,
        },
    )?;
    println!(
        "Imported account '{}' on network '{}'.",
        label, network_name
    );
    Ok(())
}

async fn rename_account(conn: &mut Connection, args: AccountRenameArgs) -> Result<()> {
    let seeds_by_id = seed_labels_by_id(conn)?;
    let networks_by_hash = network_names_by_genesis_hash()?;

    let seed_scope = if args.show_addresses {
        Some(resolve_seed_scope_for_addresses(
            conn,
            args.seed.as_deref(),
            args.non_interactive,
        )?)
    } else {
        None
    };

    let record = match args.old_label.as_deref() {
        Some(old_label) => {
            let matches = accounts::list(conn)?
                .into_iter()
                .filter(|record| record.label == old_label)
                .filter(|record| {
                    seed_scope_matches_record(record, seed_scope.as_ref(), &seeds_by_id)
                })
                .collect::<Vec<_>>();
            choose_account_match(
                matches,
                &seeds_by_id,
                &networks_by_hash,
                args.show_addresses,
                seed_scope.as_ref(),
                conn,
                args.non_interactive,
            )?
        }
        None if args.non_interactive => {
            bail!("account label must be provided in --non-interactive mode")
        }
        None => {
            let candidates = accounts::list(conn)?
                .into_iter()
                .filter(|record| {
                    seed_scope_matches_record(record, seed_scope.as_ref(), &seeds_by_id)
                })
                .collect::<Vec<_>>();
            select_account_fuzzy(
                conn,
                candidates,
                &seeds_by_id,
                &networks_by_hash,
                AccountSelectConfig {
                    show_addresses: args.show_addresses,
                    seed_scope: seed_scope.as_ref(),
                    always_prompt: false,
                    show_network: true,
                },
            )?
        }
    };

    let new_label = match args.new_label {
        Some(label) => label,
        None if args.non_interactive => {
            bail!("new account label must be provided in --non-interactive mode")
        }
        None => input("New account label:")
            .placeholder(&record.label)
            .validate(|value: &String| {
                if value.is_empty() {
                    Err("Account label is required.")
                } else {
                    Ok(())
                }
            })
            .interact()?,
    };
    validate_label("account", &new_label)?;
    accounts::rename(conn, record.id, &new_label)?;
    println!("Account '{}' renamed to '{}'.", record.label, new_label);
    Ok(())
}

async fn new(conn: &mut Connection, args: AccountNewArgs) -> Result<()> {
    let (seed_label, seed_source) = resolve_seed_label(
        conn,
        args.seed.as_deref(),
        args.non_interactive,
        args.no_defaults,
    )?;
    let (network_name, network_entry, endpoint, endpoint_label, network_source) =
        resolve_account_network_context(
            conn,
            args.network.as_deref(),
            args.node.clone(),
            args.non_interactive,
            args.no_defaults,
        )
        .await?;

    log_resolved_context(&[
        ContextLine {
            label: "seed:",
            value: seed_label.clone(),
            source: seed_source,
        },
        ContextLine {
            label: "network:",
            value: format!("{network_name} @ {endpoint_label}"),
            source: network_source,
        },
    ])?;

    let seed = seeds::find_by_label(conn, &seed_label)?
        .with_context(|| format!("seed '{}' is not configured", seed_label))?;
    let (mut identity, identity_source) = resolve_identity(
        conn,
        &network_entry.genesis_hash,
        &seed.id,
        args.identity.as_deref(),
        args.non_interactive,
    )?;

    let label = resolve_account_label(args.label, args.non_interactive)?;
    validate_label("account", &label)?;
    if accounts::find_by_network_and_label(conn, &network_entry.genesis_hash, &label)?.is_some() {
        bail!(
            "account label '{}' already exists on network '{}'",
            label,
            network_name
        );
    }

    log_resolved_context(&[ContextLine {
        label: "identity:",
        value: identity.label.clone(),
        source: identity_source,
    }])?;

    let password: String = password(format!("Password for seed '{}': ", seed.label)).interact()?;
    let unlocked_seed = seeds::unlock_context(conn, &seed.label, &password)?;

    if identity.status == IdentityStatus::Pending {
        let spin = spinner();
        spin.start("Checking pending identity status...");
        identity = confirm_identity_if_pending(conn, identity, &unlocked_seed.dek).await?;
        spin.clear();
    }
    ensure_identity_selectable(&identity, now_unix_seconds()?)?;

    let seed_phrase = std::str::from_utf8(&unlocked_seed.secret)
        .context("stored seed phrase is not UTF-8")?
        .to_owned();
    let net = infer_net(&network_name, &network_entry.node_endpoint, &endpoint_label);
    let wallet = ConcordiumHdWallet::from_seed_phrase(&seed_phrase, net)?;

    let spin = spinner();
    spin.start(format!("Connecting to node: {endpoint_label}"));
    let mut client = ccd_wallet_core::config::connect_v2_client(endpoint.clone())
        .await
        .with_context(|| format!("failed to connect to Concordium node at {endpoint_label}"))?;
    spin.clear();

    let spin = spinner();
    spin.start("Fetching chain cryptographic parameters...");
    let global_context = client
        .get_cryptographic_parameters(v2::BlockIdentifier::LastFinal)
        .await
        .with_context(|| format!("failed to load cryptographic parameters from {endpoint_label}"))?
        .response;
    spin.clear();

    let spin = spinner();
    spin.start("Fetching identity providers...");
    let ip_infos = fetch_identity_providers(&mut client).await?;
    spin.clear();
    let ip_info = ip_infos
        .iter()
        .find(|ip| ip.ip_identity.0 == identity.ip_identity)
        .with_context(|| {
            format!(
                "identity provider {} is not registered on the selected network",
                identity.ip_identity
            )
        })?;

    let spin = spinner();
    spin.start("Fetching anonymity revokers...");
    let ar_infos = fetch_anonymity_revokers(&mut client).await?;
    spin.clear();

    let identity_payload =
        identities::decrypt_private_payload(conn, identity.id, &unlocked_seed.dek)?;
    let identity_object = identity_payload
        .identity_object
        .as_ref()
        .context("identity has no stored identity object")?;
    let identity_object = parse_identity_object(identity_object)?;
    let credential_counter = accounts::next_credential_counter(
        conn,
        &network_entry.genesis_hash,
        &unlocked_seed.record.id,
        identity.ip_identity,
        identity.identity_index,
    )?;
    let credential_counter_u8 = credential_counter_to_u8(credential_counter)?;
    let account_id = accounts::insert_pending(
        conn,
        accounts::PendingAccount {
            network_genesis_hash: &network_entry.genesis_hash,
            seed_id: &unlocked_seed.record.id,
            ip_identity: identity.ip_identity,
            identity_index: identity.identity_index,
            credential_counter,
            label: &label,
        },
    )?;

    let spin = spinner();
    spin.start("Constructing account credential deployment...");
    let credential_deployment = build_credential_deployment(CredentialDeploymentInput {
        wallet: &wallet,
        ip_info,
        ar_infos: &ar_infos,
        global_context: &global_context,
        identity_object,
        identity_index: identity.identity_index,
        credential_counter: credential_counter_u8,
    })?;
    spin.clear();

    let block_item: BlockItem<Payload> = BlockItem::from(credential_deployment);
    let spin = spinner();
    spin.start("Submitting account credential deployment...");
    let transaction_hash = client
        .send_block_item(&block_item)
        .await
        .context("failed to submit account credential deployment")?;
    spin.clear();
    let transaction_hash_label = format!("{transaction_hash}");
    accounts::set_submitted_transaction(conn, account_id, &transaction_hash_label)?;

    if args.no_wait {
        cliclack::log::success(format!(
            "Account '{label}' submitted and pending finalization. Transaction hash: {transaction_hash_label}"
        ))?;
        return Ok(());
    }

    let spin = spinner();
    spin.start("Waiting for account creation finalization...");
    let (_block_hash, summary) = client
        .wait_until_finalized(&transaction_hash)
        .await
        .context("failed while waiting for account creation finalization")?;
    spin.clear();
    let details = summary.details.known_or_err()?;
    let account_address = match details {
        BlockItemSummaryDetails::AccountCreation(details) => format!("{}", details.address),
        other => bail!("expected account creation finalization, got {other:?}"),
    };
    accounts::set_finalized(
        conn,
        account_id,
        &unlocked_seed.dek,
        Some(&transaction_hash_label),
        &account_address,
    )?;
    cliclack::log::success(format!(
        "Account '{label}' created successfully. Address: {account_address}"
    ))?;
    Ok(())
}

fn seed_labels_by_id(conn: &Connection) -> Result<BTreeMap<String, String>> {
    Ok(seeds::list(conn)?
        .into_iter()
        .map(|seed| (seed.id, seed.label))
        .collect())
}

fn network_names_by_genesis_hash() -> Result<BTreeMap<String, String>> {
    Ok(load()?
        .networks
        .into_iter()
        .map(|(name, entry)| (entry.genesis_hash, name))
        .collect())
}

fn resolve_network_scope(
    conn: &Connection,
    explicit: Option<&str>,
    non_interactive: bool,
    no_defaults: bool,
    allow_all: bool,
) -> Result<(ScopeSelection, ResolutionSource)> {
    let app_config = load()?;
    match explicit {
        Some("all") if allow_all => Ok((ScopeSelection::All, ResolutionSource::Explicit)),
        Some(name) => app_config
            .networks
            .get(name)
            .map(|_| {
                (
                    ScopeSelection::One(name.to_owned()),
                    ResolutionSource::Explicit,
                )
            })
            .with_context(|| format!("network '{}' is not registered", name)),
        None => {
            let active = wallet_state::get(conn, wallet_state::ACTIVE_NETWORK_KEY)?;
            if no_defaults {
                return Ok((
                    prompt_for_network_scope(&app_config, active.as_deref(), allow_all)?,
                    ResolutionSource::Prompted,
                ));
            }
            match active {
                Some(name) => Ok((ScopeSelection::One(name), ResolutionSource::ActiveDefault)),
                None if non_interactive => bail!(
                    "no active network is set; provide `--network` or run `ccd-wallet network use <NAME>`"
                ),
                None => Ok((
                    prompt_for_network_scope(&app_config, None, allow_all)?,
                    ResolutionSource::Prompted,
                )),
            }
        }
    }
}

fn resolve_account_list_seed_scope(
    conn: &Connection,
    explicit: Option<&str>,
) -> Result<(ScopeSelection, ResolutionSource)> {
    match explicit {
        Some("all") => Ok((ScopeSelection::All, ResolutionSource::Explicit)),
        Some(label) => seeds::find_by_label(conn, label)?
            .map(|seed| (ScopeSelection::One(seed.label), ResolutionSource::Explicit))
            .with_context(|| format!("seed '{}' is not configured", label)),
        None => Ok((ScopeSelection::All, ResolutionSource::Inferred)),
    }
}

fn resolve_seed_scope_for_addresses(
    conn: &Connection,
    explicit: Option<&str>,
    _non_interactive: bool,
) -> Result<ScopeSelection> {
    match explicit {
        Some(label) => seeds::find_by_label(conn, label)?
            .map(|seed| ScopeSelection::One(seed.label))
            .with_context(|| format!("seed '{}' is not configured", label)),
        None => bail!("`--seed <LABEL>` is required with `--show-addresses`"),
    }
}

fn prompt_for_network_scope(
    app_config: &AppConfig,
    active: Option<&str>,
    allow_all: bool,
) -> Result<ScopeSelection> {
    if app_config.networks.is_empty() {
        bail!("no networks are configured; run `ccd-wallet network add` first")
    }
    let mut items = Vec::new();
    if allow_all {
        items.push(SelectItem {
            value: ScopeSelection::All,
            label: "All networks".to_owned(),
            hint: String::new(),
        });
    }
    items.extend(app_config.networks.iter().map(|(name, entry)| SelectItem {
        value: ScopeSelection::One(name.clone()),
        label: name.clone(),
        hint: entry.node_endpoint.clone(),
    }));
    let initial = active.map(|value| ScopeSelection::One(value.to_owned()));
    select_or_single("Select network", &items, initial.as_ref())
}

fn log_scope_context(
    seed_scope: &(ScopeSelection, ResolutionSource),
    network_scope: &(ScopeSelection, ResolutionSource),
) -> Result<()> {
    log_resolved_context(&[
        ContextLine {
            label: "seed:",
            value: match &seed_scope.0 {
                ScopeSelection::All => "all".to_owned(),
                ScopeSelection::One(value) => value.clone(),
            },
            source: seed_scope.1,
        },
        ContextLine {
            label: "network:",
            value: match &network_scope.0 {
                ScopeSelection::All => "all".to_owned(),
                ScopeSelection::One(value) => value.clone(),
            },
            source: network_scope.1,
        },
    ])
}

fn matches_seed_scope(
    record: &accounts::AccountRecord,
    scope: &(ScopeSelection, ResolutionSource),
    labels: &BTreeMap<String, String>,
) -> bool {
    if record.source_kind == accounts::AccountSourceKind::Imported {
        return !matches!(scope, (ScopeSelection::One(_), ResolutionSource::Explicit));
    }
    match &scope.0 {
        ScopeSelection::All => true,
        ScopeSelection::One(label) => labels.get(&record.seed_id) == Some(label),
    }
}

fn matches_network_scope(
    record: &accounts::AccountRecord,
    scope: &(ScopeSelection, ResolutionSource),
    names: &BTreeMap<String, String>,
) -> bool {
    match &scope.0 {
        ScopeSelection::All => true,
        ScopeSelection::One(name) => names.get(&record.network_genesis_hash) == Some(name),
    }
}

fn matches_account_status(
    record: &accounts::AccountRecord,
    status: Option<AccountListStatus>,
) -> bool {
    match status {
        None => true,
        Some(AccountListStatus::Pending) => record.status == accounts::AccountStatus::Pending,
        Some(AccountListStatus::Finalized) => record.status == accounts::AccountStatus::Finalized,
    }
}

fn seed_scope_matches_record(
    record: &accounts::AccountRecord,
    seed_scope: Option<&ScopeSelection>,
    labels: &BTreeMap<String, String>,
) -> bool {
    match seed_scope {
        None => true,
        Some(ScopeSelection::All) => true,
        Some(ScopeSelection::One(label)) => {
            record.source_kind == accounts::AccountSourceKind::Imported
                || labels.get(&record.seed_id) == Some(label)
        }
    }
}

fn load_account_addresses(
    conn: &Connection,
    records: &[accounts::AccountRecord],
    seeds_by_id: &BTreeMap<String, String>,
    seed_scope: &(ScopeSelection, ResolutionSource),
) -> Result<BTreeMap<i64, String>> {
    let mut by_seed: BTreeMap<String, Vec<&accounts::AccountRecord>> = BTreeMap::new();
    let mut by_imported_network: BTreeMap<String, Vec<&accounts::AccountRecord>> = BTreeMap::new();
    for record in records {
        if record.source_kind == accounts::AccountSourceKind::Imported {
            by_imported_network
                .entry(record.network_genesis_hash.clone())
                .or_default()
                .push(record);
        } else {
            by_seed
                .entry(record.seed_id.clone())
                .or_default()
                .push(record);
        }
    }

    let mut addresses = BTreeMap::new();
    for (network_genesis_hash, imported_records) in by_imported_network {
        let password: String = password(format!(
            "Imported accounts vault password for network '{}': ",
            network_genesis_hash
        ))
        .allow_empty()
        .interact()?;
        let unlocked = accounts::unlock_imported_vault(conn, &network_genesis_hash, &password)?;
        for record in imported_records {
            let payload = accounts::decrypt_imported_payload(conn, record.id, &unlocked.dek)?;
            addresses.insert(record.id, payload.account_address);
        }
    }

    for (seed_id, seed_records) in by_seed {
        let seed_label = seeds_by_id
            .get(&seed_id)
            .context("account references unknown seed")?;
        let password: String = password(format!("Password for seed '{}': ", seed_label))
            .allow_empty()
            .interact()?;
        let unlocked = seeds::unlock_context(conn, seed_label, &password)?;
        for record in seed_records {
            let payload = accounts::decrypt_private_payload(conn, record.id, &unlocked.dek)?;
            addresses.insert(record.id, payload.account_address);
        }
        if matches!(&seed_scope.0, ScopeSelection::One(_)) {
            break;
        }
    }
    Ok(addresses)
}

fn render_account_fuzzy_text(
    record: &accounts::AccountRecord,
    seed_label: &str,
    network_name: &str,
    address: Option<&str>,
    show_network: bool,
) -> String {
    let status_prefix = match record.status {
        accounts::AccountStatus::Pending => "[pending] ",
        accounts::AccountStatus::Finalized => "",
    };
    let owner_tag = if record.source_kind == accounts::AccountSourceKind::Imported {
        "[imported]".to_owned()
    } else {
        format!("[{seed_label}]")
    };
    let label = match address {
        Some(address) => format!(
            "{}{} {} ({address})",
            status_prefix, owner_tag, record.label
        ),
        None => format!("{}{} {}", status_prefix, owner_tag, record.label),
    };
    let network_suffix = if show_network {
        format!(" - {network_name}")
    } else {
        String::new()
    };
    if record.source_kind == accounts::AccountSourceKind::Imported {
        return format!("{label}{network_suffix}");
    }
    format!(
        "{}{} • provider:{} • identity:{} • cred:{}",
        label, network_suffix, record.ip_identity, record.identity_index, record.credential_counter
    )
}

#[derive(Clone, Copy, Debug)]
struct AccountSelectConfig<'a> {
    show_addresses: bool,
    seed_scope: Option<&'a ScopeSelection>,
    always_prompt: bool,
    show_network: bool,
}

fn select_account_fuzzy(
    conn: &Connection,
    candidates: Vec<accounts::AccountRecord>,
    seeds_by_id: &BTreeMap<String, String>,
    networks_by_hash: &BTreeMap<String, String>,
    config: AccountSelectConfig<'_>,
) -> Result<accounts::AccountRecord> {
    if candidates.is_empty() {
        bail!("no matching accounts are available")
    }
    let addresses = if config.show_addresses {
        let concrete_seed_scope = config
            .seed_scope
            .cloned()
            .context("a seed must be resolved to show addresses")?;
        load_account_addresses(
            conn,
            &candidates,
            seeds_by_id,
            &(concrete_seed_scope, ResolutionSource::Explicit),
        )?
    } else {
        BTreeMap::new()
    };
    let items = candidates
        .iter()
        .map(|record| {
            let seed_label = seeds_by_id
                .get(&record.seed_id)
                .cloned()
                .unwrap_or_else(|| "<unknown-seed>".to_owned());
            let network_name = networks_by_hash
                .get(&record.network_genesis_hash)
                .cloned()
                .unwrap_or_else(|| record.network_genesis_hash.clone());
            FuzzySelectItem {
                value: record.id,
                text: render_account_fuzzy_text(
                    record,
                    &seed_label,
                    &network_name,
                    addresses.get(&record.id).map(String::as_str),
                    config.show_network,
                ),
            }
        })
        .collect::<Vec<_>>();
    let id = if config.always_prompt {
        fuzzy_select_always("Select account", &items)?
    } else {
        fuzzy_select_or_single("Select account", &items)?
    };
    candidates
        .into_iter()
        .find(|record| record.id == id)
        .context("selected account was not found")
}

fn choose_account_match(
    matches: Vec<accounts::AccountRecord>,
    seeds_by_id: &BTreeMap<String, String>,
    networks_by_hash: &BTreeMap<String, String>,
    show_addresses: bool,
    seed_scope: Option<&ScopeSelection>,
    conn: &Connection,
    non_interactive: bool,
) -> Result<accounts::AccountRecord> {
    if matches.is_empty() {
        bail!("account is not configured")
    } else if matches.len() == 1 {
        Ok(matches.into_iter().next().unwrap())
    } else if non_interactive {
        bail!("account label is ambiguous across multiple networks; rerun interactively")
    } else {
        select_account_fuzzy(
            conn,
            matches,
            seeds_by_id,
            networks_by_hash,
            AccountSelectConfig {
                show_addresses,
                seed_scope,
                always_prompt: false,
                show_network: true,
            },
        )
    }
}

fn resolve_export_network(
    conn: &Connection,
    explicit: Option<&str>,
    non_interactive: bool,
    no_defaults: bool,
) -> Result<(String, NetworkEntry, ResolutionSource)> {
    let (scope, source) =
        resolve_network_scope(conn, explicit, non_interactive, no_defaults, false)?;
    let ScopeSelection::One(network_name) = scope else {
        bail!("account export requires a concrete network")
    };
    let app_config = load()?;
    let entry = app_config
        .networks
        .get(&network_name)
        .cloned()
        .with_context(|| format!("network '{}' is not registered", network_name))?;
    Ok((network_name, entry, source))
}

pub(crate) fn resolve_export_account(
    conn: &Connection,
    network_name: &str,
    network_genesis_hash: &str,
    explicit_label: Option<&str>,
    non_interactive: bool,
    always_prompt: bool,
) -> Result<accounts::AccountRecord> {
    let seeds_by_id = seed_labels_by_id(conn)?;
    let networks_by_hash = network_names_by_genesis_hash()?;
    let candidates = exportable_accounts_for_network(conn, network_genesis_hash)?;
    match explicit_label {
        Some(label) => {
            let matches = candidates
                .into_iter()
                .filter(|record| record.label == label)
                .collect::<Vec<_>>();
            if matches.is_empty() {
                bail!(
                    "finalized account '{}' is not configured on network '{}'",
                    label,
                    network_name
                );
            }
            choose_account_match(
                matches,
                &seeds_by_id,
                &networks_by_hash,
                false,
                None,
                conn,
                non_interactive,
            )
        }
        None if non_interactive => {
            bail!("account label must be provided in --non-interactive mode")
        }
        None => select_account_fuzzy(
            conn,
            candidates,
            &seeds_by_id,
            &networks_by_hash,
            AccountSelectConfig {
                show_addresses: false,
                seed_scope: None,
                always_prompt,
                show_network: false,
            },
        ),
    }
}

fn exportable_accounts_for_network(
    conn: &Connection,
    network_genesis_hash: &str,
) -> Result<Vec<accounts::AccountRecord>> {
    let mut records = accounts::list(conn)?
        .into_iter()
        .filter(|record| record.network_genesis_hash == network_genesis_hash)
        .filter(|record| record.status == accounts::AccountStatus::Finalized)
        .collect::<Vec<_>>();
    records.sort_by(|a, b| a.label.cmp(&b.label));
    if records.is_empty() {
        bail!(
            "no finalized accounts are available on network '{}'",
            network_genesis_hash
        );
    }
    Ok(records)
}

fn resolve_export_output_path(
    explicit: Option<PathBuf>,
    account_label: &str,
    non_interactive: bool,
) -> Result<PathBuf> {
    match explicit {
        Some(path) => expand_tilde_path(&path),
        None if non_interactive => {
            bail!("output path must be provided with `--out <FILE>` in --non-interactive mode")
        }
        None => {
            let suggested = format!("{account_label}.json");
            let path: String = input("Output file:")
                .default_input(&suggested)
                .validate(|value: &String| {
                    if value.is_empty() {
                        Err("Output file is required.")
                    } else {
                        Ok(())
                    }
                })
                .interact()?;
            expand_tilde_path(Path::new(&path))
        }
    }
}

fn expand_tilde_path(path: &Path) -> Result<PathBuf> {
    let text = path.to_string_lossy();
    if text == "~" || text.starts_with("~/") {
        let home = std::env::var("HOME")
            .context("could not determine home directory: $HOME is not set")?;
        if text == "~" {
            return Ok(PathBuf::from(home));
        }
        let suffix = text.trim_start_matches("~/");
        return Ok(PathBuf::from(home).join(suffix));
    }
    Ok(path.to_path_buf())
}

/// In-memory cache of account-secret ownership domains unlocked during one command.
#[derive(Debug, Default)]
pub(crate) struct AccountReferenceUnlocks {
    seed_deks: BTreeMap<String, Zeroizing<[u8; KEY_LEN]>>,
    imported_vault_deks: BTreeMap<String, Zeroizing<[u8; KEY_LEN]>>,
}

impl AccountReferenceUnlocks {
    /// Create an empty per-command account-reference unlock cache.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut unlocks = AccountReferenceUnlocks::new();
    /// ```
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AccountReferenceContext<'a> {
    pub(crate) network_name: &'a str,
    pub(crate) network_genesis_hash: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AccountReferenceSuggestion {
    pub(crate) label: String,
    pub(crate) text: String,
}

/// Render autocomplete suggestions for finalized local accounts on a network.
///
/// # Arguments
/// * `conn` - Open wallet store connection.
/// * `network_genesis_hash` - Network partition whose finalized accounts should be suggested.
///
/// # Errors
/// Returns an error if account or seed metadata cannot be read.
///
/// # Examples
///
/// ```ignore
/// let suggestions = account_reference_suggestions(&conn, genesis_hash)?;
/// ```
pub(crate) fn account_reference_suggestions(
    conn: &Connection,
    network_genesis_hash: &str,
) -> Result<Vec<AccountReferenceSuggestion>> {
    let seeds_by_id = seed_labels_by_id(conn)?;
    let mut suggestions = accounts::list(conn)?
        .into_iter()
        .filter(|record| record.network_genesis_hash == network_genesis_hash)
        .filter(|record| record.status == accounts::AccountStatus::Finalized)
        .map(|record| {
            let owner = match record.source_kind {
                accounts::AccountSourceKind::Derived => seeds_by_id
                    .get(&record.seed_id)
                    .map(|seed_label| format!("[{seed_label}]"))
                    .unwrap_or_else(|| "[<unknown-seed>]".to_owned()),
                accounts::AccountSourceKind::Imported => "[imported]".to_owned(),
            };
            AccountReferenceSuggestion {
                text: format!("{owner} {}", record.label),
                label: record.label,
            }
        })
        .collect::<Vec<_>>();
    suggestions.sort_by(|a, b| a.text.cmp(&b.text));
    Ok(suggestions)
}

/// Resolve a non-sender account reference from explicit input or an interactive prompt.
///
/// # Arguments
/// * `conn` - Open wallet store connection.
/// * `context` - Network context used for local label lookup and prompts.
/// * `explicit` - Optional CLI-supplied account reference.
/// * `prompt` - Prompt text used when interactive fallback is required.
/// * `label` - Human-readable field label used in errors.
/// * `non_interactive` - Whether prompt fallback is disabled.
/// * `unlocks` - Per-command cache of unlocked account-secret domains.
///
/// # Errors
/// Returns an error if input is missing in non-interactive mode, if the value is neither a raw
/// address nor a finalized local account label, or if local account payload decryption fails.
///
/// # Examples
///
/// ```ignore
/// let address = resolve_account_reference(
///     &conn,
///     AccountReferenceContext { network_name: "testnet", network_genesis_hash: genesis_hash },
///     Some("alice"),
///     "Recipient account:",
///     "recipient",
///     false,
///     &mut unlocks,
/// )?;
/// ```
pub(crate) fn resolve_account_reference(
    conn: &Connection,
    context: AccountReferenceContext<'_>,
    explicit: Option<&str>,
    prompt: &str,
    label: &str,
    non_interactive: bool,
    unlocks: &mut AccountReferenceUnlocks,
) -> Result<AccountAddress> {
    match explicit {
        Some(value) => resolve_account_reference_value(
            conn,
            context.network_name,
            context.network_genesis_hash,
            value,
            label,
            unlocks,
            &[],
        ),
        None if non_interactive => {
            bail!(
                "{label} account address or local account label must be provided in --non-interactive mode"
            )
        }
        None => {
            let suggestions = account_reference_suggestions(conn, context.network_genesis_hash)?;
            let value: String = input(prompt)
                .autocomplete(
                    suggestions
                        .iter()
                        .map(|suggestion| suggestion.text.clone())
                        .collect::<Vec<_>>(),
                )
                .interact()?;
            resolve_account_reference_value(
                conn,
                context.network_name,
                context.network_genesis_hash,
                &value,
                label,
                unlocks,
                &suggestions,
            )
        }
    }
}

/// Resolve repeated explicit account references.
///
/// # Arguments
/// * `conn` - Open wallet store connection.
/// * `context` - Network context used for local label lookup and prompts.
/// * `values` - Explicit account references to resolve.
/// * `label` - Human-readable field label used in errors.
/// * `unlocks` - Per-command cache of unlocked account-secret domains.
///
/// # Errors
/// Returns an error if any value is neither a raw address nor a finalized local account label.
///
/// # Examples
///
/// ```ignore
/// let targets = resolve_account_references(&conn, AccountReferenceContext { network_name: "testnet", network_genesis_hash: genesis_hash }, &args.targets, "target", &mut unlocks)?;
/// ```
pub(crate) fn resolve_account_references(
    conn: &Connection,
    context: AccountReferenceContext<'_>,
    values: &[String],
    label: &str,
    unlocks: &mut AccountReferenceUnlocks,
) -> Result<Vec<AccountAddress>> {
    values
        .iter()
        .map(|value| {
            resolve_account_reference_value(
                conn,
                context.network_name,
                context.network_genesis_hash,
                value,
                label,
                unlocks,
                &[],
            )
        })
        .collect()
}

fn resolve_account_reference_value(
    conn: &Connection,
    network_name: &str,
    network_genesis_hash: &str,
    value: &str,
    label: &str,
    unlocks: &mut AccountReferenceUnlocks,
    suggestions: &[AccountReferenceSuggestion],
) -> Result<AccountAddress> {
    if let Some(suggestion) = suggestions
        .iter()
        .find(|suggestion| suggestion.text == value)
    {
        return resolve_local_account_reference(
            conn,
            network_name,
            network_genesis_hash,
            &suggestion.label,
            label,
            unlocks,
        );
    }

    if let Ok(address) = AccountAddress::from_str(value) {
        return Ok(address);
    }

    resolve_local_account_reference(
        conn,
        network_name,
        network_genesis_hash,
        value,
        label,
        unlocks,
    )
}

fn resolve_local_account_reference(
    conn: &Connection,
    network_name: &str,
    network_genesis_hash: &str,
    account_label: &str,
    label: &str,
    unlocks: &mut AccountReferenceUnlocks,
) -> Result<AccountAddress> {
    let record = accounts::find_by_network_and_label(conn, network_genesis_hash, account_label)?
        .with_context(|| {
            format!(
                "{label} account reference '{account_label}' is not a valid address or finalized local account label"
            )
        })?;
    if record.status != accounts::AccountStatus::Finalized {
        bail!("{label} account reference '{account_label}' is not finalized")
    }
    decrypt_local_account_reference_address(conn, network_name, &record, unlocks)
}

fn decrypt_local_account_reference_address(
    conn: &Connection,
    network_name: &str,
    record: &accounts::AccountRecord,
    unlocks: &mut AccountReferenceUnlocks,
) -> Result<AccountAddress> {
    let address = match record.source_kind {
        accounts::AccountSourceKind::Derived => {
            let seed = seeds::list(conn)?
                .into_iter()
                .find(|seed| seed.id == record.seed_id)
                .context("selected account references unknown seed")?;
            if !unlocks.seed_deks.contains_key(&seed.id) {
                let password: String = password(format!("Password for seed '{}':", seed.label))
                    .allow_empty()
                    .interact()?;
                let unlocked = seeds::unlock_context(conn, &seed.label, &password)?;
                unlocks.seed_deks.insert(seed.id.clone(), unlocked.dek);
            }
            let dek = unlocks
                .seed_deks
                .get(&seed.id)
                .context("unlocked seed was not cached")?;
            accounts::decrypt_private_payload(conn, record.id, dek)?.account_address
        }
        accounts::AccountSourceKind::Imported => {
            let key = record
                .imported_vault_id
                .clone()
                .unwrap_or_else(|| record.network_genesis_hash.clone());
            if !unlocks.imported_vault_deks.contains_key(&key) {
                let vault_password: String = password(format!(
                    "Imported accounts vault password for '{}':",
                    network_name
                ))
                .allow_empty()
                .interact()?;
                let unlocked = accounts::unlock_imported_vault(
                    conn,
                    &record.network_genesis_hash,
                    &vault_password,
                )?;
                unlocks
                    .imported_vault_deks
                    .insert(key.clone(), unlocked.dek);
            }
            let dek = unlocks
                .imported_vault_deks
                .get(&key)
                .context("unlocked imported account vault was not cached")?;
            accounts::decrypt_imported_payload(conn, record.id, dek)?.account_address
        }
    };
    AccountAddress::from_str(&address).context("stored account address is invalid")
}

pub(crate) fn build_export_wallet_account(
    conn: &Connection,
    network_name: &str,
    network_entry: &NetworkEntry,
    account: &accounts::AccountRecord,
) -> Result<WalletAccount> {
    build_export_wallet_account_with_unlocks(
        conn,
        network_name,
        network_entry,
        account,
        &mut AccountReferenceUnlocks::new(),
    )
}

/// Build a signer wallet while recording unlocked ownership domains for this command.
///
/// # Arguments
/// * `conn` - Open wallet store connection.
/// * `network_name` - Human-readable network name for prompts and network inference.
/// * `network_entry` - Resolved network configuration.
/// * `account` - Finalized local account record to sign with.
/// * `unlocks` - Per-command cache updated with the signer's ownership domain.
///
/// # Errors
/// Returns an error if the account cannot be unlocked or converted into a signer wallet.
///
/// # Examples
///
/// ```ignore
/// let wallet = build_export_wallet_account_with_unlocks(&conn, "testnet", &entry, &account, &mut unlocks)?;
/// ```
pub(crate) fn build_export_wallet_account_with_unlocks(
    conn: &Connection,
    network_name: &str,
    network_entry: &NetworkEntry,
    account: &accounts::AccountRecord,
    unlocks: &mut AccountReferenceUnlocks,
) -> Result<WalletAccount> {
    match account.source_kind {
        accounts::AccountSourceKind::Derived => {
            build_derived_export_wallet_account(conn, network_name, network_entry, account, unlocks)
        }
        accounts::AccountSourceKind::Imported => {
            build_imported_export_wallet_account(conn, network_name, account, unlocks)
        }
    }
}

fn build_derived_export_wallet_account(
    conn: &Connection,
    network_name: &str,
    network_entry: &NetworkEntry,
    account: &accounts::AccountRecord,
    unlocks: &mut AccountReferenceUnlocks,
) -> Result<WalletAccount> {
    let seed = seeds::list(conn)?
        .into_iter()
        .find(|seed| seed.id == account.seed_id)
        .context("selected account references unknown seed")?;
    let password: String = password(format!("Password for seed '{}':", seed.label))
        .allow_empty()
        .interact()?;
    let unlocked = seeds::unlock_context(conn, &seed.label, &password)?;
    let payload = accounts::decrypt_private_payload(conn, account.id, &unlocked.dek)?;
    let seed_phrase = std::str::from_utf8(&unlocked.secret)
        .context("seed phrase is not valid UTF-8")?
        .to_owned();
    unlocks.seed_deks.insert(seed.id.clone(), unlocked.dek);
    let net = infer_net(
        network_name,
        &network_entry.node_endpoint,
        &network_entry.node_endpoint,
    );
    wallet_account_from_derived_parts(payload.account_address, &seed_phrase, account, net)
}

fn wallet_account_from_derived_parts(
    account_address: String,
    seed_phrase: &str,
    account: &accounts::AccountRecord,
    net: Net,
) -> Result<WalletAccount> {
    let wallet = ConcordiumHdWallet::from_seed_phrase(seed_phrase, net)?;
    let signing_key = wallet.get_account_signing_key(
        account.ip_identity,
        account.identity_index,
        account.credential_counter,
    )?;
    let mut keys = BTreeMap::new();
    keys.insert(
        KeyIndex(0),
        KeyPair::from(ed25519_dalek::SigningKey::from_bytes(&signing_key)),
    );
    Ok(WalletAccount {
        address: AccountAddress::from_str(&account_address)?,
        keys: AccountKeys::from(CredentialData {
            keys,
            threshold: SignatureThreshold::ONE,
        }),
    })
}

fn build_imported_export_wallet_account(
    conn: &Connection,
    network_name: &str,
    account: &accounts::AccountRecord,
    unlocks: &mut AccountReferenceUnlocks,
) -> Result<WalletAccount> {
    let vault_password: String = password(format!(
        "Imported accounts vault password for '{}':",
        network_name
    ))
    .allow_empty()
    .interact()?;
    let unlocked =
        accounts::unlock_imported_vault(conn, &account.network_genesis_hash, &vault_password)?;
    let payload = accounts::decrypt_imported_payload(conn, account.id, &unlocked.dek)?;
    let key = account
        .imported_vault_id
        .clone()
        .unwrap_or_else(|| account.network_genesis_hash.clone());
    unlocks.imported_vault_deks.insert(key, unlocked.dek);
    wallet_account_from_imported_payload(&payload)
}

fn wallet_account_from_imported_payload(
    payload: &accounts::ImportedAccountSecretPayload,
) -> Result<WalletAccount> {
    WalletAccount::from_json_value(serde_json::json!({
        "address": payload.account_address,
        "accountKeys": payload.account_keys,
    }))
    .context("failed to build signer for imported account")
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct MinimalWalletAccountJson<'a> {
    address: AccountAddress,
    account_keys: &'a AccountKeys,
}

fn serialize_wallet_account_minimal(account: &WalletAccount) -> Result<String> {
    serde_json::to_string_pretty(&MinimalWalletAccountJson {
        address: account.address,
        account_keys: &account.keys,
    })
    .context("failed to serialise wallet account export")
}

fn write_export_file(path: &Path, content: &str) -> Result<()> {
    if path.exists() {
        bail!("refusing to overwrite existing file {}", path.display());
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if !parent.exists() {
        bail!("output directory does not exist: {}", parent.display());
    }
    let temp_path = export_temp_path(path)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .with_context(|| {
            format!(
                "failed to create temporary export file {}",
                temp_path.display()
            )
        })?;
    file.write_all(content.as_bytes())
        .with_context(|| format!("failed to write export file {}", temp_path.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to flush export file {}", temp_path.display()))?;
    fs::rename(&temp_path, path).with_context(|| {
        format!(
            "failed to move temporary export file {} to {}",
            temp_path.display(),
            path.display()
        )
    })?;
    Ok(())
}

fn export_temp_path(path: &Path) -> Result<PathBuf> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .context("output file name must be valid UTF-8")?;
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?
        .as_nanos();
    Ok(parent.join(format!(".{file_name}.tmp-{}-{unique}", std::process::id())))
}

fn prompt_imported_vault_password(
    conn: &Connection,
    network_name: &str,
    network_genesis_hash: &str,
) -> Result<String> {
    let exists = accounts::imported_vault_exists(conn, network_genesis_hash)?;
    if !exists {
        cliclack::log::info(format!(
            "Setting up imported accounts vault for '{}'.",
            network_name
        ))?;
    }
    let prompt = if exists {
        format!("Vault password for '{}':", network_name)
    } else {
        format!("Set vault password for '{}':", network_name)
    };
    let vault_password = password(prompt).allow_empty().interact()?;
    if !exists {
        let confirmation = password(format!("Confirm vault password for '{}':", network_name))
            .allow_empty()
            .interact()?;
        if vault_password != confirmation {
            bail!("imported accounts vault password confirmation did not match");
        }
    }
    Ok(vault_password)
}

fn validate_genesis_import_file(file: &Path) -> Result<()> {
    if file.is_dir() {
        bail!("genesis account import expects a single JSON file, not a directory")
    }
    Ok(())
}

fn resolve_import_label(
    explicit: Option<String>,
    file: &Path,
    non_interactive: bool,
    network_genesis_hash: &str,
    conn: &Connection,
) -> Result<String> {
    let suggested = file
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("imported-account")
        .to_owned();
    let label = match explicit {
        Some(label) => label,
        None if non_interactive => {
            bail!("account label must be provided in --non-interactive mode")
        }
        None => input("Imported account label:")
            .default_input(&suggested)
            .validate(|value: &String| {
                if value.is_empty() {
                    Err("Account label is required.")
                } else {
                    Ok(())
                }
            })
            .interact()?,
    };
    validate_label("account", &label)?;
    if accounts::find_by_network_and_label(conn, network_genesis_hash, &label)?.is_some() {
        bail!(
            "account label '{}' already exists for network '{}'",
            label,
            network_genesis_hash
        );
    }
    Ok(label)
}

fn resolve_account_label(explicit: Option<String>, non_interactive: bool) -> Result<String> {
    match explicit {
        Some(label) => Ok(label),
        None if non_interactive => {
            bail!("account label must be provided in --non-interactive mode")
        }
        None => Ok(input("Account label:")
            .validate(|value: &String| {
                if value.is_empty() {
                    Err("Account label is required.")
                } else {
                    Ok(())
                }
            })
            .interact()?),
    }
}

fn resolve_seed_label(
    conn: &Connection,
    explicit: Option<&str>,
    non_interactive: bool,
    no_defaults: bool,
) -> Result<(String, ResolutionSource)> {
    match explicit {
        Some(label) => seeds::find_by_label(conn, label)?
            .map(|s| (s.label, ResolutionSource::Explicit))
            .with_context(|| format!("seed '{}' is not configured", label)),
        None => {
            let active = wallet_state::get(conn, wallet_state::ACTIVE_SEED_KEY)?;
            if no_defaults {
                return Ok((
                    prompt_for_seed_label(conn, active.as_deref())?,
                    ResolutionSource::Prompted,
                ));
            }
            match active {
                Some(label) => Ok((label, ResolutionSource::ActiveDefault)),
                None if non_interactive => bail!(
                    "No active seed. Run `ccd-wallet seed use <LABEL>` or supply `--seed <LABEL>`."
                ),
                None => Ok((
                    prompt_for_seed_label(conn, None)?,
                    ResolutionSource::Prompted,
                )),
            }
        }
    }
}

fn prompt_for_seed_label(conn: &Connection, active: Option<&str>) -> Result<String> {
    let seeds = seeds::list(conn)?;
    if seeds.is_empty() {
        bail!("no seeds are configured; run `ccd-wallet seed add <LABEL>` first")
    }

    let items = seeds
        .iter()
        .map(|seed| SelectItem {
            value: seed.label.clone(),
            label: seed.label.clone(),
            hint: String::new(),
        })
        .collect::<Vec<_>>();
    let initial = active.map(str::to_owned);
    select_or_single("Select seed", &items, initial.as_ref())
}

fn resolve_identity(
    conn: &Connection,
    network_genesis_hash: &str,
    seed_id: &str,
    explicit: Option<&str>,
    non_interactive: bool,
) -> Result<(IdentityRecord, ResolutionSource)> {
    let now = now_unix_seconds()?;
    match explicit {
        Some(label) => {
            let identity = identities::find_by_network_seed_and_label(
                conn,
                network_genesis_hash,
                seed_id,
                label,
            )?
            .with_context(|| {
                format!(
                    "identity '{}' is not configured for the selected seed and network",
                    label
                )
            })?;
            ensure_identity_selectable(&identity, now)?;
            Ok((identity, ResolutionSource::Explicit))
        }
        None if non_interactive => bail!(
            "identity label must be provided in --non-interactive mode with `--identity <LABEL>`"
        ),
        None => Ok((
            prompt_for_identity(conn, network_genesis_hash, seed_id, now)?,
            ResolutionSource::Prompted,
        )),
    }
}

fn prompt_for_identity(
    conn: &Connection,
    network_genesis_hash: &str,
    seed_id: &str,
    now: i64,
) -> Result<IdentityRecord> {
    let identities = identities::list_by_network_and_seed(conn, network_genesis_hash, seed_id)?;
    let expired_count = identities
        .iter()
        .filter(|identity| {
            identity
                .expires_at
                .is_some_and(|expires_at| expires_at <= now)
        })
        .count();
    let usable = identities
        .into_iter()
        .filter(|identity| is_identity_selectable(identity, now))
        .collect::<Vec<_>>();
    if usable.is_empty() {
        bail!(
            "no usable identities are available for account creation on the selected seed and network"
        )
    }

    let items = usable
        .iter()
        .map(|identity| SelectItem {
            value: identity.id,
            label: identity.label.clone(),
            hint: match identity.status {
                IdentityStatus::Done => format!(
                    "provider {}, identity index {}",
                    identity.ip_identity, identity.identity_index
                ),
                IdentityStatus::Pending => format!(
                    "pending; provider {}, identity index {}",
                    identity.ip_identity, identity.identity_index
                ),
            },
        })
        .collect::<Vec<_>>();
    let prompt = if expired_count == 0 {
        "Select identity".to_owned()
    } else if expired_count == 1 {
        "Select identity (1 expired identity hidden)".to_owned()
    } else {
        format!("Select identity ({expired_count} expired identities hidden)")
    };
    let id = select_identity_always(&prompt, &items)?;
    usable
        .into_iter()
        .find(|identity| identity.id == id)
        .context("selected identity was not found")
}

fn select_identity_always(prompt: &str, items: &[SelectItem<i64>]) -> Result<i64> {
    let mut picker = select(prompt);
    for item in items {
        picker = picker.item(item.value, item.label.clone(), item.hint.clone());
    }
    Ok(picker.interact()?)
}

async fn confirm_identity_if_pending(
    conn: &mut Connection,
    identity: IdentityRecord,
    seed_dek: &[u8; KEY_LEN],
) -> Result<IdentityRecord> {
    if identity.status != IdentityStatus::Pending {
        return Ok(identity);
    }

    let payload = identities::decrypt_private_payload(conn, identity.id, seed_dek)?;
    match client::poll_code_uri(&payload.code_uri).await? {
        PollResult::Pending => bail!(
            "identity '{}' is still pending and cannot be used for account creation yet",
            identity.label
        ),
        PollResult::ProviderError(detail) => {
            identities::delete(conn, identity.id)?;
            bail!(detail)
        }
        PollResult::Done(token) => {
            identities::set_done(conn, identity.id, seed_dek, token)?;
            identities::find_by_network_seed_and_label(
                conn,
                &identity.network_genesis_hash,
                &identity.seed_id,
                &identity.label,
            )?
            .with_context(|| {
                format!(
                    "identity '{}' was not found after confirmation",
                    identity.label
                )
            })
        }
    }
}

fn ensure_identity_selectable(identity: &IdentityRecord, now: i64) -> Result<()> {
    if !is_identity_selectable(identity, now) {
        if identity
            .expires_at
            .is_some_and(|expires_at| expires_at <= now)
        {
            bail!("identity '{}' is expired", identity.label);
        }
        if identity.status == IdentityStatus::Done && identity.expires_at.is_none() {
            bail!("identity '{}' has no expiry metadata", identity.label);
        }
        bail!(
            "identity '{}' is not usable for account creation",
            identity.label
        );
    }
    Ok(())
}

fn is_identity_selectable(identity: &IdentityRecord, now: i64) -> bool {
    match identity.status {
        IdentityStatus::Done => identity
            .expires_at
            .is_some_and(|expires_at| expires_at > now),
        IdentityStatus::Pending => true,
    }
}

async fn resolve_import_network_context(
    conn: &Connection,
    network: Option<&str>,
    non_interactive: bool,
) -> Result<(String, NetworkEntry, v2::Endpoint, String, ResolutionSource)> {
    let app_config = load()?;
    let selected_network = match network {
        Some(name) => (name.to_owned(), ResolutionSource::Explicit),
        None if non_interactive => {
            bail!("network must be provided with `--network <NAME>` in --non-interactive mode")
        }
        None => {
            let active = wallet_state::get(conn, wallet_state::ACTIVE_NETWORK_KEY)?;
            (
                prompt_for_network_name(&app_config, active.as_deref())?,
                ResolutionSource::Prompted,
            )
        }
    };

    let entry = app_config
        .networks
        .get(&selected_network.0)
        .cloned()
        .with_context(|| format!("network '{}' is not registered", selected_network.0))?;
    let endpoint: v2::Endpoint =
        ccd_wallet_core::config::normalize_url_string(&entry.node_endpoint)
            .parse()
            .with_context(|| {
                format!(
                    "network '{}' has an invalid stored endpoint: {}",
                    selected_network.0, entry.node_endpoint
                )
            })?;
    let endpoint_label = ccd_wallet_core::config::endpoint_label(&endpoint);
    let node_genesis_hash = fetch_node_genesis_hash(endpoint.clone(), &endpoint_label).await?;
    if node_genesis_hash != entry.genesis_hash {
        bail!(
            "configured node for network '{}' points to genesis hash {}, which does not match the stored network genesis hash {}",
            selected_network.0,
            node_genesis_hash,
            entry.genesis_hash
        );
    }
    Ok((
        selected_network.0,
        entry,
        endpoint,
        endpoint_label,
        selected_network.1,
    ))
}

pub(crate) async fn resolve_account_network_context(
    conn: &Connection,
    network: Option<&str>,
    node_override: Option<v2::Endpoint>,
    non_interactive: bool,
    no_defaults: bool,
) -> Result<(String, NetworkEntry, v2::Endpoint, String, ResolutionSource)> {
    let app_config = load()?;

    if let Some(endpoint) = node_override {
        let endpoint_label = ccd_wallet_core::config::endpoint_label(&endpoint);
        let node_genesis_hash = fetch_node_genesis_hash(endpoint.clone(), &endpoint_label).await?;

        if let Some(network_name) = network {
            let entry = app_config
                .networks
                .get(network_name)
                .cloned()
                .with_context(|| format!("network '{}' is not registered", network_name))?;
            if entry.genesis_hash != node_genesis_hash {
                bail!(
                    "node at {} belongs to genesis hash {}, which does not match configured network '{}' ({})",
                    endpoint_label,
                    node_genesis_hash,
                    network_name,
                    entry.genesis_hash
                );
            }
            return Ok((
                network_name.to_owned(),
                entry,
                endpoint,
                endpoint_label,
                ResolutionSource::Explicit,
            ));
        }

        let matches = app_config
            .networks
            .iter()
            .filter(|(_, entry)| entry.genesis_hash == node_genesis_hash)
            .map(|(name, entry)| (name.clone(), entry.clone()))
            .collect::<Vec<_>>();
        if matches.is_empty() {
            bail!(
                "no configured network matches the supplied node at {} (genesis hash: {})",
                endpoint_label,
                node_genesis_hash
            );
        }
        let active_network = wallet_state::get(conn, wallet_state::ACTIVE_NETWORK_KEY)?;
        let name = if no_defaults && matches.len() > 1 {
            prompt_for_matching_network_name(&matches, active_network.as_deref())?
        } else {
            matches[0].0.clone()
        };
        let entry = matches
            .into_iter()
            .find(|(candidate, _)| candidate == &name)
            .map(|(_, entry)| entry)
            .context("selected network was not found")?;
        let source = if no_defaults {
            ResolutionSource::Prompted
        } else {
            ResolutionSource::Inferred
        };
        return Ok((name, entry, endpoint, endpoint_label, source));
    }

    let (selected_network, source) = match network {
        Some(name) => (name.to_owned(), ResolutionSource::Explicit),
        None => {
            let active = wallet_state::get(conn, wallet_state::ACTIVE_NETWORK_KEY)?;
            if no_defaults {
                (
                    prompt_for_network_name(&app_config, active.as_deref())?,
                    ResolutionSource::Prompted,
                )
            } else {
                match active {
                    Some(name) => (name, ResolutionSource::ActiveDefault),
                    None if non_interactive => bail!(
                        "no active network is set; provide `--network` or run `ccd-wallet network use <NAME>`"
                    ),
                    None => (
                        prompt_for_network_name(&app_config, None)?,
                        ResolutionSource::Prompted,
                    ),
                }
            }
        }
    };

    let entry = app_config
        .networks
        .get(&selected_network)
        .cloned()
        .with_context(|| format!("network '{}' is not registered", selected_network))?;
    let endpoint: v2::Endpoint =
        ccd_wallet_core::config::normalize_url_string(&entry.node_endpoint)
            .parse()
            .with_context(|| {
                format!(
                    "network '{}' has an invalid stored endpoint: {}",
                    selected_network, entry.node_endpoint
                )
            })?;
    let endpoint_label = ccd_wallet_core::config::endpoint_label(&endpoint);
    let node_genesis_hash = fetch_node_genesis_hash(endpoint.clone(), &endpoint_label).await?;
    if node_genesis_hash != entry.genesis_hash {
        bail!(
            "configured node for network '{}' points to genesis hash {}, which does not match the stored network genesis hash {}",
            selected_network,
            node_genesis_hash,
            entry.genesis_hash
        );
    }
    Ok((selected_network, entry, endpoint, endpoint_label, source))
}

fn prompt_for_network_name(app_config: &AppConfig, active: Option<&str>) -> Result<String> {
    if app_config.networks.is_empty() {
        bail!("no networks are configured; run `ccd-wallet network add` first")
    }
    let items = app_config
        .networks
        .iter()
        .map(|(name, entry)| SelectItem {
            value: name.clone(),
            label: name.clone(),
            hint: entry.node_endpoint.to_string(),
        })
        .collect::<Vec<_>>();
    let initial = active.map(str::to_owned);
    select_or_single("Select network", &items, initial.as_ref())
}

async fn fetch_identity_providers(client: &mut v2::Client) -> Result<Vec<IpInfo<IpPairing>>> {
    let mut stream = client
        .get_identity_providers(v2::BlockIdentifier::LastFinal)
        .await
        .context("failed to fetch identity providers from the node")?
        .response;

    let mut providers = Vec::new();
    while let Some(item) = stream.next().await {
        providers.push(item.context("failed to read identity provider from stream")?);
    }
    if providers.is_empty() {
        bail!("no identity providers are available on the selected network");
    }
    Ok(providers)
}

async fn fetch_anonymity_revokers(
    client: &mut v2::Client,
) -> Result<std::collections::BTreeMap<ArIdentity, ArInfo<ArCurve>>> {
    let mut stream = client
        .get_anonymity_revokers(v2::BlockIdentifier::LastFinal)
        .await
        .context("failed to fetch anonymity revokers from the node")?
        .response;

    let mut revokers = std::collections::BTreeMap::new();
    while let Some(item) = stream.next().await {
        let ar_info = item.context("failed to read anonymity revoker from stream")?;
        revokers.insert(ar_info.ar_identity, ar_info);
    }
    if revokers.is_empty() {
        bail!("no anonymity revokers are available on the selected network");
    }
    Ok(revokers)
}

fn infer_net(network_name: &str, node_endpoint: &str, endpoint_label: &str) -> Net {
    let haystack = format!("{network_name} {node_endpoint} {endpoint_label}").to_ascii_lowercase();
    if haystack.contains("testnet") || haystack.contains("staging") || haystack.contains("test") {
        Net::Testnet
    } else {
        Net::Mainnet
    }
}

fn prompt_for_matching_network_name(
    matches: &[(String, NetworkEntry)],
    active: Option<&str>,
) -> Result<String> {
    let items = matches
        .iter()
        .map(|(name, entry)| SelectItem {
            value: name.clone(),
            label: name.clone(),
            hint: entry.node_endpoint.to_string(),
        })
        .collect::<Vec<_>>();
    let initial = active.map(str::to_owned);
    select_or_single("Select network", &items, initial.as_ref())
}

async fn fetch_node_genesis_hash(endpoint: v2::Endpoint, endpoint_label: &str) -> Result<String> {
    let mut client = ccd_wallet_core::config::connect_v2_client(endpoint)
        .await
        .with_context(|| format!("failed to connect to Concordium node at {endpoint_label}"))?;
    let consensus_info = client
        .get_consensus_info()
        .await
        .with_context(|| format!("failed to query consensus info from node at {endpoint_label}"))?;
    Ok(format!("{}", consensus_info.genesis_block))
}

fn validate_label(kind: &str, label: &str) -> Result<()> {
    if label.is_empty() {
        bail!("{kind} label must not be empty");
    }
    if !label
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        bail!("{kind} labels may contain only ASCII letters, digits, dash, and underscore");
    }
    Ok(())
}

fn now_unix_seconds() -> Result<i64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?;
    Ok(duration.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccd_wallet_core::store::migrations;

    const TEST_SEED_PHRASE: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    fn identity(status: IdentityStatus, expires_at: Option<i64>) -> IdentityRecord {
        IdentityRecord {
            id: 1,
            seed_id: "seed-id".to_owned(),
            network_genesis_hash: "genesis".to_owned(),
            ip_identity: 7,
            identity_index: 0,
            label: "identity".to_owned(),
            status,
            created_at: 0,
            expires_at,
        }
    }

    fn derived_record() -> accounts::AccountRecord {
        accounts::AccountRecord {
            id: 1,
            seed_id: "seed-id".to_owned(),
            network_genesis_hash: "genesis".to_owned(),
            ip_identity: 0,
            identity_index: 0,
            credential_counter: 0,
            source_kind: accounts::AccountSourceKind::Derived,
            imported_vault_id: None,
            import_kind: None,
            source_metadata_json: None,
            label: "account".to_owned(),
            status: accounts::AccountStatus::Finalized,
            transaction_hash: None,
            created_at: 0,
            updated_at: 0,
        }
    }

    fn imported_payload(address: &str) -> accounts::ImportedAccountSecretPayload {
        accounts::ImportedAccountSecretPayload {
            account_address: address.to_owned(),
            account_keys: serde_json::json!({
                "keys": {
                    "0": {
                        "keys": {
                            "0": {
                                "signKey": hex::encode([7u8; 32]),
                                "verifyKey": hex::encode(
                                    ed25519_dalek::SigningKey::from_bytes(&[7u8; 32])
                                        .verifying_key()
                                        .to_bytes()
                                )
                            }
                        },
                        "threshold": 1
                    }
                },
                "threshold": 1
            }),
            credentials: serde_json::json!({}),
            encryption_public_key: None,
            encryption_secret_key: None,
            credential_holder_info: None,
            source: accounts::ImportedAccountSourceMetadata {
                import_kind: "genesis".to_owned(),
                original_filename: Some("import.json".to_owned()),
            },
        }
    }

    fn finalized_derived_account(
        conn: &mut Connection,
        seed_id: &str,
        dek: &[u8; KEY_LEN],
        network_genesis_hash: &str,
        label: &str,
        credential_counter: u32,
        address: &str,
    ) -> i64 {
        let id = accounts::insert_pending(
            conn,
            accounts::PendingAccount {
                network_genesis_hash,
                seed_id,
                ip_identity: 0,
                identity_index: 0,
                credential_counter,
                label,
            },
        )
        .unwrap();
        accounts::set_finalized(conn, id, dek, None, address).unwrap();
        id
    }

    #[test]
    fn account_reference_prefers_raw_address_over_matching_label() {
        let mut conn = conn();
        let seed = seeds::add(&conn, "seed", TEST_SEED_PHRASE.as_bytes(), "password").unwrap();
        let unlocked = seeds::unlock_context(&conn, "seed", "password").unwrap();
        let address_label = "4QkqdUnrjShrUrHpE96odLM6J77nWzEryifzqNnwNk4FYNge8a";
        let stored_address = "4UC8o4m8AgTxt5VBFMdLwMCwwJQVJwjesNzW7RPXkACynrULmd";
        finalized_derived_account(
            &mut conn,
            &seed.id,
            &unlocked.dek,
            "genesis",
            address_label,
            0,
            stored_address,
        );

        let resolved = resolve_account_reference(
            &conn,
            AccountReferenceContext {
                network_name: "testnet",
                network_genesis_hash: "genesis",
            },
            Some(address_label),
            "Recipient account address or local label:",
            "recipient",
            true,
            &mut AccountReferenceUnlocks::new(),
        )
        .unwrap();

        assert_eq!(resolved, AccountAddress::from_str(address_label).unwrap());
    }

    #[test]
    fn account_reference_resolves_finalized_local_label() {
        let mut conn = conn();
        let seed = seeds::add(&conn, "seed", TEST_SEED_PHRASE.as_bytes(), "password").unwrap();
        let unlocked = seeds::unlock_context(&conn, "seed", "password").unwrap();
        let address = "4QkqdUnrjShrUrHpE96odLM6J77nWzEryifzqNnwNk4FYNge8a";
        finalized_derived_account(
            &mut conn,
            &seed.id,
            &unlocked.dek,
            "genesis",
            "alice",
            0,
            address,
        );
        let mut unlocks = AccountReferenceUnlocks::new();
        unlocks
            .seed_deks
            .insert(seed.id.clone(), unlocked.dek.clone());

        let resolved = resolve_account_reference(
            &conn,
            AccountReferenceContext {
                network_name: "testnet",
                network_genesis_hash: "genesis",
            },
            Some("alice"),
            "Recipient account address or local label:",
            "recipient",
            true,
            &mut unlocks,
        )
        .unwrap();

        assert_eq!(resolved, AccountAddress::from_str(address).unwrap());
    }

    #[test]
    fn account_reference_rejects_missing_and_pending_labels() {
        let conn = conn();
        let seed = seeds::add(&conn, "seed", TEST_SEED_PHRASE.as_bytes(), "password").unwrap();
        accounts::insert_pending(
            &conn,
            accounts::PendingAccount {
                network_genesis_hash: "genesis",
                seed_id: &seed.id,
                ip_identity: 0,
                identity_index: 0,
                credential_counter: 0,
                label: "pending",
            },
        )
        .unwrap();

        let missing = resolve_account_reference(
            &conn,
            AccountReferenceContext {
                network_name: "testnet",
                network_genesis_hash: "genesis",
            },
            Some("missing"),
            "Recipient account address or local label:",
            "recipient",
            true,
            &mut AccountReferenceUnlocks::new(),
        )
        .unwrap_err();
        assert!(missing.to_string().contains("not a valid address"));

        let pending = resolve_account_reference(
            &conn,
            AccountReferenceContext {
                network_name: "testnet",
                network_genesis_hash: "genesis",
            },
            Some("pending"),
            "Recipient account address or local label:",
            "recipient",
            true,
            &mut AccountReferenceUnlocks::new(),
        )
        .unwrap_err();
        assert!(pending.to_string().contains("not finalized"));
    }

    #[test]
    fn account_reference_reuses_cached_seed_dek_for_multiple_labels() {
        let mut conn = conn();
        let seed = seeds::add(&conn, "seed", TEST_SEED_PHRASE.as_bytes(), "password").unwrap();
        let unlocked = seeds::unlock_context(&conn, "seed", "password").unwrap();
        let alice = "4QkqdUnrjShrUrHpE96odLM6J77nWzEryifzqNnwNk4FYNge8a";
        let bob = "4UC8o4m8AgTxt5VBFMdLwMCwwJQVJwjesNzW7RPXkACynrULmd";
        finalized_derived_account(
            &mut conn,
            &seed.id,
            &unlocked.dek,
            "genesis",
            "alice",
            0,
            alice,
        );
        finalized_derived_account(&mut conn, &seed.id, &unlocked.dek, "genesis", "bob", 1, bob);
        let mut unlocks = AccountReferenceUnlocks::new();
        unlocks
            .seed_deks
            .insert(seed.id.clone(), unlocked.dek.clone());

        let resolved = resolve_account_references(
            &conn,
            AccountReferenceContext {
                network_name: "testnet",
                network_genesis_hash: "genesis",
            },
            &["alice".to_owned(), "bob".to_owned()],
            "recipient",
            &mut unlocks,
        )
        .unwrap();

        assert_eq!(
            resolved,
            vec![
                AccountAddress::from_str(alice).unwrap(),
                AccountAddress::from_str(bob).unwrap()
            ]
        );
    }

    #[test]
    fn account_reference_suggestions_show_seed_and_imported_owners() {
        let mut conn = conn();
        let seed = seeds::add(&conn, "main-seed", TEST_SEED_PHRASE.as_bytes(), "password").unwrap();
        let unlocked = seeds::unlock_context(&conn, "main-seed", "password").unwrap();
        finalized_derived_account(
            &mut conn,
            &seed.id,
            &unlocked.dek,
            "genesis",
            "alice",
            0,
            "4QkqdUnrjShrUrHpE96odLM6J77nWzEryifzqNnwNk4FYNge8a",
        );
        accounts::insert_pending(
            &conn,
            accounts::PendingAccount {
                network_genesis_hash: "genesis",
                seed_id: &seed.id,
                ip_identity: 0,
                identity_index: 0,
                credential_counter: 1,
                label: "pending",
            },
        )
        .unwrap();
        let vault =
            accounts::create_or_unlock_imported_vault(&conn, "genesis", "password").unwrap();
        accounts::import_imported_account(
            &mut conn,
            &vault.dek,
            &vault.record,
            accounts::ImportedAccount {
                network_genesis_hash: "genesis",
                label: "baker-0",
                import_kind: "genesis",
                source_metadata_json: None,
                payload: &imported_payload("4UC8o4m8AgTxt5VBFMdLwMCwwJQVJwjesNzW7RPXkACynrULmd"),
            },
        )
        .unwrap();

        let suggestions = account_reference_suggestions(&conn, "genesis").unwrap();
        let texts = suggestions
            .into_iter()
            .map(|suggestion| suggestion.text)
            .collect::<Vec<_>>();

        assert_eq!(texts, vec!["[imported] baker-0", "[main-seed] alice"]);
    }

    #[test]
    fn done_identity_requires_future_expiry_to_be_selectable() {
        assert!(is_identity_selectable(
            &identity(IdentityStatus::Done, Some(200)),
            100
        ));
        assert!(!is_identity_selectable(
            &identity(IdentityStatus::Done, Some(100)),
            100
        ));
        assert!(!is_identity_selectable(
            &identity(IdentityStatus::Done, None),
            100
        ));
    }

    #[test]
    fn pending_identity_is_selectable_for_lazy_confirmation() {
        assert!(is_identity_selectable(
            &identity(IdentityStatus::Pending, None),
            100
        ));
    }

    #[test]
    fn explicit_expired_identity_errors_actionably() {
        let err = ensure_identity_selectable(&identity(IdentityStatus::Done, Some(100)), 100)
            .unwrap_err();
        assert!(err.to_string().contains("expired"));
    }

    #[test]
    fn labels_use_seed_label_format() {
        validate_label("account", "main-account_1").unwrap();
        assert!(validate_label("account", "bad label").is_err());
        assert!(validate_label("account", "bad.label").is_err());
    }

    #[test]
    fn imported_vault_password_confirmation_is_required_only_on_first_import() {
        let conn = conn();
        assert!(!accounts::imported_vault_exists(&conn, "genesis").unwrap());
        accounts::create_or_unlock_imported_vault(&conn, "genesis", "").unwrap();
        assert!(accounts::imported_vault_exists(&conn, "genesis").unwrap());
        assert!(accounts::unlock_imported_vault(&conn, "genesis", "").is_ok());
    }

    #[test]
    fn import_label_validation_rejects_duplicates_and_missing_noninteractive() {
        let conn = conn();
        let seed = seeds::add(&conn, "seed", b"seed secret", "password").unwrap();
        accounts::insert_pending(
            &conn,
            accounts::PendingAccount {
                network_genesis_hash: "genesis",
                seed_id: &seed.id,
                ip_identity: 0,
                identity_index: 0,
                credential_counter: 0,
                label: "existing",
            },
        )
        .unwrap();

        let err = resolve_import_label(None, Path::new("baker-0.json"), true, "genesis", &conn)
            .unwrap_err();
        assert!(err.to_string().contains("account label must be provided"));

        let err = resolve_import_label(
            Some("existing".to_owned()),
            Path::new("baker-0.json"),
            true,
            "genesis",
            &conn,
        )
        .unwrap_err();
        assert!(err.to_string().contains("already exists"));

        assert_eq!(
            resolve_import_label(
                Some("baker-0".to_owned()),
                Path::new("baker-0.json"),
                true,
                "genesis",
                &conn,
            )
            .unwrap(),
            "baker-0"
        );
    }

    #[test]
    fn genesis_import_directory_path_is_rejected() {
        assert!(validate_genesis_import_file(Path::new(".")).is_err());
    }

    #[test]
    fn account_status_filter_matches_status() {
        let pending = accounts::AccountRecord {
            id: 1,
            seed_id: "seed-id".to_owned(),
            network_genesis_hash: "genesis".to_owned(),
            ip_identity: 0,
            identity_index: 0,
            credential_counter: 0,
            source_kind: accounts::AccountSourceKind::Derived,
            imported_vault_id: None,
            import_kind: None,
            source_metadata_json: None,
            label: "pending-account".to_owned(),
            status: accounts::AccountStatus::Pending,
            transaction_hash: None,
            created_at: 0,
            updated_at: 0,
        };
        let finalized = accounts::AccountRecord {
            status: accounts::AccountStatus::Finalized,
            label: "finalized-account".to_owned(),
            ..pending.clone()
        };

        assert!(matches_account_status(
            &pending,
            Some(AccountListStatus::Pending)
        ));
        assert!(matches_account_status(
            &finalized,
            Some(AccountListStatus::Finalized)
        ));
        assert!(!matches_account_status(
            &pending,
            Some(AccountListStatus::Finalized)
        ));
    }

    #[test]
    fn render_account_fuzzy_text_uses_bracket_first_rows() {
        let pending = accounts::AccountRecord {
            id: 1,
            seed_id: "seed-id".to_owned(),
            network_genesis_hash: "genesis".to_owned(),
            ip_identity: 0,
            identity_index: 0,
            credential_counter: 0,
            source_kind: accounts::AccountSourceKind::Derived,
            imported_vault_id: None,
            import_kind: None,
            source_metadata_json: None,
            label: "pending-account".to_owned(),
            status: accounts::AccountStatus::Pending,
            transaction_hash: None,
            created_at: 0,
            updated_at: 0,
        };
        let finalized = accounts::AccountRecord {
            status: accounts::AccountStatus::Finalized,
            label: "finalized-account".to_owned(),
            ..pending.clone()
        };

        assert_eq!(
            render_account_fuzzy_text(&pending, "test", "testnet", None, true),
            "[pending] [test] pending-account - testnet • provider:0 • identity:0 • cred:0"
        );
        assert_eq!(
            render_account_fuzzy_text(&finalized, "test", "testnet", Some("addr-test"), true),
            "[test] finalized-account (addr-test) - testnet • provider:0 • identity:0 • cred:0"
        );
        assert_eq!(
            render_account_fuzzy_text(&finalized, "test", "testnet", None, false),
            "[test] finalized-account • provider:0 • identity:0 • cred:0"
        );

        let imported = accounts::AccountRecord {
            source_kind: accounts::AccountSourceKind::Imported,
            imported_vault_id: Some("vault".to_owned()),
            import_kind: Some("genesis".to_owned()),
            source_metadata_json: None,
            seed_id: String::new(),
            label: "baker-0".to_owned(),
            ..finalized
        };
        assert_eq!(
            render_account_fuzzy_text(&imported, "", "local", None, true),
            "[imported] baker-0 - local"
        );
        assert_eq!(
            render_account_fuzzy_text(&imported, "", "local", None, false),
            "[imported] baker-0"
        );
    }

    #[test]
    fn account_list_defaults_to_all_seeds() {
        let conn = conn();
        let (scope, source) = resolve_account_list_seed_scope(&conn, None).unwrap();
        assert_eq!(scope, ScopeSelection::All);
        assert_eq!(source, ResolutionSource::Inferred);
    }

    #[test]
    fn account_show_raw_address_target_resolves_without_local_lookup() {
        let conn = conn();
        let context = account_show_test_context("genesis");
        let target = resolve_account_show_target(
            &conn,
            &context,
            "4UC8o4m8AgTxt5VBFMdLwMCwwJQVJwjesNzW7RPXkACynrULmd",
        )
        .unwrap();
        assert!(matches!(target, AccountShowTarget::RawAddress(_)));
    }

    #[test]
    fn account_show_local_label_is_constrained_by_network() {
        let conn = conn();
        let seed = seeds::add(&conn, "seed", TEST_SEED_PHRASE.as_bytes(), "password").unwrap();
        accounts::insert_pending(
            &conn,
            accounts::PendingAccount {
                network_genesis_hash: "genesis-1",
                seed_id: &seed.id,
                ip_identity: 0,
                identity_index: 0,
                credential_counter: 0,
                label: "alice",
            },
        )
        .unwrap();
        accounts::insert_pending(
            &conn,
            accounts::PendingAccount {
                network_genesis_hash: "genesis-2",
                seed_id: &seed.id,
                ip_identity: 0,
                identity_index: 0,
                credential_counter: 1,
                label: "alice",
            },
        )
        .unwrap();

        let target =
            resolve_account_show_target(&conn, &account_show_test_context("genesis-2"), "alice")
                .unwrap();
        match target {
            AccountShowTarget::LocalPending(metadata, _) => {
                assert_eq!(metadata.label, "alice");
                assert_eq!(metadata.seed.as_deref(), Some("seed"));
            }
            other => panic!("unexpected target: {other:?}"),
        }
    }

    #[test]
    fn account_show_header_formats_local_metadata() {
        let derived = AccountShowView::pending(
            "testnet".to_owned(),
            AccountShowLocalMetadata {
                label: "alice".to_owned(),
                seed: Some("main-seed".to_owned()),
                source: "derived".to_owned(),
            },
            None,
        );
        assert_eq!(
            render_account_show_header(&derived),
            "[main-seed : alice] testnet"
        );

        let imported = AccountShowView {
            network: "local".to_owned(),
            address: Some("addr".to_owned()),
            local: Some(AccountShowLocalMetadata {
                label: "genesis".to_owned(),
                seed: None,
                source: "imported".to_owned(),
            }),
            status: "finalized".to_owned(),
            transaction_hash: None,
            ccd: None,
            tokens: Vec::new(),
            protocol: None,
        };
        assert_eq!(
            render_account_show_header(&imported),
            "[genesis] addr @ local"
        );
    }

    #[test]
    fn account_show_balance_helpers_compute_locked_amounts() {
        let ccd_locked = ccd_locked_from_amounts(
            concordium_rust_sdk::common::types::Amount::from_micro_ccd(125),
            concordium_rust_sdk::common::types::Amount::from_micro_ccd(100),
        );
        assert_eq!(ccd_locked.micro_ccd(), 25);

        let token_locked = token_locked_amount(
            TokenAmount::from_raw(1_000, 2),
            TokenAmount::from_raw(750, 2),
        )
        .unwrap();
        assert_eq!(token_locked.value(), 250);
        assert_eq!(token_locked.decimals(), 2);
    }

    #[test]
    fn account_show_renders_default_verbose_pending_and_json() {
        let view = AccountShowView {
            network: "testnet".to_owned(),
            address: Some("addr".to_owned()),
            local: Some(AccountShowLocalMetadata {
                label: "alice".to_owned(),
                seed: Some("seed".to_owned()),
                source: "derived".to_owned(),
            }),
            status: "finalized".to_owned(),
            transaction_hash: None,
            ccd: Some(AccountShowCcdView {
                balance: "10.0 CCD".to_owned(),
                available: "7.0 CCD".to_owned(),
                locked: "3.0 CCD".to_owned(),
                release_schedule: vec![AccountShowReleaseView {
                    amount: "3.0 CCD".to_owned(),
                    timestamp: "2026-06-10T12:00:00Z".to_owned(),
                }],
            }),
            tokens: vec![AccountShowTokenView {
                id: "EUROe".to_owned(),
                balance: "1000.00".to_owned(),
                available: "750.00".to_owned(),
                locked: "250.00".to_owned(),
            }],
            protocol: Some(AccountShowProtocolView {
                account_index: "123".to_owned(),
                nonce: "42".to_owned(),
                credential_count: 1,
                threshold: "1".to_owned(),
                staking: "none".to_owned(),
            }),
        };
        let default = render_account_show(&view, false);
        assert!(default.contains("CCD balance: 10.0 CCD"));
        assert!(default.contains("release schedule:"));
        assert!(default.contains("EUROe balance: 1000.00"));
        assert!(!default.contains("next nonce"));

        let verbose = render_account_show(&view, true);
        assert!(verbose.contains("next nonce: 42"));
        assert!(verbose.contains("account index: 123"));

        let pending = AccountShowView::pending(
            "testnet".to_owned(),
            AccountShowLocalMetadata {
                label: "alice".to_owned(),
                seed: Some("seed".to_owned()),
                source: "derived".to_owned(),
            },
            Some("tx-hash".to_owned()),
        );
        assert!(render_account_show(&pending, false).contains("Status: pending"));
        let json = serde_json::to_string(&pending).unwrap();
        assert!(json.contains("tx-hash"));
    }

    fn account_show_test_context(genesis_hash: &str) -> AccountShowNetworkContext {
        AccountShowNetworkContext {
            endpoint: "http://localhost:20000".parse().unwrap(),
            endpoint_label: "http://localhost:20000".to_owned(),
            display_name: "testnet".to_owned(),
            genesis_hash: genesis_hash.to_owned(),
        }
    }

    #[test]
    fn show_addresses_requires_explicit_seed_argument() {
        let conn = conn();
        let err = resolve_seed_scope_for_addresses(&conn, None, true).unwrap_err();
        assert!(
            err.to_string()
                .contains("`--seed <LABEL>` is required with `--show-addresses`")
        );
    }

    #[test]
    fn export_output_requires_explicit_path_in_noninteractive_mode() {
        let err = resolve_export_output_path(None, "alice", true).unwrap_err();
        assert!(err.to_string().contains("--out <FILE>"));
    }

    #[test]
    fn export_output_expands_tilde_to_home_directory() {
        let path = resolve_export_output_path(
            Some(PathBuf::from("~/Downloads/export.json")),
            "alice",
            true,
        )
        .unwrap();
        assert!(path.is_absolute());
        assert!(path.ends_with("Downloads/export.json"));
    }

    #[test]
    fn exportable_accounts_only_include_finalized_records_for_selected_network() {
        let mut conn = conn();
        let seed = seeds::add(&conn, "seed", TEST_SEED_PHRASE.as_bytes(), "password").unwrap();
        let finalized_id = accounts::insert_pending(
            &conn,
            accounts::PendingAccount {
                network_genesis_hash: "genesis-a",
                seed_id: &seed.id,
                ip_identity: 0,
                identity_index: 0,
                credential_counter: 0,
                label: "finalized-a",
            },
        )
        .unwrap();
        let unlocked = seeds::unlock_context(&conn, "seed", "password").unwrap();
        accounts::set_finalized(
            &mut conn,
            finalized_id,
            &unlocked.dek,
            None,
            "4QkqdUnrjShrUrHpE96odLM6J77nWzEryifzqNnwNk4FYNge8a",
        )
        .unwrap();
        accounts::insert_pending(
            &conn,
            accounts::PendingAccount {
                network_genesis_hash: "genesis-a",
                seed_id: &seed.id,
                ip_identity: 0,
                identity_index: 0,
                credential_counter: 1,
                label: "pending-a",
            },
        )
        .unwrap();
        let other_network_id = accounts::insert_pending(
            &conn,
            accounts::PendingAccount {
                network_genesis_hash: "genesis-b",
                seed_id: &seed.id,
                ip_identity: 0,
                identity_index: 0,
                credential_counter: 2,
                label: "finalized-b",
            },
        )
        .unwrap();
        accounts::set_finalized(
            &mut conn,
            other_network_id,
            &unlocked.dek,
            None,
            "3AiAikM6wKghQ8cKfscQEfRdcfExsVMSzY4R1W3pG6f4R7k2aT",
        )
        .unwrap();

        let records = exportable_accounts_for_network(&conn, "genesis-a").unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].label, "finalized-a");
    }

    #[test]
    fn serialised_minimal_wallet_account_is_sdk_compatible() {
        let wallet = wallet_account_from_derived_parts(
            "4QkqdUnrjShrUrHpE96odLM6J77nWzEryifzqNnwNk4FYNge8a".to_owned(),
            TEST_SEED_PHRASE,
            &derived_record(),
            Net::Testnet,
        )
        .unwrap();
        let json = serialize_wallet_account_minimal(&wallet).unwrap();
        let reparsed = WalletAccount::from_json_str(&json).unwrap();
        assert_eq!(reparsed.address, wallet.address);
        let reparsed_json = serde_json::to_value(&reparsed.keys).unwrap();
        let original_json = serde_json::to_value(&wallet.keys).unwrap();
        assert_eq!(reparsed_json, original_json);
    }

    #[test]
    fn imported_payload_builds_sdk_compatible_wallet_account() {
        let payload = imported_payload("4QkqdUnrjShrUrHpE96odLM6J77nWzEryifzqNnwNk4FYNge8a");
        let wallet = wallet_account_from_imported_payload(&payload).unwrap();
        let json = serialize_wallet_account_minimal(&wallet).unwrap();
        let reparsed = WalletAccount::from_json_str(&json).unwrap();
        assert_eq!(reparsed.address, wallet.address);
        let reparsed_json = serde_json::to_value(&reparsed.keys).unwrap();
        let original_json = serde_json::to_value(&wallet.keys).unwrap();
        assert_eq!(reparsed_json, original_json);
    }

    #[test]
    fn export_file_writer_refuses_existing_destination() {
        let dir = std::env::temp_dir().join(format!(
            "ccd-wallet-export-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("account.json");
        fs::write(&path, "existing").unwrap();

        let err = write_export_file(&path, "{}").unwrap_err();
        assert!(
            err.to_string()
                .contains("refusing to overwrite existing file")
        );

        fs::remove_dir_all(&dir).unwrap();
    }
}
