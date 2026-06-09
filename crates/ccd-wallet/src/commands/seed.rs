use crate::{
    cli::{SeedSubcommand, SeedSyncArgs},
    commands::ui::{
        ContextLine, ResolutionSource, SelectItem, log_resolved_context, select_or_single,
    },
};
use anyhow::{Context, Result, bail};
use bip39::{Language, Mnemonic};
use ccd_wallet_core::{
    config,
    store::{accounts, config::NetworkEntry, crypto::KEY_LEN, identities, seeds, wallet_state},
    wallet::{ConcordiumHdWallet, CredId, Net, PrfKey},
};
use ccd_wallet_identity_provider::{
    build_recovery_request, build_recovery_request_from_id_cred_sec,
    client::{self, RecoveryResult, WalletProxyIpEntry},
};
use cliclack::{input, multi_progress, multiselect, password, progress_bar, spinner};
use concordium_rust_sdk::{
    endpoints::QueryError,
    id::{constants::IpPairing, types::GlobalContext},
    v2::{self, AccountIdentifier},
};
use console::Term;
use futures_util::{StreamExt, stream};
use rusqlite::Connection;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use zeroize::Zeroizing;

const SEED_REVEAL_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_EMPTY_IDENTITIES: u32 = 20;
const MAX_EMPTY_CREDENTIALS: u32 = 20;
const PROVIDER_CONCURRENCY: usize = 4;
const ACCOUNT_CONCURRENCY: usize = 4;

/// Enter the terminal alternate screen buffer (saves normal screen, shows blank buffer).
const ENTER_ALT_SCREEN: &str = "\x1b[?1049h";
/// Leave the terminal alternate screen buffer (restores normal screen).
const LEAVE_ALT_SCREEN: &str = "\x1b[?1049l";

pub trait SeedPrompts {
    fn prompt_seed_label(&mut self, prompt: &str) -> Result<String>;
    fn prompt_seed_label_with_placeholder(
        &mut self,
        prompt: &str,
        placeholder: &str,
    ) -> Result<String>;
    fn select_seed_label(
        &mut self,
        prompt: &str,
        items: &[SelectItem<String>],
        active: Option<&str>,
    ) -> Result<String>;
    fn select_provider_ids(
        &mut self,
        prompt: &str,
        items: &[SelectItem<u32>],
        initial: &[u32],
    ) -> Result<Vec<u32>>;
    fn prompt_seed_phrase(&mut self) -> Result<String>;
    fn prompt_password(&mut self) -> Result<String>;
    fn prompt_password_confirmation(&mut self) -> Result<String>;
    fn prompt_unlock_password(&mut self, label: &str) -> Result<String>;
    fn prompt_delete_confirmation(
        &mut self,
        label: &str,
        identity_count: usize,
        account_count: usize,
    ) -> Result<String>;
}

pub trait SeedPhraseRevealer {
    fn reveal(&mut self, label: &str, seed_phrase: &str) -> Result<()>;
}

pub struct TerminalSeedPrompts;

impl SeedPrompts for TerminalSeedPrompts {
    fn prompt_seed_label(&mut self, prompt: &str) -> Result<String> {
        Ok(input(prompt)
            .validate(|value: &String| {
                if value.is_empty() {
                    Err("Seed label is required.")
                } else {
                    Ok(())
                }
            })
            .interact()?)
    }

    fn prompt_seed_label_with_placeholder(
        &mut self,
        prompt: &str,
        placeholder: &str,
    ) -> Result<String> {
        Ok(input(prompt)
            .placeholder(placeholder)
            .validate(|value: &String| {
                if value.is_empty() {
                    Err("Seed label is required.")
                } else {
                    Ok(())
                }
            })
            .interact()?)
    }

    fn select_seed_label(
        &mut self,
        prompt: &str,
        items: &[SelectItem<String>],
        active: Option<&str>,
    ) -> Result<String> {
        let initial = active.map(str::to_owned);
        select_or_single(prompt, items, initial.as_ref())
    }

    fn select_provider_ids(
        &mut self,
        prompt: &str,
        items: &[SelectItem<u32>],
        initial: &[u32],
    ) -> Result<Vec<u32>> {
        if items.len() == 1 {
            return Ok(vec![items[0].value]);
        }

        let mut picker = multiselect(prompt).filter_mode();
        if !initial.is_empty() {
            picker = picker.initial_values(initial.to_vec());
        }
        for item in items {
            picker = picker.item(item.value, item.label.clone(), item.hint.clone());
        }
        Ok(picker.interact()?)
    }

    fn prompt_seed_phrase(&mut self) -> Result<String> {
        Ok(password("Enter seed phrase:").mask('▪').interact()?)
    }

    fn prompt_password(&mut self) -> Result<String> {
        Ok(password("Set password:")
            .mask('▪')
            .allow_empty()
            .interact()?)
    }

    fn prompt_password_confirmation(&mut self) -> Result<String> {
        Ok(password("Confirm password:")
            .mask('▪')
            .allow_empty()
            .interact()?)
    }

    fn prompt_unlock_password(&mut self, label: &str) -> Result<String> {
        Ok(password(format!("Password for seed '{label}':"))
            .mask('▪')
            .allow_empty()
            .interact()?)
    }

    fn prompt_delete_confirmation(
        &mut self,
        label: &str,
        identity_count: usize,
        account_count: usize,
    ) -> Result<String> {
        cliclack::log::warning(format!(
            "This will delete seed '{label}' and remove {} and {} owned by it.",
            format_count(identity_count, "identity", "identities"),
            format_count(account_count, "account", "accounts"),
        ))?;
        Ok(input(format!("Type '{label}' to confirm:"))
            .validate(|value: &String| {
                if value.is_empty() {
                    Err("Confirmation is required.")
                } else {
                    Ok(())
                }
            })
            .interact()?)
    }
}

pub struct TerminalSeedPhraseRevealer;

impl SeedPhraseRevealer for TerminalSeedPhraseRevealer {
    fn reveal(&mut self, label: &str, seed_phrase: &str) -> Result<()> {
        reveal_seed_phrase_until_key_or_timeout(label, seed_phrase, SEED_REVEAL_TIMEOUT)
    }
}

pub async fn run(conn: &mut Connection, command: SeedSubcommand) -> Result<()> {
    let mut prompts = TerminalSeedPrompts;
    let mut revealer = TerminalSeedPhraseRevealer;
    run_with_io(conn, command, &mut prompts, &mut revealer).await
}

async fn run_with_io(
    conn: &mut Connection,
    command: SeedSubcommand,
    prompts: &mut impl SeedPrompts,
    revealer: &mut impl SeedPhraseRevealer,
) -> Result<()> {
    match command {
        SeedSubcommand::Add(args) => {
            add(
                conn,
                args.label,
                args.random,
                args.restore,
                args.non_interactive,
                prompts,
                revealer,
            )
            .await
        }
        SeedSubcommand::Delete(args) => {
            delete_seed(conn, args.label, args.non_interactive, prompts).await
        }
        SeedSubcommand::List => list_seeds(conn).await,
        SeedSubcommand::Rename(args) => {
            rename_seed(
                conn,
                args.old_label,
                args.new_label,
                args.non_interactive,
                prompts,
            )
            .await
        }
        SeedSubcommand::Sync(args) => sync_seed(conn, args, prompts).await,
        SeedSubcommand::Use(args) => {
            use_seed(conn, args.label, args.non_interactive, prompts).await
        }
        SeedSubcommand::Show(args) => {
            show(conn, args.label, args.no_defaults, prompts, revealer).await
        }
    }
}

async fn add(
    conn: &mut Connection,
    label: Option<String>,
    random: bool,
    restore: Option<String>,
    non_interactive: bool,
    prompts: &mut impl SeedPrompts,
    revealer: &mut impl SeedPhraseRevealer,
) -> Result<()> {
    let label = resolve_required_seed_label(
        label,
        non_interactive,
        prompts,
        "Seed label:",
        "seed label must be provided in --non-interactive mode",
    )?;
    validate_seed_label(&label)?;

    if seeds::find_by_label(conn, &label)?.is_some() {
        bail!("seed label '{label}' already exists");
    }

    let restore_target = if let Some(network_name) = restore.as_deref() {
        Some(resolve_sync_network_context(conn, Some(network_name), non_interactive, false).await?)
    } else {
        None
    };

    let seed_phrase = if random {
        generate_seed_phrase()?
    } else {
        let seed_phrase = normalize_seed_phrase(&prompts.prompt_seed_phrase()?);
        validate_seed_phrase(&seed_phrase)?;
        seed_phrase
    };

    let password = prompts.prompt_password()?;
    let password_confirmation = prompts.prompt_password_confirmation()?;
    if password != password_confirmation {
        bail!("passwords do not match");
    }

    seeds::add(conn, &label, seed_phrase.as_bytes(), &password)?;

    if random {
        revealer.reveal(&label, &seed_phrase)?;
    }

    if restore_target.is_some() {
        cliclack::log::success(format!("Seed '{label}' added successfully."))?;
    } else {
        println!("Seed '{label}' added successfully.");
    }

    if let Some((resolved_network_name, network_entry, endpoint, endpoint_label, _)) =
        restore_target
    {
        let unlocked_seed = seeds::unlock_context(conn, &label, &password)?;
        log_resolved_context(&[
            ContextLine {
                label: "seed:",
                value: label.clone(),
                source: ResolutionSource::Explicit,
            },
            ContextLine {
                label: "network:",
                value: format!("{resolved_network_name} @ {endpoint_label}"),
                source: ResolutionSource::Explicit,
            },
        ])?;
        run_seed_recovery(
            conn,
            &label,
            &unlocked_seed,
            &resolved_network_name,
            &network_entry,
            endpoint,
            &[],
            non_interactive,
            prompts,
        )
        .await?;
    }

    Ok(())
}

async fn list_seeds(conn: &Connection) -> Result<()> {
    let active = wallet_state::get(conn, wallet_state::ACTIVE_SEED_KEY)?;
    let seeds = seeds::list(conn)?;
    let identities = identities::list(conn)?;
    let accounts = accounts::list(conn)?;
    for seed in seeds {
        let identity_count = identities
            .iter()
            .filter(|record| record.signer_owner_id == seed.id)
            .count();
        let account_count = accounts
            .iter()
            .filter(|record| record.signer_owner_id == seed.id)
            .count();
        println!(
            "{}",
            render_seed_list_text(
                &seed.label,
                active.as_deref() == Some(seed.label.as_str()),
                identity_count,
                account_count,
            )
        );
    }
    Ok(())
}

