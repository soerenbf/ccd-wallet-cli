mod new;

use crate::{
    cli::{
        IdentityExportArgs, IdentityListArgs, IdentityRenameArgs, IdentityShowArgs,
        IdentitySubcommand,
    },
    commands::{
        input::{Defaultable, InputMode, Promptable},
        ui::{
            ContextLine, FuzzySelectItem, ResolutionSource, SelectItem, fuzzy_select_or_single,
            log_resolved_context, select_or_single,
        },
    },
};
use anyhow::{Context, Result, bail};
use ccd_wallet_core::store::{
    config::{AppConfig, load},
    identities::{self, IdentityPrivatePayload, IdentityRecord, IdentityStatus},
    seeds,
    signer_owners::{self, SignerOwnerKind, SignerOwnerRecord},
    wallet_state,
};
use chrono::{DateTime, SecondsFormat, Utc};
use cliclack::{input, password};
use console::Term;
use rusqlite::Connection;
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const IDENTITY_REVEAL_TIMEOUT: Duration = Duration::from_secs(30);

/// Enter the terminal alternate screen buffer (saves normal screen, shows blank buffer).
const ENTER_ALT_SCREEN: &str = "\x1b[?1049h";
/// Leave the terminal alternate screen buffer (restores normal screen).
const LEAVE_ALT_SCREEN: &str = "\x1b[?1049l";

#[derive(Clone, Debug, Eq, PartialEq)]
enum ScopeSelection {
    All,
    One(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IdentityListStatus {
    Pending,
    Done,
    Expired,
}

impl IdentityListStatus {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "done" => Ok(Self::Done),
            "expired" => Ok(Self::Expired),
            other => bail!("unsupported identity status '{other}'"),
        }
    }
}

pub async fn run(conn: &mut Connection, command: IdentitySubcommand) -> Result<()> {
    match command {
        IdentitySubcommand::Export(args) => export_identity(conn, args).await,
        IdentitySubcommand::List(args) => list_identities(conn, args).await,
        IdentitySubcommand::New(args) => new::run(conn, *args).await,
        IdentitySubcommand::Show(args) => show_identity(conn, args).await,
        IdentitySubcommand::Rename(args) => rename_identity(conn, args).await,
    }
}

async fn list_identities(conn: &Connection, args: IdentityListArgs) -> Result<()> {
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
        .map(IdentityListStatus::parse)
        .transpose()?;

    log_scope_context(&seed_scope, &network_scope)?;

    let seeds_by_id = seed_labels_by_id(conn)?;
    let key_source_tags_by_id = key_source_tags_by_id(conn)?;
    let networks_by_hash = network_names_by_genesis_hash()?;
    let now = now_unix_seconds()?;
    let mut identities = identities::list(conn)?
        .into_iter()
        .filter(|record| matches_seed_scope(record, &seed_scope, &seeds_by_id))
        .filter(|record| matches_network_scope(record, &network_scope, &networks_by_hash))
        .filter(|record| {
            args.provider
                .is_none_or(|provider| record.ip_identity == provider)
        })
        .filter(|record| matches_identity_status(record, status_filter, now))
        .collect::<Vec<_>>();
    identities.sort_by(|a, b| a.label.cmp(&b.label));

    for record in identities {
        let seed_label = key_source_tags_by_id
            .get(&record.signer_owner_id)
            .cloned()
            .unwrap_or_else(|| "unknown-key-source".to_owned());
        let network_name = networks_by_hash
            .get(&record.network_genesis_hash)
            .cloned()
            .unwrap_or_else(|| record.network_genesis_hash.clone());
        println!(
            "{}",
            render_identity_fuzzy_text(&record, &seed_label, &network_name, now)
        );
    }
    Ok(())
}

async fn rename_identity(conn: &Connection, args: IdentityRenameArgs) -> Result<()> {
    let seeds_by_id = key_source_tags_by_id(conn)?;
    let networks_by_hash = network_names_by_genesis_hash()?;
    let now = now_unix_seconds()?;

    let record = match args.old_label.as_deref() {
        Some(old_label) => resolve_identity_by_label(
            conn,
            old_label,
            &seeds_by_id,
            &networks_by_hash,
            now,
            args.non_interactive,
        )?,
        None => Promptable::Missing {
            value_name: "identity label",
        }
        .resolve_with(InputMode::from_flags(args.non_interactive, false), || {
            select_identity_fuzzy(
                identities::list(conn)?,
                &seeds_by_id,
                &networks_by_hash,
                now,
            )
        })?
        .into_value(),
    };

    let new_label = Promptable::from_option(args.new_label, "new identity label")
        .resolve_with(InputMode::from_flags(args.non_interactive, false), || {
            prompt_identity_label_with_placeholder("New identity label:", &record.label)
        })?
        .into_value();
    validate_label("identity", &new_label)?;
    identities::rename(conn, record.id, &new_label)?;
    println!("Identity '{}' renamed to '{}'.", record.label, new_label);
    Ok(())
}

