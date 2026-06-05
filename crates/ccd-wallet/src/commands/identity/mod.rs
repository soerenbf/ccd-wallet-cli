mod new;

use crate::{
    cli::{IdentityListArgs, IdentityRenameArgs, IdentitySubcommand},
    commands::ui::{
        ContextLine, FuzzySelectItem, ResolutionSource, SelectItem, fuzzy_select_or_single,
        log_resolved_context, select_or_single,
    },
};
use anyhow::{Context, Result, bail};
use ccd_wallet_core::store::{
    config::{AppConfig, load},
    identities::{self, IdentityRecord, IdentityStatus},
    seeds,
    signer_owners::{self, SignerOwnerKind},
    wallet_state,
};
use chrono::{DateTime, Utc};
use cliclack::input;
use rusqlite::Connection;
use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

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
        IdentitySubcommand::List(args) => list_identities(conn, args).await,
        IdentitySubcommand::New(args) => new::run(conn, *args).await,
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
        Some(old_label) => {
            let matches = identities::list(conn)?
                .into_iter()
                .filter(|record| record.label == old_label)
                .collect::<Vec<_>>();
            choose_identity_match(
                matches,
                &seeds_by_id,
                &networks_by_hash,
                now,
                args.non_interactive,
            )?
        }
        None if args.non_interactive => {
            bail!("identity label must be provided in --non-interactive mode")
        }
        None => select_identity_fuzzy(
            identities::list(conn)?,
            &seeds_by_id,
            &networks_by_hash,
            now,
        )?,
    };

    let new_label = match args.new_label {
        Some(label) => label,
        None if args.non_interactive => {
            bail!("new identity label must be provided in --non-interactive mode")
        }
        None => input("New identity label:")
            .placeholder(&record.label)
            .validate(|value: &String| {
                if value.is_empty() {
                    Err("Identity label is required.")
                } else {
                    Ok(())
                }
            })
            .interact()?,
    };
    validate_label("identity", &new_label)?;
    identities::rename(conn, record.id, &new_label)?;
    println!("Identity '{}' renamed to '{}'.", record.label, new_label);
    Ok(())
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

fn now_unix_seconds() -> Result<i64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?;
    Ok(duration.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