fn render_seed_list_text(
    label: &str,
    active: bool,
    identity_count: usize,
    account_count: usize,
) -> String {
    render_seed_text(label, active, identity_count, account_count, true)
}

fn render_seed_selector_text(label: &str, identity_count: usize, account_count: usize) -> String {
    render_seed_text(label, false, identity_count, account_count, false)
}

fn render_seed_text(
    label: &str,
    active: bool,
    identity_count: usize,
    account_count: usize,
    show_active: bool,
) -> String {
    let mut text = format!(
        "{label} — {} • {}",
        format_count(identity_count, "identity", "identities"),
        format_count(account_count, "account", "accounts"),
    );
    if show_active && active {
        text.push_str(" • active");
    }
    text
}

fn format_count(count: usize, singular: &str, plural: &str) -> String {
    let noun = if count == 1 { singular } else { plural };
    format!("{count} {noun}")
}

async fn rename_seed(
    conn: &Connection,
    old_label: Option<String>,
    new_label: Option<String>,
    non_interactive: bool,
    prompts: &mut impl SeedPrompts,
) -> Result<()> {
    let old_label = match old_label {
        Some(label) => label,
        None if non_interactive => bail!("seed label must be provided in --non-interactive mode"),
        None => select_seed_label(conn, prompts)?,
    };
    ensure_seed_exists(conn, &old_label)?;
    let new_label = match new_label {
        Some(label) => label,
        None if non_interactive => {
            bail!("new seed label must be provided in --non-interactive mode")
        }
        None => prompts.prompt_seed_label_with_placeholder("New seed label:", &old_label)?,
    };
    validate_seed_label(&new_label)?;
    seeds::rename(conn, &old_label, &new_label)?;
    if wallet_state::get(conn, wallet_state::ACTIVE_SEED_KEY)?.as_deref()
        == Some(old_label.as_str())
    {
        wallet_state::set(conn, wallet_state::ACTIVE_SEED_KEY, &new_label)?;
    }
    println!("Seed '{old_label}' renamed to '{new_label}'.");
    Ok(())
}

async fn use_seed(
    conn: &Connection,
    label: Option<String>,
    non_interactive: bool,
    prompts: &mut impl SeedPrompts,
) -> Result<()> {
    let label = match label {
        Some(label) => label,
        None if non_interactive => {
            bail!("seed label must be provided in --non-interactive mode")
        }
        None => select_seed_label(conn, prompts)?,
    };
    ensure_seed_exists(conn, &label)?;
    wallet_state::set(conn, wallet_state::ACTIVE_SEED_KEY, &label)?;

    println!("Active seed set to '{label}'.");

    Ok(())
}

async fn delete_seed(
    conn: &Connection,
    label: Option<String>,
    non_interactive: bool,
    prompts: &mut impl SeedPrompts,
) -> Result<()> {
    let label = resolve_required_seed_label(
        label,
        non_interactive,
        prompts,
        "Seed label:",
        "seed label must be provided in --non-interactive mode",
    )?;
    let seed = ensure_seed_exists(conn, &label)?;
    let identity_count = identities::list(conn)?
        .into_iter()
        .filter(|record| record.signer_owner_id == seed.id)
        .count();
    let account_count = accounts::list(conn)?
        .into_iter()
        .filter(|record| record.signer_owner_id == seed.id)
        .count();
    let confirmation = prompts.prompt_delete_confirmation(&label, identity_count, account_count)?;
    if confirmation != label {
        bail!("seed deletion aborted: confirmation did not match '{label}'");
    }

    seeds::remove(conn, &label)?;
    if wallet_state::get(conn, wallet_state::ACTIVE_SEED_KEY)?.as_deref() == Some(label.as_str()) {
        wallet_state::remove(conn, wallet_state::ACTIVE_SEED_KEY)?;
    }

    println!("Seed '{label}' deleted successfully.");

    Ok(())
}

async fn show(
    conn: &Connection,
    label: Option<String>,
    no_defaults: bool,
    prompts: &mut impl SeedPrompts,
    revealer: &mut impl SeedPhraseRevealer,
) -> Result<()> {
    let label = resolve_seed_label(conn, label, no_defaults, prompts)?;
    ensure_seed_exists(conn, &label)?;

    let password = prompts.prompt_unlock_password(&label)?;
    let seed_phrase = seeds::unlock(conn, &label, &password)?;
    let seed_phrase =
        std::str::from_utf8(&seed_phrase).context("stored seed phrase is not UTF-8")?;

    revealer.reveal(&label, seed_phrase)
}

fn ensure_seed_exists(conn: &Connection, label: &str) -> Result<seeds::SeedRecord> {
    seeds::find_by_label(conn, label)?.with_context(|| format!("seed '{label}' is not configured"))
}

fn resolve_required_seed_label(
    label: Option<String>,
    non_interactive: bool,
    prompts: &mut impl SeedPrompts,
    prompt: &str,
    error: &str,
) -> Result<String> {
    match label {
        Some(label) => Ok(label),
        None if non_interactive => bail!("{error}"),
        None => prompts.prompt_seed_label(prompt),
    }
}

fn resolve_seed_label(
    conn: &Connection,
    label: Option<String>,
    no_defaults: bool,
    prompts: &mut impl SeedPrompts,
) -> Result<String> {
    match label {
        Some(label) => Ok(label),
        None if no_defaults => select_seed_label(conn, prompts),
        None => wallet_state::get(conn, wallet_state::ACTIVE_SEED_KEY)?.with_context(
            || "no active seed is set; provide a seed label or run `ccd-wallet seed use <LABEL>`",
        ),
    }
}

fn select_seed_label(conn: &Connection, prompts: &mut impl SeedPrompts) -> Result<String> {
    let seeds = seeds::list(conn)?;
    if seeds.is_empty() {
        bail!("no seeds are configured; run `ccd-wallet seed add <LABEL>` first")
    }
    let identities = identities::list(conn)?;
    let accounts = accounts::list(conn)?;
    let active = wallet_state::get(conn, wallet_state::ACTIVE_SEED_KEY)?;
    let items = seeds
        .iter()
        .map(|seed| {
            let identity_count = identities
                .iter()
                .filter(|record| record.signer_owner_id == seed.id)
                .count();
            let account_count = accounts
                .iter()
                .filter(|record| record.signer_owner_id == seed.id)
                .count();
            SelectItem {
                value: seed.label.clone(),
                label: render_seed_selector_text(&seed.label, identity_count, account_count),
                hint: String::new(),
            }
        })
        .collect::<Vec<_>>();
    if items.len() == 1 {
        return Ok(items[0].value.clone());
    }
    prompts.select_seed_label("Select seed", &items, active.as_deref())
}

pub fn normalize_seed_phrase(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn validate_seed_phrase(normalized: &str) -> Result<()> {
    Mnemonic::parse_in_normalized(Language::English, normalized)
        .map(|_| ())
        .map_err(|err| anyhow::anyhow!("invalid seed phrase: {err}"))
}

pub fn generate_seed_phrase() -> Result<String> {
    Mnemonic::generate_in(Language::English, 24)
        .map(|mnemonic| mnemonic.to_string())
        .map_err(|err| anyhow::anyhow!("failed to generate seed phrase: {err}"))
}

pub fn validate_seed_label(label: &str) -> Result<()> {
    if label.is_empty() {
        bail!("seed label must not be empty");
    }

    if !label
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        bail!("seed labels may contain only ASCII letters, digits, dash, and underscore");
    }

    Ok(())
}

fn reveal_seed_phrase_until_key_or_timeout(
    label: &str,
    seed_phrase: &str,
    timeout: Duration,
) -> Result<()> {
    let term = Term::stdout();
    print!("{ENTER_ALT_SCREEN}");
    term.clear_screen()?;
    term.move_cursor_to(0, 0)?;

    let result = reveal_seed_phrase_inner(&term, label, seed_phrase, timeout);

    term.clear_screen()?;
    term.move_cursor_to(0, 0)?;
    print!("{LEAVE_ALT_SCREEN}");

    result
}

fn reveal_seed_phrase_inner(
    term: &Term,
    label: &str,
    seed_phrase: &str,
    timeout: Duration,
) -> Result<()> {
    term.write_line(&format!("Seed phrase for '{label}':\n"))?;
    term.write_line(&format!("{seed_phrase}\n"))?;
    term.write_line(
        "Copy this now. Press any key to hide. It will hide automatically in 30 seconds.",
    )?;

    wait_for_key_or_timeout(timeout);
    Ok(())
}

fn wait_for_key_or_timeout(timeout: Duration) {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = Term::stdout().read_key();
        let _ = tx.send(());
    });

    let _ = rx.recv_timeout(timeout);
}

#[derive(Clone, Debug)]
struct RecoveryProvider {
    provider_id: u32,
    name: String,
    recovery_start: String,
    ip_info: concordium_rust_sdk::id::types::IpInfo<IpPairing>,
}

#[derive(Clone, Debug)]
struct DiscoveredIdentity {
    provider_id: u32,
    identity_index: u32,
    identity_object: Value,
}

#[derive(Clone, Debug)]
struct DiscoveredAccount {
    provider_id: u32,
    identity_index: u32,
    credential_counter: u32,
    account_address: String,
}

#[derive(Clone, Debug, Default)]
struct RecoveryAggregate {
    total_providers: u64,
    queued_providers: u64,
    running_providers: u64,
    completed_providers: u64,
    failed_providers: u64,
    skipped_providers: u64,
    identity_probes: u64,
    account_probes: u64,
    discovered_identities: u64,
    discovered_accounts: u64,
    active: BTreeMap<String, String>,
}

#[derive(Debug, Default)]
struct RecoverySummary {
    inserted_identities: usize,
    inserted_accounts: usize,
    updated_identities: usize,
    updated_accounts: usize,
    skipped_providers: Vec<String>,
    failed_providers: Vec<String>,
}

#[derive(Debug)]
struct ProviderRecoveryOutput {
    provider_id: u32,
    provider_name: String,
    identities: Vec<DiscoveredIdentity>,
    accounts: Vec<DiscoveredAccount>,
}

#[derive(Clone)]
struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    fn check(&self) -> Result<()> {
        if self.is_cancelled() {
            bail!("recovery cancelled");
        }
        Ok(())
    }
}

struct TerminalRecoveryReporter {
    multi: cliclack::MultiProgress,
    provider_bar: cliclack::ProgressBar,
    detail: cliclack::ProgressBar,
}