async fn show_identity(conn: &Connection, args: IdentityShowArgs) -> Result<()> {
    let unlocked = unlock_identity_payload_for_show(conn, args)?;
    let text = render_identity_reveal_text(&unlocked.view);
    reveal_identity_until_key_or_timeout(&text, IDENTITY_REVEAL_TIMEOUT)
}

async fn export_identity(conn: &Connection, args: IdentityExportArgs) -> Result<()> {
    let unlocked = unlock_identity_payload(conn, &args.label, false)?;
    let output_path = resolve_export_output_path(args.output, &unlocked.view.identity.label)?;
    let json = serde_json::to_string_pretty(&unlocked.view)
        .context("failed to serialise identity export")?;
    write_export_file(&output_path, &json)?;
    println!(
        "Exported identity '{}' to {}.",
        unlocked.view.identity.label,
        output_path.display()
    );
    Ok(())
}

struct UnlockedIdentityView {
    view: IdentityExportView,
}

fn unlock_identity_payload_for_show(
    conn: &Connection,
    args: IdentityShowArgs,
) -> Result<UnlockedIdentityView> {
    let key_source_tags_by_id = key_source_tags_by_id(conn)?;
    let networks_by_hash = network_names_by_genesis_hash()?;
    let now = now_unix_seconds()?;
    let record = resolve_identity_for_show(
        conn,
        args.label.as_deref(),
        args.network.as_deref(),
        &key_source_tags_by_id,
        &networks_by_hash,
        now,
    )?;
    unlock_identity_payload_record(conn, record, &networks_by_hash)
}

fn unlock_identity_payload(
    conn: &Connection,
    label: &str,
    non_interactive_resolution: bool,
) -> Result<UnlockedIdentityView> {
    let key_source_tags_by_id = key_source_tags_by_id(conn)?;
    let networks_by_hash = network_names_by_genesis_hash()?;
    let now = now_unix_seconds()?;
    let record = resolve_identity_by_label(
        conn,
        label,
        &key_source_tags_by_id,
        &networks_by_hash,
        now,
        non_interactive_resolution,
    )?;
    unlock_identity_payload_record(conn, record, &networks_by_hash)
}

