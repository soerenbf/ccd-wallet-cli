use crate::{
    cli::{
        AccountImportGenesisArgs, AccountImportSubcommand, AccountListArgs, AccountNewArgs,
        AccountRenameArgs, AccountSubcommand,
    },
    commands::ui::{
        ContextLine, FuzzySelectItem, ResolutionSource, SelectItem, fuzzy_select_or_single,
        log_resolved_context, select_or_single,
    },
};
use anyhow::{Context, Result, bail};
use ccd_wallet_core::{
    account_creation::{
        CredentialDeploymentInput, build_credential_deployment, credential_counter_to_u8,
        parse_identity_object,
    },
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
    id::{
        constants::{ArCurve, IpPairing},
        types::{ArIdentity, ArInfo, IpInfo},
    },
    types::{
        BlockItemSummaryDetails,
        transactions::{BlockItem, Payload},
    },
    v2,
};
use futures_util::StreamExt;
use rusqlite::Connection;
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

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
        AccountSubcommand::Import(command) => match command.command {
            AccountImportSubcommand::Genesis(args) => import_genesis(conn, args).await,
        },
        AccountSubcommand::List(args) => list_accounts(conn, args).await,
        AccountSubcommand::New(args) => new(conn, *args).await,
        AccountSubcommand::Rename(args) => rename_account(conn, args).await,
    }
}

async fn list_accounts(conn: &mut Connection, args: AccountListArgs) -> Result<()> {
    let seed_scope = resolve_seed_scope(
        conn,
        args.seed.as_deref(),
        args.non_interactive,
        args.no_defaults,
        true,
    )?;
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
            )
        );
    }
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
                args.show_addresses,
                seed_scope.as_ref(),
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

fn resolve_seed_scope(
    conn: &Connection,
    explicit: Option<&str>,
    non_interactive: bool,
    no_defaults: bool,
    allow_all: bool,
) -> Result<(ScopeSelection, ResolutionSource)> {
    match explicit {
        Some("all") if allow_all => Ok((ScopeSelection::All, ResolutionSource::Explicit)),
        Some(label) => seeds::find_by_label(conn, label)?
            .map(|seed| (ScopeSelection::One(seed.label), ResolutionSource::Explicit))
            .with_context(|| format!("seed '{}' is not configured", label)),
        None => {
            if allow_all && seeds::list(conn)?.is_empty() {
                return Ok((ScopeSelection::All, ResolutionSource::Inferred));
            }
            let active = wallet_state::get(conn, wallet_state::ACTIVE_SEED_KEY)?;
            if no_defaults {
                return Ok((
                    prompt_for_seed_scope(conn, active.as_deref(), allow_all)?,
                    ResolutionSource::Prompted,
                ));
            }
            match active {
                Some(label) => Ok((ScopeSelection::One(label), ResolutionSource::ActiveDefault)),
                None if non_interactive => bail!(
                    "No active seed. Run `ccd-wallet seed use <LABEL>` or supply `--seed <LABEL>`."
                ),
                None => Ok((
                    prompt_for_seed_scope(conn, None, allow_all)?,
                    ResolutionSource::Prompted,
                )),
            }
        }
    }
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

fn resolve_seed_scope_for_addresses(
    conn: &Connection,
    explicit: Option<&str>,
    non_interactive: bool,
) -> Result<ScopeSelection> {
    match explicit {
        Some(label) => seeds::find_by_label(conn, label)?
            .map(|seed| ScopeSelection::One(seed.label))
            .with_context(|| format!("seed '{}' is not configured", label)),
        None if non_interactive => {
            bail!("`--seed <LABEL>` is required with `--show-addresses` in --non-interactive mode")
        }
        None => prompt_for_seed_scope(
            conn,
            wallet_state::get(conn, wallet_state::ACTIVE_SEED_KEY)?.as_deref(),
            false,
        ),
    }
}

fn prompt_for_seed_scope(
    conn: &Connection,
    active: Option<&str>,
    allow_all: bool,
) -> Result<ScopeSelection> {
    let seeds = seeds::list(conn)?;
    if seeds.is_empty() {
        if allow_all {
            return Ok(ScopeSelection::All);
        }
        bail!("no seeds are configured; run `ccd-wallet seed add <LABEL>` first")
    }
    let mut items = Vec::new();
    if allow_all {
        items.push(SelectItem {
            value: ScopeSelection::All,
            label: "All seeds".to_owned(),
            hint: String::new(),
        });
    }
    items.extend(seeds.into_iter().map(|seed| SelectItem {
        value: ScopeSelection::One(seed.label.clone()),
        label: seed.label,
        hint: String::new(),
    }));
    let initial = active.map(|value| ScopeSelection::One(value.to_owned()));
    select_or_single("Select seed", &items, initial.as_ref())
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
        let password: String =
            password(format!("Password for seed '{}': ", seed_label)).interact()?;
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
) -> String {
    let prefix = match record.status {
        accounts::AccountStatus::Pending => "[pending] ",
        accounts::AccountStatus::Finalized => "",
    };
    let label = match address {
        Some(address) => format!("{}{} ({address})", prefix, record.label),
        None => format!("{}{}", prefix, record.label),
    };
    if record.source_kind == accounts::AccountSourceKind::Imported {
        return format!("{} — {} • imported", label, network_name);
    }
    format!(
        "{} — {} • seed:{} • provider:{} • identity:{} • cred:{}",
        label,
        network_name,
        seed_label,
        record.ip_identity,
        record.identity_index,
        record.credential_counter
    )
}

fn select_account_fuzzy(
    conn: &Connection,
    candidates: Vec<accounts::AccountRecord>,
    seeds_by_id: &BTreeMap<String, String>,
    networks_by_hash: &BTreeMap<String, String>,
    show_addresses: bool,
    seed_scope: Option<&ScopeSelection>,
) -> Result<accounts::AccountRecord> {
    if candidates.is_empty() {
        bail!("no matching accounts are available")
    }
    let addresses = if show_addresses {
        let concrete_seed_scope = seed_scope
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
                ),
            }
        })
        .collect::<Vec<_>>();
    let id = fuzzy_select_or_single("Select account", &items)?;
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
            show_addresses,
            seed_scope,
        )
    }
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

async fn resolve_account_network_context(
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
    fn render_account_fuzzy_text_uses_conditional_badges() {
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

        assert!(
            render_account_fuzzy_text(&pending, "test", "testnet", None)
                .starts_with("[pending] pending-account")
        );
        assert!(
            render_account_fuzzy_text(&finalized, "test", "testnet", None)
                .starts_with("finalized-account")
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
            render_account_fuzzy_text(&imported, "", "local", None),
            "baker-0 — local • imported"
        );
    }

    #[test]
    fn show_addresses_requires_seed_in_non_interactive_mode() {
        let conn = conn();
        let err = resolve_seed_scope_for_addresses(&conn, None, true).unwrap_err();
        assert!(err.to_string().contains("--seed <LABEL>"));
    }
}