impl TerminalRecoveryReporter {
    fn start(seed_label: &str, network_name: &str, total_providers: usize) -> Self {
        let multi = multi_progress(format!("Restoring seed '{seed_label}' on '{network_name}'"));
        let provider_bar = multi.add(progress_bar(total_providers as u64));
        provider_bar.start(format!("Providers complete: 0/{total_providers}"));
        let detail = multi.add(spinner());
        detail.start("Preparing recovery...");
        Self {
            multi,
            provider_bar,
            detail,
        }
    }

    fn update(&self, aggregate: &RecoveryAggregate) {
        let finished = aggregate.completed_providers
            + aggregate.failed_providers
            + aggregate.skipped_providers;
        self.provider_bar.set_position(finished);
        self.provider_bar.set_message(format!(
            "Providers complete: {finished}/{}",
            aggregate.total_providers
        ));

        let active = aggregate
            .active
            .values()
            .take(3)
            .cloned()
            .collect::<Vec<_>>();
        let mut message = format!(
            "Providers: {} queued • {} running • {} complete • {} failed\nRecovered: {} identities • {} accounts\nProbes: {} identities • {} accounts\nSkipped: {} provider{}",
            aggregate.queued_providers,
            aggregate.running_providers,
            aggregate.completed_providers,
            aggregate.failed_providers,
            aggregate.discovered_identities,
            aggregate.discovered_accounts,
            aggregate.identity_probes,
            aggregate.account_probes,
            aggregate.skipped_providers,
            if aggregate.skipped_providers == 1 {
                ""
            } else {
                "s"
            },
        );
        if !active.is_empty() {
            message.push_str("\nActive:\n");
            message.push_str(&active.join("\n"));
        }
        self.detail.set_message(message);
    }

    fn finish(&self) {
        self.multi.stop();
    }
}