fn unlock_identity_payload_record(
    conn: &Connection,
    record: IdentityRecord,
    networks_by_hash: &BTreeMap<String, String>,
) -> Result<UnlockedIdentityView> {
    let owner = signer_owners::find_by_id(conn, &record.signer_owner_id)?.with_context(|| {
        format!(
            "key source for identity '{}' is not configured",
            record.label
        )
    })?;
    let key_source = KeySourceContext::from_owner(&owner);
    let password = password(format!("Password for key source '{}':", owner.label))
        .allow_empty()
        .interact()?;
    let unlocked = signer_owners::unlock_by_id(conn, &record.signer_owner_id, &password)
        .with_context(|| format!("failed to unlock key source '{}'", owner.label))?;
    let payload = identities::decrypt_private_payload(conn, record.id, &unlocked.dek)?;
    let network = NetworkContext::from_record(&record, networks_by_hash);
    let view = IdentityExportView::new(record, network, key_source, payload)?;
    Ok(UnlockedIdentityView { view })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdentityExportView {
    version: u32,
    identity: IdentityExportMetadata,
    network: NetworkContext,
    key_source: KeySourceContext,
    private_payload: PrivatePayloadView,
}

impl IdentityExportView {
    fn new(
        record: IdentityRecord,
        network: NetworkContext,
        key_source: KeySourceContext,
        payload: IdentityPrivatePayload,
    ) -> Result<Self> {
        Ok(Self {
            version: 1,
            identity: IdentityExportMetadata::from_record(&record)?,
            network,
            key_source,
            private_payload: PrivatePayloadView {
                code_uri: payload.code_uri().map(ToOwned::to_owned),
                identity_object: payload.identity_object().cloned(),
            },
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdentityExportMetadata {
    label: String,
    status: String,
    provider: u32,
    identity_index: u32,
    created_at: String,
    expires_at: Option<String>,
}

impl IdentityExportMetadata {
    fn from_record(record: &IdentityRecord) -> Result<Self> {
        Ok(Self {
            label: record.label.clone(),
            status: identity_status_text(record.status).to_owned(),
            provider: record.ip_identity,
            identity_index: record.identity_index,
            created_at: format_timestamp_rfc3339(record.created_at)?,
            expires_at: record
                .expires_at
                .map(format_timestamp_rfc3339)
                .transpose()?,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NetworkContext {
    label: String,
    genesis_hash: String,
}

impl NetworkContext {
    fn from_record(record: &IdentityRecord, networks_by_hash: &BTreeMap<String, String>) -> Self {
        Self {
            label: networks_by_hash
                .get(&record.network_genesis_hash)
                .cloned()
                .unwrap_or_else(|| "unknown".to_owned()),
            genesis_hash: record.network_genesis_hash.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct KeySourceContext {
    kind: String,
    label: String,
}

impl KeySourceContext {
    fn from_owner(owner: &SignerOwnerRecord) -> Self {
        Self {
            kind: owner.kind.as_str().to_owned(),
            label: owner.label.clone(),
        }
    }

    fn display(&self) -> String {
        format!("{}:{}", self.kind, self.label)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PrivatePayloadView {
    code_uri: Option<String>,
    identity_object: Option<Value>,
}

fn render_identity_reveal_text(view: &IdentityExportView) -> String {
    let mut lines = vec![
        "Identity".to_owned(),
        format!("  Label:       {}", view.identity.label),
        format!("  Status:      {}", view.identity.status),
        format!("  Network:     {}", view.network.label),
        format!("  Key source:  {}", view.key_source.display()),
        format!("  Provider:    {}", view.identity.provider),
        format!("  Index:       {}", view.identity.identity_index),
        format!("  Created:     {}", view.identity.created_at),
    ];
    if let Some(expires_at) = &view.identity.expires_at {
        lines.push(format!("  Expires:     {expires_at}"));
    }
    if let Some(code_uri) = &view.private_payload.code_uri {
        lines.extend([
            String::new(),
            "Private payload".to_owned(),
            format!("  Code URI:    {code_uri}"),
        ]);
    }
    lines.extend([String::new(), "Issued identity object".to_owned()]);
    match &view.private_payload.identity_object {
        Some(identity_object) => {
            let flattened = flatten_identity_object_for_reveal(identity_object);
            if flattened.is_empty() {
                lines.push("  <empty>".to_owned());
            } else {
                lines.extend(flattened.into_iter().map(|(path, value)| {
                    format!("  {}: {value}", identity_reveal_display_key(&path))
                }));
            }
        }
        None => lines.push("  <not available yet>".to_owned()),
    }
    lines.extend([
        String::new(),
        "Press any key to hide. It will hide automatically in 30 seconds.".to_owned(),
        format!(
            "Use `ccd-wallet identity export {} --out <FILE>` to persist this identity as JSON.",
            view.identity.label
        ),
    ]);
    lines.join("\n")
}

fn flatten_identity_object_for_reveal(value: &Value) -> Vec<(String, String)> {
    let mut lines = Vec::new();
    flatten_identity_object_for_reveal_inner(value, "", &mut lines);
    lines
}

fn flatten_identity_object_for_reveal_inner(
    value: &Value,
    path: &str,
    lines: &mut Vec<(String, String)>,
) {
    match value {
        Value::Object(map) if map.is_empty() => {
            if is_allowed_identity_reveal_path(path) {
                lines.push((path_or_root(path), "{}".to_owned()));
            }
        }
        Value::Object(map) => {
            let mut entries = map.iter().collect::<Vec<_>>();
            entries.sort_by_key(|(key, _)| *key);
            for (key, child) in entries {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                if identity_reveal_path_can_contain_visible_value(&child_path) {
                    flatten_identity_object_for_reveal_inner(child, &child_path, lines);
                }
            }
        }
        Value::Array(values) if values.is_empty() => {
            if is_allowed_identity_reveal_path(path) {
                lines.push((path_or_root(path), "[]".to_owned()));
            }
        }
        Value::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                let child_path = format!("{path}[{index}]");
                if identity_reveal_path_can_contain_visible_value(&child_path) {
                    flatten_identity_object_for_reveal_inner(child, &child_path, lines);
                }
            }
        }
        Value::String(text) if is_allowed_identity_reveal_path(path) => {
            lines.push((path_or_root(path), text.clone()))
        }
        Value::Number(number) if is_allowed_identity_reveal_path(path) => {
            lines.push((path_or_root(path), number.to_string()))
        }
        Value::Bool(value) if is_allowed_identity_reveal_path(path) => {
            lines.push((path_or_root(path), value.to_string()))
        }
        Value::Null if is_allowed_identity_reveal_path(path) => {
            lines.push((path_or_root(path), "null".to_owned()))
        }
        Value::String(_) | Value::Number(_) | Value::Bool(_) | Value::Null => {}
    }
}

fn identity_reveal_path_can_contain_visible_value(path: &str) -> bool {
    is_allowed_identity_reveal_path(path)
        || ALLOWED_IDENTITY_REVEAL_PATHS
            .iter()
            .any(|allowed| allowed.starts_with(&format!("{path}.")))
        || path == "value"
        || path == "value.attributeList"
}

fn is_allowed_identity_reveal_path(path: &str) -> bool {
    path.starts_with("value.attributeList.chosenAttributes.")
        || ALLOWED_IDENTITY_REVEAL_PATHS.contains(&path)
}

const ALLOWED_IDENTITY_REVEAL_PATHS: &[&str] = &[
    "value.attributeList.chosenAttributes",
    "value.attributeList.createdAt",
    "value.attributeList.maxAccounts",
    "value.attributeList.validTo",
];

fn identity_reveal_display_key(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or(path)
}

fn path_or_root(path: &str) -> String {
    if path.is_empty() {
        "$".to_owned()
    } else {
        path.to_owned()
    }
}

fn reveal_identity_until_key_or_timeout(text: &str, timeout: Duration) -> Result<()> {
    let term = Term::stdout();
    print!("{ENTER_ALT_SCREEN}");
    term.clear_screen()?;
    term.move_cursor_to(0, 0)?;

    let result = reveal_identity_inner(&term, text, timeout);

    term.clear_screen()?;
    term.move_cursor_to(0, 0)?;
    print!("{LEAVE_ALT_SCREEN}");

    result
}

fn reveal_identity_inner(term: &Term, text: &str, timeout: Duration) -> Result<()> {
    term.write_line(text)?;
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

fn prompt_identity_label_with_placeholder(prompt: &str, placeholder: &str) -> Result<String> {
    let label: String = input(prompt)
        .placeholder(placeholder)
        .validate(|value: &String| {
            validate_label("identity", value).map_err(|error| error.to_string())
        })
        .interact()?;
    Ok(label)
}

fn resolve_export_output_path(explicit: Option<PathBuf>, identity_label: &str) -> Result<PathBuf> {
    match explicit {
        Some(path) => expand_tilde_path(&path),
        None => {
            let suggested = format!("{identity_label}.json");
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

fn seed_labels_by_id(conn: &Connection) -> Result<BTreeMap<String, String>> {
    Ok(signer_owners::list(conn)?
        .into_iter()
        .map(|owner| (owner.id, owner.label))
        .collect())
}

fn key_source_tags_by_id(conn: &Connection) -> Result<BTreeMap<String, String>> {
    Ok(signer_owners::list(conn)?
        .into_iter()
        .map(|owner| {
            let prefix = match owner.kind {
                SignerOwnerKind::Seed => "seed",
                SignerOwnerKind::Ledger => "ledger",
            };
            (owner.id, format!("{prefix}:{}", owner.label))
        })
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
            let active = wallet_state::get(conn, wallet_state::ACTIVE_SEED_KEY)?;
            if no_defaults {
                return Ok((
                    prompt_for_seed_scope(conn, active.as_deref(), allow_all)?,
                    ResolutionSource::Prompted,
                ));
            }
            Defaultable::Missing {
                value_name: "seed",
            }
                .resolve_with_default_or_prompt(
                    InputMode::from_flags(non_interactive, false),
                    || Ok(active.map(ScopeSelection::One)),
                    || prompt_for_seed_scope(conn, None, allow_all),
                )
                .map(|resolved| {
                    let source = match resolved.source {
                        crate::commands::input::ResolvedSource::Default => {
                            ResolutionSource::ActiveDefault
                        }
                        crate::commands::input::ResolvedSource::Prompt => ResolutionSource::Prompted,
                        crate::commands::input::ResolvedSource::Explicit => ResolutionSource::Explicit,
                    };
                    (resolved.value, source)
                })
                .with_context(
                    || "No active seed. Run `ccd-wallet seed use <LABEL>` or supply `--seed <LABEL>`.",
                )
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
            Defaultable::Missing {
                value_name: "network",
            }
                .resolve_with_default_or_prompt(
                    InputMode::from_flags(non_interactive, false),
                    || Ok(active.map(ScopeSelection::One)),
                    || prompt_for_network_scope(&app_config, None, allow_all),
                )
                .map(|resolved| {
                    let source = match resolved.source {
                        crate::commands::input::ResolvedSource::Default => {
                            ResolutionSource::ActiveDefault
                        }
                        crate::commands::input::ResolvedSource::Prompt => ResolutionSource::Prompted,
                        crate::commands::input::ResolvedSource::Explicit => ResolutionSource::Explicit,
                    };
                    (resolved.value, source)
                })
                .with_context(
                    || "no active network is set; provide `--network` or run `ccd-wallet network use <NAME>`",
                )
        }
    }
}

fn prompt_for_seed_scope(
    conn: &Connection,
    active: Option<&str>,
    allow_all: bool,
) -> Result<ScopeSelection> {
    let seeds = seeds::list(conn)?;
    if seeds.is_empty() {
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
            label: "key source:",
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
    record: &IdentityRecord,
    scope: &(ScopeSelection, ResolutionSource),
    labels: &BTreeMap<String, String>,
) -> bool {
    match &scope.0 {
        ScopeSelection::All => true,
        ScopeSelection::One(label) => labels.get(&record.signer_owner_id) == Some(label),
    }
}

fn matches_network_scope(
    record: &IdentityRecord,
    scope: &(ScopeSelection, ResolutionSource),
    names: &BTreeMap<String, String>,
) -> bool {
    match &scope.0 {
        ScopeSelection::All => true,
        ScopeSelection::One(name) => names.get(&record.network_genesis_hash) == Some(name),
    }
}

fn identity_status_text(status: IdentityStatus) -> &'static str {
    match status {
        IdentityStatus::Pending => "pending",
        IdentityStatus::Done => "done",
    }
}

fn effective_identity_status(record: &IdentityRecord, now: i64) -> IdentityListStatus {
    if record.status == IdentityStatus::Pending {
        IdentityListStatus::Pending
    } else if record
        .expires_at
        .is_some_and(|expires_at| expires_at <= now)
    {
        IdentityListStatus::Expired
    } else {
        IdentityListStatus::Done
    }
}

fn matches_identity_status(
    record: &IdentityRecord,
    status: Option<IdentityListStatus>,
    now: i64,
) -> bool {
    match status {
        None => true,
        Some(filter) => effective_identity_status(record, now) == filter,
    }
}

fn render_identity_fuzzy_text(
    record: &IdentityRecord,
    seed_label: &str,
    network_name: &str,
    now: i64,
) -> String {
    let prefix = match effective_identity_status(record, now) {
        IdentityListStatus::Pending => "[pending] ",
        IdentityListStatus::Expired => "[expired] ",
        IdentityListStatus::Done => "",
    };
    let mut text = format!(
        "{}{} — {} • key source:{} • provider:{} • idx:{}",
        prefix, record.label, network_name, seed_label, record.ip_identity, record.identity_index
    );
    if let Some(expires_at) = record.expires_at {
        text.push_str(&format!(" • exp:{}", format_expiry(expires_at)));
    }
    text
}

fn resolve_identity_for_show(
    conn: &Connection,
    label: Option<&str>,
    network: Option<&str>,
    seeds_by_id: &BTreeMap<String, String>,
    networks_by_hash: &BTreeMap<String, String>,
    now: i64,
) -> Result<IdentityRecord> {
    let network_genesis_hash = match network {
        Some(name) => Some(resolve_network_genesis_hash(name)?),
        None => None,
    };
    let candidates = show_identity_candidates(conn, label, network_genesis_hash.as_deref())?;
    if label.is_some() {
        choose_identity_match(candidates, seeds_by_id, networks_by_hash, now, false)
    } else {
        select_identity_fuzzy(candidates, seeds_by_id, networks_by_hash, now)
    }
}

fn resolve_network_genesis_hash(name: &str) -> Result<String> {
    load()?
        .networks
        .get(name)
        .map(|entry| entry.genesis_hash.clone())
        .with_context(|| format!("network '{name}' is not registered"))
}

fn show_identity_candidates(
    conn: &Connection,
    label: Option<&str>,
    network_genesis_hash: Option<&str>,
) -> Result<Vec<IdentityRecord>> {
    Ok(identities::list(conn)?
        .into_iter()
        .filter(|record| label.is_none_or(|label| record.label == label))
        .filter(|record| {
            network_genesis_hash.is_none_or(|hash| record.network_genesis_hash == hash)
        })
        .collect())
}

fn resolve_identity_by_label(
    conn: &Connection,
    label: &str,
    seeds_by_id: &BTreeMap<String, String>,
    networks_by_hash: &BTreeMap<String, String>,
    now: i64,
    non_interactive: bool,
) -> Result<IdentityRecord> {
    let matches = show_identity_candidates(conn, Some(label), None)?;
    choose_identity_match(matches, seeds_by_id, networks_by_hash, now, non_interactive)
}

fn choose_identity_match(
    matches: Vec<IdentityRecord>,
    seeds_by_id: &BTreeMap<String, String>,
    networks_by_hash: &BTreeMap<String, String>,
    now: i64,
    non_interactive: bool,
) -> Result<IdentityRecord> {
    if matches.is_empty() {
        bail!("identity is not configured")
    } else if matches.len() == 1 {
        Ok(matches.into_iter().next().unwrap())
    } else if non_interactive {
        bail!("identity label is ambiguous across multiple networks; rerun interactively")
    } else {
        select_identity_fuzzy(matches, seeds_by_id, networks_by_hash, now)
    }
}

fn select_identity_fuzzy(
    candidates: Vec<IdentityRecord>,
    seeds_by_id: &BTreeMap<String, String>,
    networks_by_hash: &BTreeMap<String, String>,
    now: i64,
) -> Result<IdentityRecord> {
    if candidates.is_empty() {
        bail!("no matching identities are available")
    }
    let items = candidates
        .iter()
        .map(|record| {
            let seed_label = seeds_by_id
                .get(&record.signer_owner_id)
                .cloned()
                .unwrap_or_else(|| "<unknown-seed>".to_owned());
            let network_name = networks_by_hash
                .get(&record.network_genesis_hash)
                .cloned()
                .unwrap_or_else(|| record.network_genesis_hash.clone());
            FuzzySelectItem {
                value: record.id,
                text: render_identity_fuzzy_text(record, &seed_label, &network_name, now),
            }
        })
        .collect::<Vec<_>>();
    let id = fuzzy_select_or_single("Select identity", &items)?;
    candidates
        .into_iter()
        .find(|record| record.id == id)
        .context("selected identity was not found")
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

fn format_expiry(expires_at: i64) -> String {
    DateTime::<Utc>::from_timestamp(expires_at, 0)
        .map(|value| value.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| expires_at.to_string())
}

fn format_timestamp_rfc3339(timestamp: i64) -> Result<String> {
    DateTime::<Utc>::from_timestamp(timestamp, 0)
        .map(|value| value.to_rfc3339_opts(SecondsFormat::Secs, true))
        .with_context(|| format!("timestamp {timestamp} is out of range"))
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
    use serde_json::json;

    fn identity(status: IdentityStatus, expires_at: Option<i64>) -> IdentityRecord {
        IdentityRecord {
            id: 1,
            signer_owner_id: "seed-id".to_owned(),
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
    fn effective_status_reports_expired() {
        assert_eq!(
            effective_identity_status(&identity(IdentityStatus::Done, Some(100)), 100),
            IdentityListStatus::Expired
        );
    }

    #[test]
    fn render_identity_fuzzy_text_uses_conditional_badges() {
        let pending = render_identity_fuzzy_text(
            &identity(IdentityStatus::Pending, None),
            "test",
            "testnet",
            100,
        );
        assert!(pending.starts_with("[pending] identity"));

        let done = render_identity_fuzzy_text(
            &identity(IdentityStatus::Done, Some(200)),
            "test",
            "testnet",
            100,
        );
        assert!(done.starts_with("identity — testnet"));

        let expired = render_identity_fuzzy_text(
            &identity(IdentityStatus::Done, Some(1_811_808_000)),
            "test",
            "testnet",
            1_811_808_000,
        );
        assert!(expired.starts_with("[expired] identity"));
        assert!(expired.contains("exp:2027-06-01"));

        let ledger = render_identity_fuzzy_text(
            &identity(IdentityStatus::Done, Some(200)),
            "ledger:hardware",
            "testnet",
            100,
        );
        assert!(ledger.contains("key source:ledger:hardware"));
    }

    #[test]
    fn status_filter_matches_effective_status() {
        let pending = identity(IdentityStatus::Pending, None);
        let done = identity(IdentityStatus::Done, Some(200));
        let expired = identity(IdentityStatus::Done, Some(100));

        assert!(matches_identity_status(
            &pending,
            Some(IdentityListStatus::Pending),
            100
        ));
        assert!(matches_identity_status(
            &done,
            Some(IdentityListStatus::Done),
            100
        ));
        assert!(matches_identity_status(
            &expired,
            Some(IdentityListStatus::Expired),
            100
        ));
        assert!(!matches_identity_status(
            &done,
            Some(IdentityListStatus::Pending),
            100
        ));
    }

    #[test]
    fn format_expiry_renders_utc_date() {
        assert_eq!(format_expiry(1_811_808_000), "2027-06-01");
    }

    #[test]
    fn show_identity_candidates_filter_by_label_and_network() {
        let mut conn = Connection::open_in_memory().unwrap();
        ccd_wallet_core::store::migrations::run(&conn).unwrap();
        let seed = seeds::add(&conn, "main", b"seed secret", "password").unwrap();
        let unlocked = signer_owners::unlock_by_id(&conn, &seed.id, "password").unwrap();
        for (genesis, label, index) in [("genesis-a", "identity", 0), ("genesis-b", "identity", 1)]
        {
            identities::insert_pending(
                &mut conn,
                &unlocked.dek,
                identities::PendingIdentity {
                    network_genesis_hash: genesis,
                    signer_owner_id: &seed.id,
                    ip_identity: 1,
                    identity_index: index,
                    label,
                    code_uri: "https://issuer.example/code",
                },
            )
            .unwrap();
        }

        let all = show_identity_candidates(&conn, None, None).unwrap();
        assert_eq!(all.len(), 2);
        let filtered = show_identity_candidates(&conn, None, Some("genesis-a")).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].network_genesis_hash, "genesis-a");
        let label_and_network =
            show_identity_candidates(&conn, Some("identity"), Some("genesis-b")).unwrap();
        assert_eq!(label_and_network.len(), 1);
        assert_eq!(label_and_network[0].network_genesis_hash, "genesis-b");
    }

    #[test]
    fn identity_reveal_only_includes_attribute_list_fields() {
        let flattened = flatten_identity_object_for_reveal(&json!({
            "value": {
                "attributeList": {
                    "chosenAttributes": {
                        "countryOfResidence": "DK",
                        "firstName": "Ledger"
                    },
                    "createdAt": "202606",
                    "maxAccounts": 200,
                    "validTo": "202706",
                    "ignored": "hidden"
                },
                "preIdentityObject": {
                    "idCredPub": "large-public-value"
                }
            },
            "signature": "large-signature"
        }));
        let text = flattened
            .iter()
            .map(|(path, value)| format!("{}: {value}", identity_reveal_display_key(path)))
            .collect::<Vec<_>>()
            .join("\n");

        assert!(text.contains("countryOfResidence: DK"));
        assert!(text.contains("firstName: Ledger"));
        assert!(text.contains("createdAt: 202606"));
        assert!(text.contains("maxAccounts: 200"));
        assert!(text.contains("validTo: 202706"));
        assert!(!text.contains("value.attributeList"));
        assert!(!text.contains("ignored"));
        assert!(!text.contains("idCredPub"));
        assert!(!text.contains("signature"));
        assert!(!text.contains("large-"));
    }

    #[test]
    fn identity_export_view_uses_wallet_schema_without_internal_ids() {
        let record = IdentityRecord {
            id: 42,
            signer_owner_id: "owner-id".to_owned(),
            network_genesis_hash: "genesis".to_owned(),
            ip_identity: 7,
            identity_index: 3,
            label: "identity".to_owned(),
            status: IdentityStatus::Done,
            created_at: 1_811_808_000,
            expires_at: Some(1_843_344_000),
        };
        let view = IdentityExportView::new(
            record,
            NetworkContext {
                label: "testnet".to_owned(),
                genesis_hash: "genesis".to_owned(),
            },
            KeySourceContext {
                kind: "seed".to_owned(),
                label: "main".to_owned(),
            },
            IdentityPrivatePayload::done(json!({"attribute": "value"})),
        )
        .unwrap();

        let json = serde_json::to_value(&view).unwrap();
        assert_eq!(json["version"], 1);
        assert_eq!(json["identity"]["label"], "identity");
        assert_eq!(json["identity"]["createdAt"], "2027-06-01T00:00:00Z");
        assert_eq!(json["network"]["label"], "testnet");
        assert_eq!(json["network"]["genesisHash"], "genesis");
        assert_eq!(json["keySource"]["kind"], "seed");
        assert!(json["privatePayload"]["codeUri"].is_null());
        assert!(json.get("signer_owner_id").is_none());
        assert!(json.get("id").is_none());
    }

    #[test]
    fn render_identity_reveal_text_covers_pending_and_completed() {
        let completed = IdentityExportView::new(
            identity(IdentityStatus::Done, Some(200)),
            NetworkContext {
                label: "testnet".to_owned(),
                genesis_hash: "genesis".to_owned(),
            },
            KeySourceContext {
                kind: "ledger".to_owned(),
                label: "hardware".to_owned(),
            },
            IdentityPrivatePayload::done(json!({
                "value": {
                    "attributeList": {
                        "chosenAttributes": {"firstName": "Ledger"},
                        "createdAt": "202606",
                        "maxAccounts": 200,
                        "validTo": "202706"
                    }
                }
            })),
        )
        .unwrap();
        let completed_text = render_identity_reveal_text(&completed);
        assert!(completed_text.contains("Key source:  ledger:hardware"));
        assert!(!completed_text.contains("Code URI:"));
        assert!(completed_text.contains("firstName: Ledger"));
        assert!(completed_text.contains("createdAt: 202606"));
        assert!(completed_text.contains("maxAccounts: 200"));
        assert!(completed_text.contains("validTo: 202706"));
        assert!(!completed_text.contains("genesis"));

        let pending = IdentityExportView::new(
            identity(IdentityStatus::Pending, None),
            NetworkContext {
                label: "testnet".to_owned(),
                genesis_hash: "genesis".to_owned(),
            },
            KeySourceContext {
                kind: "seed".to_owned(),
                label: "main".to_owned(),
            },
            IdentityPrivatePayload::pending("https://issuer.example/pending"),
        )
        .unwrap();
        let pending_text = render_identity_reveal_text(&pending);
        assert!(pending_text.contains("<not available yet>"));
        assert!(pending_text.contains("Status:      pending"));

        let pending_with_code_uri = IdentityExportView::new(
            identity(IdentityStatus::Pending, None),
            NetworkContext {
                label: "testnet".to_owned(),
                genesis_hash: "genesis".to_owned(),
            },
            KeySourceContext {
                kind: "seed".to_owned(),
                label: "main".to_owned(),
            },
            IdentityPrivatePayload::pending("https://issuer.example/pending"),
        )
        .unwrap();
        let pending_with_code_uri_text = render_identity_reveal_text(&pending_with_code_uri);
        assert!(pending_with_code_uri_text.contains("Code URI:    https://issuer.example/pending"));
    }

    #[test]
    fn identity_private_payload_decrypts_only_after_key_source_unlock() {
        let mut conn = Connection::open_in_memory().unwrap();
        ccd_wallet_core::store::migrations::run(&conn).unwrap();
        let seed = seeds::add(&conn, "main", b"seed secret", "password").unwrap();
        let unlocked = signer_owners::unlock_by_id(&conn, &seed.id, "password").unwrap();
        let identity_id = identities::insert_pending(
            &mut conn,
            &unlocked.dek,
            identities::PendingIdentity {
                network_genesis_hash: "genesis",
                signer_owner_id: &seed.id,
                ip_identity: 1,
                identity_index: 0,
                label: "identity",
                code_uri: "https://issuer.example/code",
            },
        )
        .unwrap();

        let payload =
            identities::decrypt_private_payload(&conn, identity_id, &unlocked.dek).unwrap();
        assert_eq!(payload.code_uri(), Some("https://issuer.example/code"));
        assert!(signer_owners::unlock_by_id(&conn, &seed.id, "wrong").is_err());
    }

    #[test]
    fn write_export_file_refuses_to_overwrite_existing_file() {
        let dir = std::env::temp_dir().join(format!(
            "ccd-wallet-identity-export-test-{}-{}",
            std::process::id(),
            now_unix_seconds().unwrap()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("identity.json");
        write_export_file(&path, "{}").unwrap();
        let err = write_export_file(&path, "{}").unwrap_err();
        assert!(err.to_string().contains("refusing to overwrite"));
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(&dir);
    }
}