async fn sync_seed(
    conn: &mut Connection,
    args: SeedSyncArgs,
    prompts: &mut impl SeedPrompts,
) -> Result<()> {
    let (seed_label, seed_source) = resolve_sync_seed_label(
        conn,
        args.label.as_deref(),
        args.non_interactive,
        args.no_defaults,
        prompts,
    )?;
    let (network_name, network_entry, endpoint, endpoint_label, network_source) =
        resolve_sync_network_context(
            conn,
            args.network.as_deref(),
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

    let password = prompts.prompt_unlock_password(&seed_label)?;
    let unlocked_seed = seeds::unlock_context(conn, &seed_label, &password)?;

    run_seed_recovery(
        conn,
        &seed_label,
        &unlocked_seed,
        &network_name,
        &network_entry,
        endpoint,
        &args.providers,
        args.non_interactive,
        prompts,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_seed_recovery(
    conn: &mut Connection,
    seed_label: &str,
    unlocked_seed: &seeds::UnlockedSeed,
    network_name: &str,
    network_entry: &NetworkEntry,
    endpoint: v2::Endpoint,
    explicit_provider_filters: &[String],
    non_interactive: bool,
    prompts: &mut impl SeedPrompts,
) -> Result<()> {
    let seed_phrase = std::str::from_utf8(&unlocked_seed.secret)
        .context("stored seed phrase is not UTF-8")?
        .to_owned();
    let net = infer_net(
        network_name,
        network_entry.wallet_proxy.as_deref(),
        &network_entry.node_endpoint,
    );
    let wallet = Arc::new(ConcordiumHdWallet::from_seed_phrase(&seed_phrase, net)?);

    let spin = spinner();
    spin.start(format!(
        "Connecting to node: {}",
        network_entry.node_endpoint
    ));
    let mut client = config::connect_v2_client(endpoint.clone())
        .await
        .with_context(|| {
            format!(
                "failed to connect to Concordium node at {}",
                network_entry.node_endpoint
            )
        })?;
    spin.clear();

    let spin = spinner();
    spin.start("Fetching chain cryptographic parameters...");
    let global_context = Arc::new(
        client
            .get_cryptographic_parameters(v2::BlockIdentifier::LastFinal)
            .await
            .with_context(|| {
                format!(
                    "failed to load cryptographic parameters from {}",
                    network_entry.node_endpoint
                )
            })?
            .response,
    );
    spin.clear();

    let spin = spinner();
    spin.start("Fetching identity providers...");
    let wallet_proxy = network_entry
        .wallet_proxy
        .as_deref()
        .context("selected network has no wallet_proxy configured")?;
    let wallet_proxy_entries = client::fetch_wallet_proxy_ip_info(wallet_proxy).await?;
    spin.clear();

    let (available_providers, skipped_providers) =
        extract_recovery_providers(&wallet_proxy_entries);
    let selected_providers = resolve_recovery_providers(
        &available_providers,
        explicit_provider_filters,
        non_interactive,
        prompts,
    )?;
    if selected_providers.is_empty() {
        bail!("no recovery-capable identity providers are available on the selected network");
    }

    let existing_identities = identities::list_by_network_and_signer_owner(
        conn,
        &network_entry.genesis_hash,
        &unlocked_seed.record.id,
    )?;
    let existing_accounts = accounts::list(conn)?
        .into_iter()
        .filter(|record| {
            record.network_genesis_hash == network_entry.genesis_hash
                && record.signer_owner_id == unlocked_seed.record.id
        })
        .collect::<Vec<_>>();

    let identity_statuses = existing_identities.iter().fold(
        BTreeMap::<u32, BTreeMap<u32, identities::IdentityStatus>>::new(),
        |mut acc, record| {
            acc.entry(record.ip_identity)
                .or_default()
                .insert(record.identity_index, record.status);
            acc
        },
    );
    let used_accounts = existing_accounts.iter().fold(
        BTreeMap::<(u32, u32), BTreeSet<u32>>::new(),
        |mut acc, record| {
            acc.entry((record.ip_identity, record.identity_index))
                .or_default()
                .insert(record.credential_counter);
            acc
        },
    );

    let cancellation = CancellationToken::new();
    let ctrl_c_cancellation = cancellation.clone();
    let ctrl_c_task = tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        ctrl_c_cancellation.cancel();
    });

    let aggregate = Arc::new(Mutex::new(RecoveryAggregate {
        total_providers: selected_providers.len() as u64,
        queued_providers: selected_providers.len() as u64,
        skipped_providers: skipped_providers.len() as u64,
        ..Default::default()
    }));
    let reporter =
        TerminalRecoveryReporter::start(seed_label, network_name, selected_providers.len());
    reporter.update(&aggregate.lock().unwrap());

    let mut provider_stream = std::pin::pin!(
        stream::iter(selected_providers.into_iter().map(|provider| {
            let wallet = wallet.clone();
            let global_context = global_context.clone();
            let endpoint = endpoint.clone();
            let aggregate = aggregate.clone();
            let statuses = identity_statuses
                .get(&provider.provider_id)
                .cloned()
                .unwrap_or_default();
            let used_accounts = used_accounts.clone();
            let cancellation = cancellation.clone();
            async move {
                recover_provider(
                    provider,
                    wallet,
                    global_context,
                    endpoint,
                    statuses,
                    used_accounts,
                    aggregate,
                    cancellation,
                )
                .await
            }
        }))
        .buffer_unordered(PROVIDER_CONCURRENCY)
    );

    let mut outputs = Vec::new();
    let mut failed_providers = Vec::new();
    let mut ticker = tokio::time::interval(Duration::from_millis(100));

    let run_result: Result<()> = loop {
        tokio::select! {
            maybe = provider_stream.next() => {
                match maybe {
                    Some(Ok(output)) => outputs.push(output),
                    Some(Err((provider_name, error))) => failed_providers.push(format!("{provider_name}: {error}")),
                    None => break Ok(()),
                }
            }
            _ = ticker.tick() => {
                if cancellation.is_cancelled() {
                    break Err(anyhow::anyhow!("recovery cancelled"));
                }
                reporter.update(&aggregate.lock().unwrap());
            }
        }
    };
    ctrl_c_task.abort();
    reporter.finish();
    run_result?;

    let mut summary = RecoverySummary {
        skipped_providers,
        failed_providers,
        ..Default::default()
    };

    for output in outputs {
        let _ = (&output.provider_id, &output.provider_name);
        for identity in output.identities {
            let label = recovered_identity_label(
                conn,
                &network_entry.genesis_hash,
                &unlocked_seed.record.id,
                identity.provider_id,
                identity.identity_index,
            )?;
            let (_, inserted) = identities::import_recovered(
                conn,
                &unlocked_seed.dek,
                identities::RecoveredIdentity {
                    network_genesis_hash: &network_entry.genesis_hash,
                    signer_owner_id: &unlocked_seed.record.id,
                    ip_identity: identity.provider_id,
                    identity_index: identity.identity_index,
                    label: &label,
                    identity_object: &identity.identity_object,
                },
            )?;
            if inserted {
                summary.inserted_identities += 1;
            } else {
                summary.updated_identities += 1;
            }
        }
        for account in output.accounts {
            let label = recovered_account_label(
                conn,
                &network_entry.genesis_hash,
                &unlocked_seed.record.id,
                account.provider_id,
                account.identity_index,
                account.credential_counter,
            )?;
            let (_, inserted) = accounts::import_recovered(
                conn,
                &unlocked_seed.dek,
                accounts::RecoveredAccount {
                    network_genesis_hash: &network_entry.genesis_hash,
                    signer_owner_id: &unlocked_seed.record.id,
                    ip_identity: account.provider_id,
                    identity_index: account.identity_index,
                    credential_counter: account.credential_counter,
                    label: &label,
                    account_address: &account.account_address,
                },
            )?;
            if inserted {
                summary.inserted_accounts += 1;
            } else {
                summary.updated_accounts += 1;
            }
        }
    }

    print_recovery_summary("seed", seed_label, network_name, &summary);

    if summary.inserted_identities == 0
        && summary.updated_identities == 0
        && summary.inserted_accounts == 0
        && summary.updated_accounts == 0
        && !summary.failed_providers.is_empty()
    {
        bail!("recovery failed for all selected providers");
    }

    Ok(())
}

/// Identity recovery material for one provider/identity tuple.
pub(crate) struct IdentityRecoveryMaterial {
    /// Identity credential secret used to build identity recovery requests.
    pub id_cred_sec: CredId,
}

/// Account discovery material for one recovered provider/identity tuple.
pub(crate) struct AccountRecoveryMaterial {
    /// PRF key used to derive credential registration IDs for account discovery.
    pub prf_key: PrfKey,
}

/// Run recovery for a Ledger-backed key source using sequential device-backed probing.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_ledger_recovery(
    conn: &mut Connection,
    key_source_label: &str,
    signer_owner_id: &str,
    signer_owner_dek: &Zeroizing<[u8; KEY_LEN]>,
    network_name: &str,
    network_entry: &NetworkEntry,
    endpoint: v2::Endpoint,
    explicit_provider_filters: &[String],
    non_interactive: bool,
    prompts: &mut impl SeedPrompts,
    identity_material_for: &mut impl FnMut(u32, u32) -> Result<IdentityRecoveryMaterial>,
    account_material_for: &mut impl FnMut(u32, u32) -> Result<AccountRecoveryMaterial>,
) -> Result<()> {
    let spin = spinner();
    spin.start(format!(
        "Connecting to node: {}",
        network_entry.node_endpoint
    ));
    let mut client = config::connect_v2_client(endpoint.clone())
        .await
        .with_context(|| {
            format!(
                "failed to connect to Concordium node at {}",
                network_entry.node_endpoint
            )
        })?;
    spin.clear();

    let spin = spinner();
    spin.start("Fetching chain cryptographic parameters...");
    let global_context = Arc::new(
        client
            .get_cryptographic_parameters(v2::BlockIdentifier::LastFinal)
            .await
            .with_context(|| {
                format!(
                    "failed to load cryptographic parameters from {}",
                    network_entry.node_endpoint
                )
            })?
            .response,
    );
    spin.clear();

    let spin = spinner();
    spin.start("Fetching identity providers...");
    let wallet_proxy = network_entry
        .wallet_proxy
        .as_deref()
        .context("selected network has no wallet_proxy configured")?;
    let wallet_proxy_entries = client::fetch_wallet_proxy_ip_info(wallet_proxy).await?;
    spin.clear();

    let (available_providers, skipped_providers) =
        extract_recovery_providers(&wallet_proxy_entries);
    let selected_providers = resolve_recovery_providers(
        &available_providers,
        explicit_provider_filters,
        non_interactive,
        prompts,
    )?;
    if selected_providers.is_empty() {
        bail!("no recovery-capable identity providers are available on the selected network");
    }

    let existing_identities = identities::list_by_network_and_signer_owner(
        conn,
        &network_entry.genesis_hash,
        signer_owner_id,
    )?;
    let existing_accounts = accounts::list(conn)?
        .into_iter()
        .filter(|record| {
            record.network_genesis_hash == network_entry.genesis_hash
                && record.signer_owner_id == signer_owner_id
        })
        .collect::<Vec<_>>();

    let identity_statuses = existing_identities.iter().fold(
        BTreeMap::<u32, BTreeMap<u32, identities::IdentityStatus>>::new(),
        |mut acc, record| {
            acc.entry(record.ip_identity)
                .or_default()
                .insert(record.identity_index, record.status);
            acc
        },
    );
    let used_accounts = existing_accounts.iter().fold(
        BTreeMap::<(u32, u32), BTreeSet<u32>>::new(),
        |mut acc, record| {
            acc.entry((record.ip_identity, record.identity_index))
                .or_default()
                .insert(record.credential_counter);
            acc
        },
    );

    let cancellation = CancellationToken::new();
    let ctrl_c_cancellation = cancellation.clone();
    let ctrl_c_task = tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        ctrl_c_cancellation.cancel();
    });

    let aggregate = Arc::new(Mutex::new(RecoveryAggregate {
        total_providers: selected_providers.len() as u64,
        queued_providers: selected_providers.len() as u64,
        skipped_providers: skipped_providers.len() as u64,
        ..Default::default()
    }));
    let reporter =
        TerminalRecoveryReporter::start(key_source_label, network_name, selected_providers.len());
    reporter.update(&aggregate.lock().unwrap());

    let mut outputs = Vec::new();
    let mut failed_providers = Vec::new();
    for provider in selected_providers {
        if cancellation.is_cancelled() {
            break;
        }
        let statuses = identity_statuses
            .get(&provider.provider_id)
            .cloned()
            .unwrap_or_default();
        let provider_used_accounts = used_accounts.clone();
        match recover_ledger_provider(
            provider,
            global_context.clone(),
            endpoint.clone(),
            statuses,
            provider_used_accounts,
            aggregate.clone(),
            cancellation.clone(),
            identity_material_for,
            account_material_for,
        )
        .await
        {
            Ok(output) => outputs.push(output),
            Err((provider_name, error)) => {
                failed_providers.push(format!("{provider_name}: {error}"))
            }
        }
        reporter.update(&aggregate.lock().unwrap());
    }
    ctrl_c_task.abort();
    reporter.finish();
    if cancellation.is_cancelled() {
        bail!("recovery cancelled");
    }

    let mut summary = RecoverySummary {
        skipped_providers,
        failed_providers,
        ..Default::default()
    };

    for output in outputs {
        let _ = (&output.provider_id, &output.provider_name);
        for identity in output.identities {
            let label = recovered_identity_label(
                conn,
                &network_entry.genesis_hash,
                signer_owner_id,
                identity.provider_id,
                identity.identity_index,
            )?;
            let (_, inserted) = identities::import_recovered(
                conn,
                signer_owner_dek,
                identities::RecoveredIdentity {
                    network_genesis_hash: &network_entry.genesis_hash,
                    signer_owner_id,
                    ip_identity: identity.provider_id,
                    identity_index: identity.identity_index,
                    label: &label,
                    identity_object: &identity.identity_object,
                },
            )?;
            if inserted {
                summary.inserted_identities += 1;
            } else {
                summary.updated_identities += 1;
            }
        }
        for account in output.accounts {
            let label = recovered_account_label(
                conn,
                &network_entry.genesis_hash,
                signer_owner_id,
                account.provider_id,
                account.identity_index,
                account.credential_counter,
            )?;
            let (_, inserted) = accounts::import_recovered(
                conn,
                signer_owner_dek,
                accounts::RecoveredAccount {
                    network_genesis_hash: &network_entry.genesis_hash,
                    signer_owner_id,
                    ip_identity: account.provider_id,
                    identity_index: account.identity_index,
                    credential_counter: account.credential_counter,
                    label: &label,
                    account_address: &account.account_address,
                },
            )?;
            if inserted {
                summary.inserted_accounts += 1;
            } else {
                summary.updated_accounts += 1;
            }
        }
    }

    print_recovery_summary(
        "Ledger key source",
        key_source_label,
        network_name,
        &summary,
    );

    if summary.inserted_identities == 0
        && summary.updated_identities == 0
        && summary.inserted_accounts == 0
        && summary.updated_accounts == 0
        && !summary.failed_providers.is_empty()
    {
        bail!("recovery failed for all selected providers");
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn recover_ledger_provider(
    provider: RecoveryProvider,
    global_context: Arc<GlobalContext<concordium_rust_sdk::id::constants::ArCurve>>,
    endpoint: v2::Endpoint,
    existing_statuses: BTreeMap<u32, identities::IdentityStatus>,
    existing_accounts: BTreeMap<(u32, u32), BTreeSet<u32>>,
    aggregate: Arc<Mutex<RecoveryAggregate>>,
    cancellation: CancellationToken,
    identity_material_for: &mut impl FnMut(u32, u32) -> Result<IdentityRecoveryMaterial>,
    account_material_for: &mut impl FnMut(u32, u32) -> Result<AccountRecoveryMaterial>,
) -> std::result::Result<ProviderRecoveryOutput, (String, anyhow::Error)> {
    update_provider_start(&aggregate, provider.provider_id, &provider.name);

    let run = async {
        let mut identity_index = 0u32;
        let mut identities = Vec::new();
        let mut accounts_found = Vec::new();

        for (known_identity_index, status) in &existing_statuses {
            cancellation.check()?;
            if *status != identities::IdentityStatus::Done {
                continue;
            }
            identity_index = identity_index.max(known_identity_index.saturating_add(1));
            let material = account_material_for(provider.provider_id, *known_identity_index)?;
            let used = existing_accounts
                .get(&(provider.provider_id, *known_identity_index))
                .cloned()
                .unwrap_or_default();
            accounts_found.extend(
                recover_ledger_accounts_for_identity(
                    material.prf_key,
                    global_context.clone(),
                    endpoint.clone(),
                    provider.provider_id,
                    *known_identity_index,
                    provider.name.clone(),
                    used,
                    aggregate.clone(),
                    cancellation.clone(),
                )
                .await?,
            );
        }

        while existing_statuses.contains_key(&identity_index) {
            identity_index += 1;
        }

        loop {
            cancellation.check()?;
            set_active(
                &aggregate,
                format!("provider:{}", provider.provider_id),
                format!(
                    "- {} (id {}): probing identity {}",
                    provider.name, provider.provider_id, identity_index
                ),
            );
            increment_identity_probes(&aggregate);

            let material = identity_material_for(provider.provider_id, identity_index)?;
            let recovery_request = build_recovery_request_from_id_cred_sec(
                material.id_cred_sec,
                &provider.ip_info,
                &global_context,
                now_unix_timestamp()?,
            )?;
            match client::recover_identity(&provider.recovery_start, &recovery_request).await? {
                RecoveryResult::Recovered(identity_object) => {
                    increment_discovered_identities(&aggregate);
                    identities.push(DiscoveredIdentity {
                        provider_id: provider.provider_id,
                        identity_index,
                        identity_object,
                    });
                    let material = account_material_for(provider.provider_id, identity_index)?;
                    let used = existing_accounts
                        .get(&(provider.provider_id, identity_index))
                        .cloned()
                        .unwrap_or_default();
                    accounts_found.extend(
                        recover_ledger_accounts_for_identity(
                            material.prf_key,
                            global_context.clone(),
                            endpoint.clone(),
                            provider.provider_id,
                            identity_index,
                            provider.name.clone(),
                            used,
                            aggregate.clone(),
                            cancellation.clone(),
                        )
                        .await?,
                    );
                }
                RecoveryResult::Missing => break,
            }
            identity_index += 1;
            while existing_statuses.contains_key(&identity_index) {
                identity_index += 1;
            }
        }

        Ok(ProviderRecoveryOutput {
            provider_id: provider.provider_id,
            provider_name: provider.name.clone(),
            identities,
            accounts: accounts_found,
        })
    }
    .await;

    clear_provider_active(&aggregate, provider.provider_id);
    match run {
        Ok(output) => {
            update_provider_complete(&aggregate, provider.provider_id);
            Ok(output)
        }
        Err(error) => {
            update_provider_failed(&aggregate, provider.provider_id);
            Err((provider.name, error))
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn recover_ledger_accounts_for_identity(
    prf_key: PrfKey,
    global_context: Arc<GlobalContext<concordium_rust_sdk::id::constants::ArCurve>>,
    endpoint: v2::Endpoint,
    provider_id: u32,
    identity_index: u32,
    provider_name: String,
    mut used_counters: BTreeSet<u32>,
    aggregate: Arc<Mutex<RecoveryAggregate>>,
    cancellation: CancellationToken,
) -> Result<Vec<DiscoveredAccount>> {
    let mut client = config::connect_v2_client(endpoint.clone())
        .await
        .with_context(|| {
            format!(
                "failed to connect to Concordium node at {}",
                config::endpoint_label(&endpoint)
            )
        })?;

    let mut accounts = Vec::new();
    let mut empty = 0u32;
    let mut credential_counter = next_unused_u32(&used_counters);

    while empty < MAX_EMPTY_CREDENTIALS {
        cancellation.check()?;
        if used_counters.contains(&credential_counter) {
            credential_counter += 1;
            continue;
        }
        let credential_counter_u8 = u8::try_from(credential_counter)
            .context("credential counter exceeded supported protocol range")?;
        set_active(
            &aggregate,
            format!("account:{provider_id}:{identity_index}"),
            format!(
                "- {} (id {}) / identity {}: probing credential {}",
                provider_name, provider_id, identity_index, credential_counter
            ),
        );
        increment_account_probes(&aggregate);

        let cred_id_exponent = prf_key
            .prf_exponent(credential_counter_u8)
            .context("failed to derive credential registration id exponent")?;
        let cred_id = concordium_rust_sdk::base::base::CredentialRegistrationID::from_exponent(
            &global_context,
            cred_id_exponent,
        );
        let identifier = AccountIdentifier::CredId(cred_id);
        match client
            .get_account_info(&identifier, v2::BlockIdentifier::LastFinal)
            .await
        {
            Ok(info) => {
                increment_discovered_accounts(&aggregate);
                let address = format!("{}", info.response.account_address);
                used_counters.insert(credential_counter);
                accounts.push(DiscoveredAccount {
                    provider_id,
                    identity_index,
                    credential_counter,
                    account_address: address,
                });
                empty = 0;
            }
            Err(err) if is_recoverable_account_miss(&err) => empty += 1,
            Err(err) => {
                return Err(err).context("failed to query account information during recovery");
            }
        }
        credential_counter += 1;
    }

    clear_active(
        &aggregate,
        &format!("account:{provider_id}:{identity_index}"),
    );
    Ok(accounts)
}

#[allow(clippy::too_many_arguments)]
async fn recover_provider(
    provider: RecoveryProvider,
    wallet: Arc<ConcordiumHdWallet>,
    global_context: Arc<GlobalContext<concordium_rust_sdk::id::constants::ArCurve>>,
    endpoint: v2::Endpoint,
    existing_statuses: BTreeMap<u32, identities::IdentityStatus>,
    existing_accounts: BTreeMap<(u32, u32), BTreeSet<u32>>,
    aggregate: Arc<Mutex<RecoveryAggregate>>,
    cancellation: CancellationToken,
) -> std::result::Result<ProviderRecoveryOutput, (String, anyhow::Error)> {
    update_provider_start(&aggregate, provider.provider_id, &provider.name);

    let run = async {
        let mut empty_indices = 0u32;
        let mut identity_index = 0u32;
        let mut identities = Vec::new();
        let mut account_identities = BTreeSet::new();

        while empty_indices < MAX_EMPTY_IDENTITIES {
            cancellation.check()?;
            set_active(
                &aggregate,
                format!("provider:{}", provider.provider_id),
                format!(
                    "- {} (id {}): probing identity {}",
                    provider.name, provider.provider_id, identity_index
                ),
            );
            increment_identity_probes(&aggregate);

            if let Some(status) = existing_statuses.get(&identity_index) {
                empty_indices = 0;
                if *status == identities::IdentityStatus::Done {
                    account_identities.insert(identity_index);
                }
                identity_index += 1;
                continue;
            }

            let recovery_request = build_recovery_request(
                &wallet,
                &provider.ip_info,
                &global_context,
                identity_index,
                now_unix_timestamp()?,
            )?;
            match client::recover_identity(&provider.recovery_start, &recovery_request).await? {
                RecoveryResult::Recovered(identity_object) => {
                    increment_discovered_identities(&aggregate);
                    account_identities.insert(identity_index);
                    identities.push(DiscoveredIdentity {
                        provider_id: provider.provider_id,
                        identity_index,
                        identity_object,
                    });
                    empty_indices = 0;
                }
                RecoveryResult::Missing => empty_indices += 1,
            }
            identity_index += 1;
        }

        let mut accounts_found = Vec::new();
        let mut account_stream = std::pin::pin!(
            stream::iter(account_identities.into_iter().map(|identity_index| {
                let wallet = wallet.clone();
                let global_context = global_context.clone();
                let endpoint = endpoint.clone();
                let aggregate = aggregate.clone();
                let provider_name = provider.name.clone();
                let cancellation = cancellation.clone();
                let used = existing_accounts
                    .get(&(provider.provider_id, identity_index))
                    .cloned()
                    .unwrap_or_default();
                async move {
                    recover_accounts_for_identity(
                        wallet,
                        global_context,
                        endpoint,
                        provider.provider_id,
                        identity_index,
                        provider_name,
                        used,
                        aggregate,
                        cancellation,
                    )
                    .await
                }
            }))
            .buffer_unordered(ACCOUNT_CONCURRENCY)
        );
        while let Some(result) = account_stream.next().await {
            cancellation.check()?;
            accounts_found.extend(result?);
        }

        Ok(ProviderRecoveryOutput {
            provider_id: provider.provider_id,
            provider_name: provider.name.clone(),
            identities,
            accounts: accounts_found,
        })
    }
    .await;

    clear_provider_active(&aggregate, provider.provider_id);
    match run {
        Ok(output) => {
            update_provider_complete(&aggregate, provider.provider_id);
            Ok(output)
        }
        Err(error) => {
            update_provider_failed(&aggregate, provider.provider_id);
            Err((provider.name, error))
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn recover_accounts_for_identity(
    wallet: Arc<ConcordiumHdWallet>,
    global_context: Arc<GlobalContext<concordium_rust_sdk::id::constants::ArCurve>>,
    endpoint: v2::Endpoint,
    provider_id: u32,
    identity_index: u32,
    provider_name: String,
    mut used_counters: BTreeSet<u32>,
    aggregate: Arc<Mutex<RecoveryAggregate>>,
    cancellation: CancellationToken,
) -> Result<Vec<DiscoveredAccount>> {
    let mut client = config::connect_v2_client(endpoint.clone())
        .await
        .with_context(|| {
            format!(
                "failed to connect to Concordium node at {}",
                config::endpoint_label(&endpoint)
            )
        })?;

    let mut accounts = Vec::new();
    let mut empty = 0u32;
    let mut credential_counter = next_unused_u32(&used_counters);

    while empty < MAX_EMPTY_CREDENTIALS {
        cancellation.check()?;
        if used_counters.contains(&credential_counter) {
            credential_counter += 1;
            continue;
        }
        let credential_counter_u8 = u8::try_from(credential_counter)
            .context("credential counter exceeded supported protocol range")?;
        set_active(
            &aggregate,
            format!("account:{provider_id}:{identity_index}"),
            format!(
                "- {} (id {}) / identity {}: probing credential {}",
                provider_name, provider_id, identity_index, credential_counter
            ),
        );
        increment_account_probes(&aggregate);

        let cred_id = wallet.get_credential_registration_id(
            provider_id,
            identity_index,
            credential_counter_u8,
            &global_context,
        )?;
        let identifier = AccountIdentifier::CredId(cred_id);
        match client
            .get_account_info(&identifier, v2::BlockIdentifier::LastFinal)
            .await
        {
            Ok(info) => {
                increment_discovered_accounts(&aggregate);
                let address = format!("{}", info.response.account_address);
                used_counters.insert(credential_counter);
                accounts.push(DiscoveredAccount {
                    provider_id,
                    identity_index,
                    credential_counter,
                    account_address: address,
                });
                empty = 0;
            }
            Err(err) if is_recoverable_account_miss(&err) => empty += 1,
            Err(err) => {
                return Err(err).context("failed to query account information during recovery");
            }
        }
        credential_counter += 1;
    }

    clear_active(
        &aggregate,
        &format!("account:{provider_id}:{identity_index}"),
    );
    Ok(accounts)
}

fn extract_recovery_providers(
    entries: &[WalletProxyIpEntry],
) -> (Vec<RecoveryProvider>, Vec<String>) {
    let (_, skipped) = classify_recovery_provider_metadata(entries.iter().map(|entry| {
        (
            entry.ip_info.ip_identity.0,
            entry.ip_info.ip_description.name.clone(),
            entry.metadata.recovery_start.clone(),
        )
    }));

    let available = entries
        .iter()
        .filter_map(|entry| {
            entry
                .metadata
                .recovery_start
                .clone()
                .map(|recovery_start| RecoveryProvider {
                    provider_id: entry.ip_info.ip_identity.0,
                    name: entry.ip_info.ip_description.name.clone(),
                    recovery_start,
                    ip_info: entry.ip_info.clone(),
                })
        })
        .collect();
    (available, skipped)
}

fn classify_recovery_provider_metadata(
    items: impl IntoIterator<Item = (u32, String, Option<String>)>,
) -> (Vec<u32>, Vec<String>) {
    let mut available = Vec::new();
    let mut skipped = Vec::new();
    for (provider_id, name, recovery_start) in items {
        if recovery_start.is_some() {
            available.push(provider_id);
        } else {
            skipped.push(format!("{name} (id {provider_id})"));
        }
    }
    (available, skipped)
}

fn resolve_recovery_providers(
    providers: &[RecoveryProvider],
    explicit_filters: &[String],
    non_interactive: bool,
    prompts: &mut impl SeedPrompts,
) -> Result<Vec<RecoveryProvider>> {
    if providers.is_empty() {
        return Ok(Vec::new());
    }

    let available_ids = providers
        .iter()
        .map(|provider| provider.provider_id)
        .collect::<BTreeSet<_>>();
    if let Some(selected_ids) = resolve_explicit_provider_ids(&available_ids, explicit_filters)? {
        return Ok(filter_providers_by_ids(providers, &selected_ids));
    }

    if non_interactive || providers.len() == 1 {
        return Ok(providers.to_vec());
    }
    let items = providers
        .iter()
        .map(|provider| SelectItem {
            value: provider.provider_id,
            label: provider.name.clone(),
            hint: format!("provider id: {}", provider.provider_id),
        })
        .collect::<Vec<_>>();
    let initial = providers
        .iter()
        .map(|provider| provider.provider_id)
        .collect::<Vec<_>>();
    let selected_ids =
        prompts.select_provider_ids("Select identity providers", &items, &initial)?;
    Ok(filter_providers_by_ids(providers, &selected_ids))
}

fn resolve_explicit_provider_ids(
    available_ids: &BTreeSet<u32>,
    explicit_filters: &[String],
) -> Result<Option<Vec<u32>>> {
    if explicit_filters.is_empty() {
        return Ok(None);
    }

    let all_count = explicit_filters
        .iter()
        .filter(|value| value.as_str() == "all")
        .count();
    if all_count > 0 && explicit_filters.len() > 1 {
        bail!("`all` cannot be combined with specific providers");
    }
    if all_count == 1 {
        return Ok(Some(available_ids.iter().copied().collect()));
    }

    let selected_ids = explicit_filters
        .iter()
        .map(|value| {
            value.parse::<u32>().with_context(|| {
                format!("invalid provider value '{value}'; use `all` or a numeric provider id")
            })
        })
        .collect::<Result<Vec<_>>>()?;

    for provider_id in &selected_ids {
        if !available_ids.contains(provider_id) {
            bail!(
                "identity provider {} is unavailable on the chosen network",
                provider_id
            );
        }
    }

    Ok(Some(selected_ids))
}

fn filter_providers_by_ids(
    providers: &[RecoveryProvider],
    selected_ids: &[u32],
) -> Vec<RecoveryProvider> {
    let selected = selected_ids.iter().copied().collect::<BTreeSet<_>>();
    providers
        .iter()
        .filter(|provider| selected.contains(&provider.provider_id))
        .cloned()
        .collect()
}

fn resolve_sync_seed_label(
    conn: &Connection,
    explicit: Option<&str>,
    non_interactive: bool,
    no_defaults: bool,
    prompts: &mut impl SeedPrompts,
) -> Result<(String, ResolutionSource)> {
    match explicit {
        Some(label) => seeds::find_by_label(conn, label)?
            .map(|record| (record.label, ResolutionSource::Explicit))
            .with_context(|| format!("seed '{}' is not configured", label)),
        None => {
            let active = wallet_state::get(conn, wallet_state::ACTIVE_SEED_KEY)?;
            if no_defaults {
                return Ok((
                    select_seed_label(conn, prompts)?,
                    ResolutionSource::Prompted,
                ));
            }
            match active {
                Some(label) => Ok((label, ResolutionSource::ActiveDefault)),
                None if non_interactive => bail!(
                    "No active seed. Run `ccd-wallet seed use <LABEL>` or supply a seed label explicitly."
                ),
                None => Ok((
                    select_seed_label(conn, prompts)?,
                    ResolutionSource::Prompted,
                )),
            }
        }
    }
}

pub(crate) async fn resolve_sync_network_context(
    conn: &Connection,
    network: Option<&str>,
    non_interactive: bool,
    no_defaults: bool,
) -> Result<(String, NetworkEntry, v2::Endpoint, String, ResolutionSource)> {
    let app_config = ccd_wallet_core::store::config::load()?;
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
        .with_context(|| {
            format!(
                "network '{}' is not registered; run `ccd-wallet network add --name {} --node <ENDPOINT> --wallet-proxy <URL>` first",
                selected_network, selected_network
            )
        })?;
    let endpoint: v2::Endpoint =
        ccd_wallet_core::config::normalize_url_string(&entry.node_endpoint)
            .parse()
            .with_context(|| {
                format!(
                    "network '{}' has an invalid stored endpoint: {}",
                    selected_network, entry.node_endpoint
                )
            })?;
    let endpoint_label = config::endpoint_label(&endpoint);
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

fn prompt_for_network_name(
    app_config: &ccd_wallet_core::store::config::AppConfig,
    active: Option<&str>,
) -> Result<String> {
    if app_config.networks.is_empty() {
        bail!("no networks are configured; run `ccd-wallet network add` first")
    }
    let items = app_config
        .networks
        .iter()
        .map(|(name, entry)| SelectItem {
            value: name.clone(),
            label: name.clone(),
            hint: entry.node_endpoint.clone(),
        })
        .collect::<Vec<_>>();
    let initial = active.map(str::to_owned);
    select_or_single("Select network", &items, initial.as_ref())
}

pub(crate) fn infer_net(
    network_name: &str,
    wallet_proxy: Option<&str>,
    endpoint_label: &str,
) -> Net {
    let haystack = format!(
        "{network_name} {} {endpoint_label}",
        wallet_proxy.unwrap_or_default()
    )
    .to_ascii_lowercase();
    if haystack.contains("testnet") || haystack.contains("staging") || haystack.contains("test") {
        Net::Testnet
    } else {
        Net::Mainnet
    }
}

async fn fetch_node_genesis_hash(endpoint: v2::Endpoint, endpoint_label: &str) -> Result<String> {
    let mut client = config::connect_v2_client(endpoint)
        .await
        .with_context(|| format!("failed to connect to Concordium node at {endpoint_label}"))?;
    let consensus_info = client
        .get_consensus_info()
        .await
        .with_context(|| format!("failed to query consensus info from node at {endpoint_label}"))?;
    Ok(format!("{}", consensus_info.genesis_block))
}

fn now_unix_timestamp() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?
        .as_secs())
}

fn next_unused_u32(used: &BTreeSet<u32>) -> u32 {
    let mut candidate = 0u32;
    while used.contains(&candidate) {
        candidate += 1;
    }
    candidate
}

fn update_provider_start(
    aggregate: &Arc<Mutex<RecoveryAggregate>>,
    provider_id: u32,
    provider_name: &str,
) {
    let mut aggregate = aggregate.lock().unwrap();
    aggregate.queued_providers = aggregate.queued_providers.saturating_sub(1);
    aggregate.running_providers += 1;
    aggregate.active.insert(
        format!("provider:{provider_id}"),
        format!("- {provider_name} (id {provider_id}): starting"),
    );
}

fn update_provider_complete(aggregate: &Arc<Mutex<RecoveryAggregate>>, provider_id: u32) {
    let mut aggregate = aggregate.lock().unwrap();
    aggregate.running_providers = aggregate.running_providers.saturating_sub(1);
    aggregate.completed_providers += 1;
    aggregate.active.remove(&format!("provider:{provider_id}"));
}

fn update_provider_failed(aggregate: &Arc<Mutex<RecoveryAggregate>>, provider_id: u32) {
    let mut aggregate = aggregate.lock().unwrap();
    aggregate.running_providers = aggregate.running_providers.saturating_sub(1);
    aggregate.failed_providers += 1;
    aggregate.active.remove(&format!("provider:{provider_id}"));
}

fn set_active(aggregate: &Arc<Mutex<RecoveryAggregate>>, key: String, value: String) {
    aggregate.lock().unwrap().active.insert(key, value);
}

fn clear_active(aggregate: &Arc<Mutex<RecoveryAggregate>>, key: &str) {
    aggregate.lock().unwrap().active.remove(key);
}

fn clear_provider_active(aggregate: &Arc<Mutex<RecoveryAggregate>>, provider_id: u32) {
    let mut aggregate = aggregate.lock().unwrap();
    aggregate.active.remove(&format!("provider:{provider_id}"));
    aggregate
        .active
        .retain(|key, _| !key.starts_with(&format!("account:{provider_id}:")));
}

fn increment_identity_probes(aggregate: &Arc<Mutex<RecoveryAggregate>>) {
    aggregate.lock().unwrap().identity_probes += 1;
}

fn increment_account_probes(aggregate: &Arc<Mutex<RecoveryAggregate>>) {
    aggregate.lock().unwrap().account_probes += 1;
}

fn increment_discovered_identities(aggregate: &Arc<Mutex<RecoveryAggregate>>) {
    aggregate.lock().unwrap().discovered_identities += 1;
}

fn increment_discovered_accounts(aggregate: &Arc<Mutex<RecoveryAggregate>>) {
    aggregate.lock().unwrap().discovered_accounts += 1;
}

fn recovered_identity_label(
    conn: &Connection,
    network_genesis_hash: &str,
    seed_id: &str,
    provider_id: u32,
    identity_index: u32,
) -> Result<String> {
    if let Some(record) = identities::find_by_network_signer_owner_ip_and_index(
        conn,
        network_genesis_hash,
        seed_id,
        provider_id,
        identity_index,
    )? {
        return Ok(record.label);
    }

    let candidate = format!("id_{provider_id}-{identity_index}");
    if identities::find_by_network_and_label(conn, network_genesis_hash, &candidate)?.is_none() {
        return Ok(candidate);
    }

    identities::next_generated_label(conn, network_genesis_hash, &candidate)
}

fn recovered_account_label(
    conn: &Connection,
    network_genesis_hash: &str,
    seed_id: &str,
    provider_id: u32,
    identity_index: u32,
    credential_counter: u32,
) -> Result<String> {
    if let Some(record) = accounts::find_by_derived_tuple(
        conn,
        network_genesis_hash,
        seed_id,
        provider_id,
        identity_index,
        credential_counter,
    )? {
        return Ok(record.label);
    }

    let candidate = format!("acc_{provider_id}-{identity_index}-{credential_counter}");
    if accounts::find_by_network_and_label(conn, network_genesis_hash, &candidate)?.is_none() {
        return Ok(candidate);
    }

    accounts::next_generated_label(conn, network_genesis_hash, &candidate)
}

fn is_recoverable_account_miss(err: &QueryError) -> bool {
    matches!(err, QueryError::NotFound) || err.is_not_found()
}

fn print_recovery_summary(
    subject_kind: &str,
    subject_label: &str,
    network_name: &str,
    summary: &RecoverySummary,
) {
    println!("Recovery summary for {subject_kind} '{subject_label}' on '{network_name}':");
    println!(
        "Recovered: {} new identities • {} updated identities • {} new accounts • {} updated accounts",
        summary.inserted_identities,
        summary.updated_identities,
        summary.inserted_accounts,
        summary.updated_accounts
    );
    if !summary.skipped_providers.is_empty() {
        println!("Skipped providers:");
        for provider in &summary.skipped_providers {
            println!("- {provider}");
        }
    }
    if !summary.failed_providers.is_empty() {
        println!("Failed providers:");
        for provider in &summary.failed_providers {
            println!("- {provider}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccd_wallet_core::store::migrations;

    const VALID_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[derive(Default)]
    struct TestPrompts {
        seed_label: String,
        selected_active_seed: Option<String>,
        select_seed_calls: usize,
        selected_provider_ids: Vec<u32>,
        seed_phrase: String,
        password: String,
        password_confirmation: String,
        unlock_password: String,
        delete_confirmation: String,
    }

    impl SeedPrompts for TestPrompts {
        fn prompt_seed_label(&mut self, _prompt: &str) -> Result<String> {
            Ok(self.seed_label.clone())
        }

        fn prompt_seed_label_with_placeholder(
            &mut self,
            _prompt: &str,
            _placeholder: &str,
        ) -> Result<String> {
            Ok(self.seed_label.clone())
        }

        fn select_seed_label(
            &mut self,
            _prompt: &str,
            _items: &[SelectItem<String>],
            active: Option<&str>,
        ) -> Result<String> {
            self.select_seed_calls += 1;
            self.selected_active_seed = active.map(str::to_owned);
            Ok(self.seed_label.clone())
        }

        fn select_provider_ids(
            &mut self,
            _prompt: &str,
            _items: &[SelectItem<u32>],
            initial: &[u32],
        ) -> Result<Vec<u32>> {
            if self.selected_provider_ids.is_empty() {
                Ok(initial.to_vec())
            } else {
                Ok(self.selected_provider_ids.clone())
            }
        }

        fn prompt_seed_phrase(&mut self) -> Result<String> {
            Ok(self.seed_phrase.clone())
        }

        fn prompt_password(&mut self) -> Result<String> {
            Ok(self.password.clone())
        }

        fn prompt_password_confirmation(&mut self) -> Result<String> {
            Ok(self.password_confirmation.clone())
        }

        fn prompt_unlock_password(&mut self, _label: &str) -> Result<String> {
            Ok(self.unlock_password.clone())
        }

        fn prompt_delete_confirmation(
            &mut self,
            _label: &str,
            _identity_count: usize,
            _account_count: usize,
        ) -> Result<String> {
            Ok(self.delete_confirmation.clone())
        }
    }

    #[derive(Default)]
    struct TestRevealer {
        revealed: Vec<(String, String)>,
    }

    impl SeedPhraseRevealer for TestRevealer {
        fn reveal(&mut self, label: &str, seed_phrase: &str) -> Result<()> {
            self.revealed
                .push((label.to_owned(), seed_phrase.to_owned()));
            Ok(())
        }
    }

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    fn add_test_seed(conn: &Connection) {
        seeds::add(conn, "main_seed", VALID_MNEMONIC.as_bytes(), "password").unwrap();
    }

    #[test]
    fn normalizes_seed_phrase_whitespace() {
        assert_eq!(
            normalize_seed_phrase("  abandon\tabandon\nabout  "),
            "abandon abandon about"
        );
    }

    #[test]
    fn validates_valid_mnemonic() {
        validate_seed_phrase(VALID_MNEMONIC).unwrap();
    }

    #[test]
    fn rejects_invalid_mnemonic() {
        assert!(validate_seed_phrase("not a valid seed phrase").is_err());
    }

    #[test]
    fn validates_seed_labels() {
        for label in ["main_seed", "cold-wallet", "Seed123"] {
            validate_seed_label(label).unwrap();
        }

        for label in ["", "main seed", "main.seed", "påse"] {
            assert!(validate_seed_label(label).is_err());
        }
    }

    #[tokio::test]
    async fn password_confirmation_mismatch_does_not_write_seed() {
        let mut conn = conn();
        let mut prompts = TestPrompts {
            seed_label: String::new(),
            seed_phrase: VALID_MNEMONIC.to_owned(),
            password: "one".to_owned(),
            password_confirmation: "two".to_owned(),
            unlock_password: String::new(),
            delete_confirmation: String::new(),
            ..Default::default()
        };
        let mut revealer = TestRevealer::default();

        let err = run_with_io(
            &mut conn,
            SeedSubcommand::Add(crate::cli::SeedAddArgs {
                label: Some("main_seed".to_owned()),
                random: false,
                restore: None,
                non_interactive: false,
            }),
            &mut prompts,
            &mut revealer,
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("passwords do not match"));
        assert!(seeds::list(&conn).unwrap().is_empty());
    }

    #[tokio::test]
    async fn invalid_seed_phrase_does_not_write_seed() {
        let mut conn = conn();
        let mut prompts = TestPrompts {
            seed_label: String::new(),
            seed_phrase: "not valid".to_owned(),
            password: "password".to_owned(),
            password_confirmation: "password".to_owned(),
            unlock_password: String::new(),
            delete_confirmation: String::new(),
            ..Default::default()
        };
        let mut revealer = TestRevealer::default();

        assert!(
            run_with_io(
                &mut conn,
                SeedSubcommand::Add(crate::cli::SeedAddArgs {
                    label: Some("main_seed".to_owned()),
                    random: false,
                    restore: None,
                    non_interactive: false,
                }),
                &mut prompts,
                &mut revealer,
            )
            .await
            .is_err()
        );
        assert!(seeds::list(&conn).unwrap().is_empty());
    }

    #[tokio::test]
    async fn seed_use_without_label_prompts_with_selector() {
        let conn = conn();
        add_test_seed(&conn);
        seeds::add(&conn, "other_seed", VALID_MNEMONIC.as_bytes(), "password").unwrap();
        wallet_state::set(&conn, wallet_state::ACTIVE_SEED_KEY, "main_seed").unwrap();

        let mut prompts = TestPrompts {
            seed_label: "main_seed".to_owned(),
            ..Default::default()
        };
        use_seed(&conn, None, false, &mut prompts).await.unwrap();

        assert_eq!(prompts.select_seed_calls, 1);
        assert_eq!(prompts.selected_active_seed, Some("main_seed".to_owned()));
        assert_eq!(
            wallet_state::get(&conn, wallet_state::ACTIVE_SEED_KEY).unwrap(),
            Some("main_seed".to_owned())
        );
    }

    #[tokio::test]
    async fn seed_use_sets_active_seed() {
        let conn = conn();
        add_test_seed(&conn);

        let mut prompts = TestPrompts::default();
        use_seed(&conn, Some("main_seed".to_owned()), false, &mut prompts)
            .await
            .unwrap();

        assert_eq!(
            wallet_state::get(&conn, wallet_state::ACTIVE_SEED_KEY).unwrap(),
            Some("main_seed".to_owned())
        );
    }

    #[tokio::test]
    async fn seed_use_rejects_unknown_seed_without_writing_state() {
        let conn = conn();

        let mut prompts = TestPrompts::default();
        assert!(
            use_seed(&conn, Some("missing".to_owned()), false, &mut prompts)
                .await
                .is_err()
        );
        assert_eq!(
            wallet_state::get(&conn, wallet_state::ACTIVE_SEED_KEY).unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn seed_show_reveals_phrase_with_correct_password() {
        let conn = conn();
        add_test_seed(&conn);
        let mut prompts = TestPrompts {
            unlock_password: "password".to_owned(),
            ..Default::default()
        };
        let mut revealer = TestRevealer::default();

        show(
            &conn,
            Some("main_seed".to_owned()),
            false,
            &mut prompts,
            &mut revealer,
        )
        .await
        .unwrap();

        assert_eq!(
            revealer.revealed,
            vec![("main_seed".to_owned(), VALID_MNEMONIC.to_owned())]
        );
    }

    #[tokio::test]
    async fn seed_show_wrong_password_does_not_reveal_phrase() {
        let conn = conn();
        add_test_seed(&conn);
        let mut prompts = TestPrompts {
            unlock_password: "wrong".to_owned(),
            ..Default::default()
        };
        let mut revealer = TestRevealer::default();

        assert!(
            show(
                &conn,
                Some("main_seed".to_owned()),
                false,
                &mut prompts,
                &mut revealer,
            )
            .await
            .is_err()
        );

        assert!(revealer.revealed.is_empty());
    }

    #[tokio::test]
    async fn seed_show_without_label_uses_active_seed() {
        let conn = conn();
        add_test_seed(&conn);
        wallet_state::set(&conn, wallet_state::ACTIVE_SEED_KEY, "main_seed").unwrap();
        let mut prompts = TestPrompts {
            unlock_password: "password".to_owned(),
            ..Default::default()
        };
        let mut revealer = TestRevealer::default();

        show(&conn, None, false, &mut prompts, &mut revealer)
            .await
            .unwrap();

        assert_eq!(revealer.revealed.len(), 1);
        assert_eq!(revealer.revealed[0].1, VALID_MNEMONIC);
    }

    #[tokio::test]
    async fn seed_show_no_defaults_skips_selection_when_only_one_seed_exists() {
        let conn = conn();
        add_test_seed(&conn);
        wallet_state::set(&conn, wallet_state::ACTIVE_SEED_KEY, "main_seed").unwrap();
        let mut prompts = TestPrompts {
            unlock_password: "password".to_owned(),
            ..Default::default()
        };
        let mut revealer = TestRevealer::default();

        show(&conn, None, true, &mut prompts, &mut revealer)
            .await
            .unwrap();

        assert_eq!(prompts.select_seed_calls, 0);
        assert_eq!(prompts.selected_active_seed, None);
        assert_eq!(revealer.revealed.len(), 1);
    }

    #[tokio::test]
    async fn seed_show_no_defaults_prompts_and_preselects_active_seed() {
        let conn = conn();
        add_test_seed(&conn);
        seeds::add(&conn, "other_seed", VALID_MNEMONIC.as_bytes(), "password").unwrap();
        wallet_state::set(&conn, wallet_state::ACTIVE_SEED_KEY, "main_seed").unwrap();
        let mut prompts = TestPrompts {
            seed_label: "main_seed".to_owned(),
            unlock_password: "password".to_owned(),
            ..Default::default()
        };
        let mut revealer = TestRevealer::default();

        show(&conn, None, true, &mut prompts, &mut revealer)
            .await
            .unwrap();

        assert_eq!(prompts.selected_active_seed, Some("main_seed".to_owned()));
        assert_eq!(revealer.revealed.len(), 1);
    }

    #[tokio::test]
    async fn seed_add_prompts_for_missing_label_in_interactive_mode() {
        let mut conn = conn();
        let mut prompts = TestPrompts {
            seed_label: "prompted_seed".to_owned(),
            seed_phrase: VALID_MNEMONIC.to_owned(),
            password: "password".to_owned(),
            password_confirmation: "password".to_owned(),
            ..Default::default()
        };
        let mut revealer = TestRevealer::default();

        run_with_io(
            &mut conn,
            SeedSubcommand::Add(crate::cli::SeedAddArgs {
                label: None,
                random: false,
                restore: None,
                non_interactive: false,
            }),
            &mut prompts,
            &mut revealer,
        )
        .await
        .unwrap();

        assert!(
            seeds::find_by_label(&conn, "prompted_seed")
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn seed_add_missing_label_errors_in_non_interactive_mode() {
        let mut conn = conn();
        let mut prompts = TestPrompts::default();
        let mut revealer = TestRevealer::default();

        let err = run_with_io(
            &mut conn,
            SeedSubcommand::Add(crate::cli::SeedAddArgs {
                label: None,
                random: false,
                restore: None,
                non_interactive: true,
            }),
            &mut prompts,
            &mut revealer,
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("--non-interactive"));
    }

    #[test]
    fn missing_active_seed_is_actionable() {
        let conn = conn();

        let mut prompts = TestPrompts::default();
        let err = resolve_seed_label(&conn, None, false, &mut prompts).unwrap_err();
        assert!(err.to_string().contains("ccd-wallet seed use <LABEL>"));
    }

    #[tokio::test]
    async fn stale_active_seed_is_actionable_before_password_prompt() {
        let conn = conn();
        wallet_state::set(&conn, wallet_state::ACTIVE_SEED_KEY, "missing").unwrap();
        let mut prompts = TestPrompts::default();
        let mut revealer = TestRevealer::default();

        let err = show(&conn, None, false, &mut prompts, &mut revealer)
            .await
            .unwrap_err();

        assert!(err.to_string().contains("seed 'missing' is not configured"));
        assert!(revealer.revealed.is_empty());
    }

    #[test]
    fn generated_seed_phrase_is_valid_mnemonic() {
        let phrase = generate_seed_phrase().unwrap();
        assert_eq!(phrase.split_whitespace().count(), 24);
        validate_seed_phrase(&phrase).unwrap();
    }

    #[test]
    fn explicit_provider_filters_support_all_and_repeated_ids() {
        let available = BTreeSet::from([2u32, 7u32, 9u32]);

        assert_eq!(
            resolve_explicit_provider_ids(&available, &["all".to_owned()])
                .unwrap()
                .unwrap(),
            vec![2, 7, 9]
        );
        assert_eq!(
            resolve_explicit_provider_ids(&available, &["2".to_owned(), "7".to_owned()])
                .unwrap()
                .unwrap(),
            vec![2, 7]
        );
    }

    #[test]
    fn explicit_provider_filters_reject_invalid_combinations() {
        let available = BTreeSet::from([2u32, 7u32, 9u32]);

        let err = resolve_explicit_provider_ids(&available, &["all".to_owned(), "2".to_owned()])
            .unwrap_err();
        assert!(err.to_string().contains("cannot be combined"));

        let err = resolve_explicit_provider_ids(&available, &["999".to_owned()]).unwrap_err();
        assert!(err.to_string().contains("unavailable"));
    }

    #[test]
    fn extract_recovery_providers_skips_missing_recovery_metadata() {
        let (available, skipped) = classify_recovery_provider_metadata([
            (
                2,
                "Provider A".to_owned(),
                Some("https://issuer-a.example/recover".to_owned()),
            ),
            (7, "Provider B".to_owned(), None),
        ]);

        assert_eq!(available, vec![2]);
        assert_eq!(skipped, vec!["Provider B (id 7)".to_owned()]);
    }

    #[test]
    fn account_not_found_is_treated_as_recoverable_miss() {
        assert!(is_recoverable_account_miss(&QueryError::NotFound));
    }

    #[tokio::test]
    async fn seed_add_restore_missing_network_errors_before_writing_seed() {
        let mut conn = conn();
        let mut prompts = TestPrompts {
            seed_phrase: VALID_MNEMONIC.to_owned(),
            password: "password".to_owned(),
            password_confirmation: "password".to_owned(),
            ..Default::default()
        };
        let mut revealer = TestRevealer::default();

        let err = run_with_io(
            &mut conn,
            SeedSubcommand::Add(crate::cli::SeedAddArgs {
                label: Some("main_seed".to_owned()),
                random: false,
                restore: Some("missingnet".to_owned()),
                non_interactive: false,
            }),
            &mut prompts,
            &mut revealer,
        )
        .await
        .unwrap_err();

        assert!(err.to_string().contains("not registered"));
        assert!(seeds::find_by_label(&conn, "main_seed").unwrap().is_none());
    }

    #[test]
    fn recovered_labels_use_tuple_based_defaults() {
        let conn = conn();
        let seed = seeds::add(&conn, "main_seed", VALID_MNEMONIC.as_bytes(), "password").unwrap();

        assert_eq!(
            recovered_identity_label(&conn, "mainnet-hash", &seed.id, 7, 3).unwrap(),
            "id_7-3"
        );
        assert_eq!(
            recovered_account_label(&conn, "mainnet-hash", &seed.id, 7, 3, 2).unwrap(),
            "acc_7-3-2"
        );
    }

    #[tokio::test]
    async fn seed_add_random_generates_stores_and_reveals_phrase() {
        let mut conn = conn();
        let mut prompts = TestPrompts {
            password: "password".to_owned(),
            password_confirmation: "password".to_owned(),
            ..Default::default()
        };
        let mut revealer = TestRevealer::default();

        run_with_io(
            &mut conn,
            SeedSubcommand::Add(crate::cli::SeedAddArgs {
                label: Some("random_seed".to_owned()),
                random: true,
                restore: None,
                non_interactive: false,
            }),
            &mut prompts,
            &mut revealer,
        )
        .await
        .unwrap();

        assert_eq!(revealer.revealed.len(), 1);
        let generated = &revealer.revealed[0].1;
        assert_eq!(generated.split_whitespace().count(), 24);
        validate_seed_phrase(generated).unwrap();

        let unlocked = seeds::unlock(&conn, "random_seed", "password").unwrap();
        assert_eq!(std::str::from_utf8(&unlocked).unwrap(), generated);
    }

    #[tokio::test]
    async fn seed_add_random_rejects_duplicate_before_revealing() {
        let mut conn = conn();
        add_test_seed(&conn);
        let mut prompts = TestPrompts {
            password: "password".to_owned(),
            password_confirmation: "password".to_owned(),
            ..Default::default()
        };
        let mut revealer = TestRevealer::default();

        assert!(
            run_with_io(
                &mut conn,
                SeedSubcommand::Add(crate::cli::SeedAddArgs {
                    label: Some("main_seed".to_owned()),
                    random: true,
                    restore: None,
                    non_interactive: false,
                }),
                &mut prompts,
                &mut revealer,
            )
            .await
            .is_err()
        );

        assert!(revealer.revealed.is_empty());
    }

    #[tokio::test]
    async fn seed_delete_deletes_seed_and_clears_active_seed() {
        let conn = conn();
        add_test_seed(&conn);
        wallet_state::set(&conn, wallet_state::ACTIVE_SEED_KEY, "main_seed").unwrap();
        let mut prompts = TestPrompts {
            delete_confirmation: "main_seed".to_owned(),
            ..Default::default()
        };

        delete_seed(&conn, Some("main_seed".to_owned()), false, &mut prompts)
            .await
            .unwrap();

        assert!(seeds::find_by_label(&conn, "main_seed").unwrap().is_none());
        assert_eq!(
            wallet_state::get(&conn, wallet_state::ACTIVE_SEED_KEY).unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn seed_delete_confirmation_mismatch_keeps_seed_and_vault() {
        let conn = conn();
        add_test_seed(&conn);
        let mut prompts = TestPrompts {
            delete_confirmation: "wrong".to_owned(),
            ..Default::default()
        };

        assert!(
            delete_seed(&conn, Some("main_seed".to_owned()), false, &mut prompts)
                .await
                .is_err()
        );

        assert!(seeds::find_by_label(&conn, "main_seed").unwrap().is_some());
        let vault_count: u32 = conn
            .query_row("SELECT COUNT(*) FROM signer_owner_vaults", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(vault_count, 1);
    }

    #[tokio::test]
    async fn seed_delete_inactive_seed_leaves_active_seed_unchanged() {
        let conn = conn();
        add_test_seed(&conn);
        seeds::add(&conn, "old_seed", VALID_MNEMONIC.as_bytes(), "password").unwrap();
        wallet_state::set(&conn, wallet_state::ACTIVE_SEED_KEY, "main_seed").unwrap();
        let mut prompts = TestPrompts {
            delete_confirmation: "old_seed".to_owned(),
            ..Default::default()
        };

        delete_seed(&conn, Some("old_seed".to_owned()), false, &mut prompts)
            .await
            .unwrap();

        assert_eq!(
            wallet_state::get(&conn, wallet_state::ACTIVE_SEED_KEY).unwrap(),
            Some("main_seed".to_owned())
        );
    }

    #[tokio::test]
    async fn seed_rename_updates_active_seed() {
        let conn = conn();
        add_test_seed(&conn);
        wallet_state::set(&conn, wallet_state::ACTIVE_SEED_KEY, "main_seed").unwrap();
        let mut prompts = TestPrompts::default();

        rename_seed(
            &conn,
            Some("main_seed".to_owned()),
            Some("daily".to_owned()),
            false,
            &mut prompts,
        )
        .await
        .unwrap();

        assert_eq!(
            wallet_state::get(&conn, wallet_state::ACTIVE_SEED_KEY).unwrap(),
            Some("daily".to_owned())
        );
        assert!(seeds::find_by_label(&conn, "daily").unwrap().is_some());
    }

    #[test]
    fn render_seed_list_text_marks_active_seed() {
        assert_eq!(
            render_seed_list_text("main_seed", true, 2, 3),
            "main_seed — 2 identities • 3 accounts • active"
        );
        assert_eq!(
            render_seed_list_text("other_seed", false, 1, 0),
            "other_seed — 1 identity • 0 accounts"
        );
        assert_eq!(
            render_seed_selector_text("main_seed", 2, 3),
            "main_seed — 2 identities • 3 accounts"
        );
    }

    #[test]
    fn format_count_handles_singular_and_plural() {
        assert_eq!(format_count(1, "account", "accounts"), "1 account");
        assert_eq!(format_count(2, "account", "accounts"), "2 accounts");
    }

    #[test]
    fn reveal_inner_waits_for_timeout_and_prints_phrase() {
        let term = Term::stdout();
        reveal_seed_phrase_inner(&term, "main_seed", VALID_MNEMONIC, Duration::from_millis(1))
            .unwrap();
    }
}
