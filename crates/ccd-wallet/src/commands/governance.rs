use crate::{
    cli::{
        GovernanceKeysCommand, GovernanceKeysImportArgs, GovernanceKeysListArgs,
        GovernanceKeysRemoveArgs, GovernanceKeysSubcommand, GovernanceSubcommand,
    },
    commands::ui::{
        ContextLine, FuzzySelectItem, ResolutionSource, SelectItem, fuzzy_multiselect_or_single,
        log_resolved_context, select_or_single,
    },
};
use anyhow::{Context, Result, bail};
use ccd_wallet_core::{
    config,
    store::{
        config::{AppConfig, NetworkEntry, load},
        governance, wallet_state,
    },
};
use cliclack::{input, password};
use concordium_rust_sdk::{types::chain_parameters::ChainParameters, v2};
use rusqlite::Connection;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GovernanceAuthorization {
    Root,
    Level1,
    Level2,
    NotAuthorized,
}

impl GovernanceAuthorization {
    fn sort_rank(self) -> u8 {
        match self {
            GovernanceAuthorization::Level2 => 0,
            GovernanceAuthorization::Level1 => 1,
            GovernanceAuthorization::Root => 2,
            GovernanceAuthorization::NotAuthorized => 3,
        }
    }

    fn tag(self) -> &'static str {
        match self {
            GovernanceAuthorization::Level2 => "[level 2]",
            GovernanceAuthorization::Level1 => "[level 1]",
            GovernanceAuthorization::Root => "[root]",
            GovernanceAuthorization::NotAuthorized => "[not authorized]",
        }
    }

    fn summary(self) -> Option<&'static str> {
        match self {
            GovernanceAuthorization::Root => {
                Some("update governance keys (root, level 1, level 2)")
            }
            GovernanceAuthorization::Level1 => Some("update governance keys (level 1, level 2)"),
            GovernanceAuthorization::Level2 | GovernanceAuthorization::NotAuthorized => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum GovernanceCapability {
    AddAr,
    AddIp,
    CcdEuro,
    Consensus,
    Cooldown,
    CreatePlt,
    Emergency,
    EuroEnergy,
    Foundation,
    GasRewards,
    Mint,
    Pool,
    Protocol,
    Time,
    TxFees,
}

impl GovernanceCapability {
    fn label(self) -> &'static str {
        match self {
            GovernanceCapability::AddAr => "add ar",
            GovernanceCapability::AddIp => "add ip",
            GovernanceCapability::CcdEuro => "ccd/euro",
            GovernanceCapability::Consensus => "consensus",
            GovernanceCapability::Cooldown => "cooldown",
            GovernanceCapability::CreatePlt => "create plt",
            GovernanceCapability::Emergency => "emergency",
            GovernanceCapability::EuroEnergy => "euro/energy",
            GovernanceCapability::Foundation => "foundation",
            GovernanceCapability::GasRewards => "gas rewards",
            GovernanceCapability::Mint => "mint",
            GovernanceCapability::Pool => "pool",
            GovernanceCapability::Protocol => "protocol",
            GovernanceCapability::Time => "time",
            GovernanceCapability::TxFees => "tx fees",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GovernanceListEntry {
    verify_key: String,
    authorization: GovernanceAuthorization,
    capabilities: Vec<GovernanceCapability>,
}

impl GovernanceListEntry {
    fn detail(&self) -> Option<String> {
        if let Some(summary) = self.authorization.summary() {
            return Some(summary.to_owned());
        }
        match self.authorization {
            GovernanceAuthorization::Level2 => Some(
                self.capabilities
                    .iter()
                    .map(|capability| capability.label())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            GovernanceAuthorization::Root
            | GovernanceAuthorization::Level1
            | GovernanceAuthorization::NotAuthorized => None,
        }
    }
}

pub async fn run(conn: &mut Connection, command: crate::cli::GovernanceSubcommand) -> Result<()> {
    match command {
        GovernanceSubcommand::Keys(GovernanceKeysCommand { command }) => match command {
            GovernanceKeysSubcommand::Import(args) => import_keys(conn, args).await,
            GovernanceKeysSubcommand::List(args) => list_keys(conn, args).await,
            GovernanceKeysSubcommand::Remove(args) => remove_keys(conn, args).await,
        },
    }
}

async fn import_keys(conn: &mut Connection, args: GovernanceKeysImportArgs) -> Result<()> {
    let file_or_dir = resolve_import_target(args.file, args.dir, args.non_interactive)?;
    let (network_name, network_entry, endpoint_label, source) =
        resolve_governance_network(conn, args.network.as_deref(), false, args.non_interactive)
            .await?;
    log_resolved_context(&[ContextLine {
        label: "network:",
        value: format!("{network_name} @ {endpoint_label}"),
        source,
    }])?;
    let vault_password =
        prompt_governance_vault_password(conn, &network_name, &network_entry.genesis_hash)?;
    let vault =
        governance::create_or_unlock_vault(conn, &network_entry.genesis_hash, &vault_password)?;
    let files = collect_import_files(&file_or_dir)?;
    let mut imported = 0usize;
    for file in files {
        let raw_json = fs::read_to_string(&file)
            .with_context(|| format!("failed to read governance key file {}", file.display()))?;
        governance::import_key_json(&mut *conn, &vault.record, &vault.dek, &raw_json)
            .with_context(|| format!("failed to import governance key from {}", file.display()))?;
        imported += 1;
    }
    println!(
        "Imported {} governance key(s) on network '{}'.",
        imported, network_name
    );
    Ok(())
}

async fn list_keys(conn: &mut Connection, args: GovernanceKeysListArgs) -> Result<()> {
    let (network_name, network_entry, endpoint_label, source) = resolve_governance_network(
        conn,
        args.network.as_deref(),
        !args.no_defaults,
        args.non_interactive,
    )
    .await?;
    log_resolved_context(&[ContextLine {
        label: "network:",
        value: format!("{network_name} @ {endpoint_label}"),
        source,
    }])?;
    ensure_governance_keys_available_for_listing(conn, &network_name, &network_entry.genesis_hash)?;
    let password = password(format!("Governance vault password for '{}':", network_name))
        .allow_empty()
        .interact()?;
    let vault = governance::unlock_vault(conn, &network_entry.genesis_hash, &password)?;
    let decrypted = governance::decrypted_keys(conn, &network_entry.genesis_hash, &vault.dek)?;
    let chain_parameters =
        fetch_chain_parameters(&network_entry.node_endpoint, &endpoint_label).await?;
    let entries = match_governance_keys(&decrypted, &chain_parameters);
    for row in render_governance_list_rows(&entries, !args.show_full) {
        println!("{row}");
    }
    Ok(())
}

async fn remove_keys(conn: &mut Connection, args: GovernanceKeysRemoveArgs) -> Result<()> {
    let (network_name, network_entry, endpoint_label, source) =
        resolve_governance_network(conn, args.network.as_deref(), false, args.non_interactive)
            .await?;
    log_resolved_context(&[ContextLine {
        label: "network:",
        value: format!("{network_name} @ {endpoint_label}"),
        source,
    }])?;
    let password_value = password(format!("Governance vault password for '{}':", network_name))
        .allow_empty()
        .interact()?;
    let vault = governance::unlock_vault(conn, &network_entry.genesis_hash, &password_value)?;

    if args.all {
        let removed = governance::remove_all(conn, &network_entry.genesis_hash)?;
        println!(
            "Removed {} governance key(s) from '{}'.",
            removed, network_name
        );
        return Ok(());
    }

    let verify_keys = match args.verify_key {
        Some(verify_key) => vec![verify_key],
        None if args.non_interactive => {
            bail!("verify key must be provided in --non-interactive mode unless `--all` is used")
        }
        None => {
            let decrypted =
                governance::decrypted_keys(conn, &network_entry.genesis_hash, &vault.dek)?;
            if decrypted.is_empty() {
                bail!("no governance keys are stored for '{}'", network_name);
            }
            let chain_parameters =
                fetch_chain_parameters(&network_entry.node_endpoint, &endpoint_label).await?;
            let entries = match_governance_keys(&decrypted, &chain_parameters);
            let items = entries
                .into_iter()
                .map(|entry| FuzzySelectItem {
                    value: entry.verify_key.clone(),
                    text: render_governance_list_row(&entry, true),
                })
                .collect::<Vec<_>>();
            let selected = fuzzy_multiselect_or_single("Select governance keys to remove", &items)?;
            if selected.is_empty() {
                bail!("at least one governance key must be selected");
            }
            selected
        }
    };

    let mut removed = Vec::new();
    for verify_key in verify_keys {
        if !governance::remove_by_verify_key(
            conn,
            &network_entry.genesis_hash,
            &vault.dek,
            &verify_key,
        )? {
            bail!(
                "governance key '{}' is not stored for network '{}'",
                verify_key,
                network_name
            );
        }
        removed.push(verify_key);
    }
    if removed.len() == 1 {
        println!(
            "Removed governance key '{}' from '{}'.",
            removed[0], network_name
        );
    } else {
        println!(
            "Removed {} governance key(s) from '{}'.",
            removed.len(),
            network_name
        );
    }
    Ok(())
}

fn resolve_import_target(
    file: Option<PathBuf>,
    dir: Option<PathBuf>,
    non_interactive: bool,
) -> Result<PathBuf> {
    match (file, dir) {
        (Some(file), None) => Ok(file),
        (None, Some(dir)) => Ok(dir),
        (None, None) if non_interactive => {
            bail!("governance key file or `--dir <DIR>` must be provided in --non-interactive mode")
        }
        (None, None) => {
            let value: String = input("Governance key file or directory:").interact()?;
            Ok(PathBuf::from(value))
        }
        (Some(_), Some(_)) => unreachable!("clap enforces conflicts"),
    }
}

fn collect_import_files(path: &Path) -> Result<Vec<PathBuf>> {
    if path.is_dir() {
        let mut files = fs::read_dir(path)
            .with_context(|| format!("failed to read governance key directory {}", path.display()))?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
            .filter(|path| {
                path.file_name().and_then(|name| name.to_str()) != Some("governance-keys.json")
            })
            .collect::<Vec<_>>();
        files.sort();
        if files.is_empty() {
            bail!(
                "no governance key JSON files were found in {}",
                path.display()
            );
        }
        Ok(files)
    } else {
        Ok(vec![path.to_path_buf()])
    }
}

async fn resolve_governance_network(
    conn: &Connection,
    network: Option<&str>,
    allow_active_default: bool,
    non_interactive: bool,
) -> Result<(String, NetworkEntry, String, ResolutionSource)> {
    let app_config = load()?;
    let (selected_name, source) = match network {
        Some(name) => (name.to_owned(), ResolutionSource::Explicit),
        None if allow_active_default => {
            match wallet_state::get(conn, wallet_state::ACTIVE_NETWORK_KEY)? {
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
        .get(&selected_name)
        .cloned()
        .with_context(|| format!("network '{}' is not registered", selected_name))?;
    Ok((
        selected_name,
        entry.clone(),
        entry.node_endpoint.clone(),
        source,
    ))
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
            hint: entry.node_endpoint.clone(),
        })
        .collect::<Vec<_>>();
    let initial = active.map(str::to_owned);
    select_or_single("Select network", &items, initial.as_ref())
}

fn prompt_governance_vault_password(
    conn: &Connection,
    network_name: &str,
    network_genesis_hash: &str,
) -> Result<String> {
    let exists = governance::governance_vault_exists(conn, network_genesis_hash)?;
    if !exists {
        cliclack::log::info(format!(
            "Setting up governance vault for '{}'.",
            network_name
        ))?;
    }
    let prompt = if exists {
        format!("Governance vault password for '{}':", network_name)
    } else {
        format!("Set governance vault password for '{}':", network_name)
    };
    let vault_password = password(prompt).allow_empty().interact()?;
    if !exists {
        let confirmation = password(format!(
            "Confirm governance vault password for '{}':",
            network_name
        ))
        .allow_empty()
        .interact()?;
        if confirmation != vault_password {
            bail!("governance vault password confirmation did not match");
        }
    }
    Ok(vault_password)
}

fn ensure_governance_keys_available_for_listing(
    conn: &Connection,
    network_name: &str,
    network_genesis_hash: &str,
) -> Result<()> {
    if governance::governance_vault_exists(conn, network_genesis_hash)? {
        return Ok(());
    }
    bail!(
        "no governance keys are stored for '{}'; import one with `ccd-wallet governance keys import ... --network {}`",
        network_name,
        network_name
    )
}

async fn fetch_chain_parameters(
    node_endpoint: &str,
    endpoint_label: &str,
) -> Result<ChainParameters> {
    let endpoint: v2::Endpoint = ccd_wallet_core::config::normalize_url_string(node_endpoint)
        .parse()
        .with_context(|| format!("invalid node endpoint: {node_endpoint}"))?;
    let mut client = config::connect_v2_client(endpoint)
        .await
        .with_context(|| format!("failed to connect to Concordium node at {endpoint_label}"))?;
    client
        .get_block_chain_parameters(&v2::BlockIdentifier::LastFinal)
        .await
        .with_context(|| format!("failed to query chain parameters from node at {endpoint_label}"))
        .map(|response| response.response)
}

fn match_governance_keys(
    keys: &[governance::DecryptedGovernanceKey],
    chain_parameters: &ChainParameters,
) -> Vec<GovernanceListEntry> {
    let root_keys = chain_parameters
        .keys
        .root_keys
        .as_ref()
        .map(|keys| &keys.keys)
        .cloned()
        .unwrap_or_default();
    let level1_keys = chain_parameters
        .keys
        .level_1_keys
        .as_ref()
        .map(|keys| &keys.keys)
        .cloned()
        .unwrap_or_default();
    let level2 = chain_parameters.keys.level_2_keys.as_ref();

    let mut entries = keys
        .iter()
        .map(|entry| {
            let (authorization, capabilities) = if root_keys.contains(&entry.public_key) {
                (GovernanceAuthorization::Root, Vec::new())
            } else if level1_keys.contains(&entry.public_key) {
                (GovernanceAuthorization::Level1, Vec::new())
            } else if let Some(level2) = level2 {
                match level2.keys.iter().position(|key| key == &entry.public_key) {
                    Some(key_index) => {
                        let capabilities = level2_capabilities_for_index(level2, key_index);
                        if capabilities.is_empty() {
                            (GovernanceAuthorization::NotAuthorized, Vec::new())
                        } else {
                            (GovernanceAuthorization::Level2, capabilities)
                        }
                    }
                    None => (GovernanceAuthorization::NotAuthorized, Vec::new()),
                }
            } else {
                (GovernanceAuthorization::NotAuthorized, Vec::new())
            };
            GovernanceListEntry {
                verify_key: governance::public_key_hex(&entry.public_key),
                authorization,
                capabilities,
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| {
        a.authorization
            .sort_rank()
            .cmp(&b.authorization.sort_rank())
            .then_with(|| a.verify_key.cmp(&b.verify_key))
    });
    entries
}

fn level2_capabilities_for_index(
    level2: &concordium_rust_sdk::types::chain_parameters::Level2Keys,
    key_index: usize,
) -> Vec<GovernanceCapability> {
    let is_authorized = |access: Option<&concordium_rust_sdk::base::updates::AccessStructure>| {
        access.is_some_and(|access| {
            access
                .authorized_keys
                .iter()
                .any(|index| usize::from(index.index) == key_index)
        })
    };

    let mut capabilities = Vec::new();
    if is_authorized(level2.add_anonymity_revoker.as_ref()) {
        capabilities.push(GovernanceCapability::AddAr);
    }
    if is_authorized(level2.add_identity_provider.as_ref()) {
        capabilities.push(GovernanceCapability::AddIp);
    }
    if is_authorized(level2.micro_ccd_per_euro.as_ref()) {
        capabilities.push(GovernanceCapability::CcdEuro);
    }
    if is_authorized(level2.consensus.as_ref()) {
        capabilities.push(GovernanceCapability::Consensus);
    }
    if is_authorized(level2.cooldown_parameters.as_ref()) {
        capabilities.push(GovernanceCapability::Cooldown);
    }
    if is_authorized(level2.create_plt.as_ref()) {
        capabilities.push(GovernanceCapability::CreatePlt);
    }
    if is_authorized(level2.emergency.as_ref()) {
        capabilities.push(GovernanceCapability::Emergency);
    }
    if is_authorized(level2.euro_per_energy.as_ref()) {
        capabilities.push(GovernanceCapability::EuroEnergy);
    }
    if is_authorized(level2.foundation_account.as_ref()) {
        capabilities.push(GovernanceCapability::Foundation);
    }
    if is_authorized(level2.param_gas_rewards.as_ref()) {
        capabilities.push(GovernanceCapability::GasRewards);
    }
    if is_authorized(level2.mint_distribution.as_ref()) {
        capabilities.push(GovernanceCapability::Mint);
    }
    if is_authorized(level2.pool_parameters.as_ref()) {
        capabilities.push(GovernanceCapability::Pool);
    }
    if is_authorized(level2.protocol.as_ref()) {
        capabilities.push(GovernanceCapability::Protocol);
    }
    if is_authorized(level2.time_parameters.as_ref()) {
        capabilities.push(GovernanceCapability::Time);
    }
    if is_authorized(level2.transaction_fee_distribution.as_ref()) {
        capabilities.push(GovernanceCapability::TxFees);
    }
    capabilities
}

fn abbreviate_verify_key(verify_key: &str) -> String {
    const EDGE: usize = 4;
    if verify_key.len() <= EDGE * 2 + 3 {
        return verify_key.to_owned();
    }
    format!(
        "{}...{}",
        &verify_key[..EDGE],
        &verify_key[verify_key.len() - EDGE..]
    )
}

fn render_governance_list_row(entry: &GovernanceListEntry, compact_key: bool) -> String {
    let tag = entry.authorization.tag();
    let verify_key = if compact_key {
        abbreviate_verify_key(&entry.verify_key)
    } else {
        entry.verify_key.clone()
    };
    match entry.detail() {
        Some(detail) => format!("{tag} {verify_key} - {detail}"),
        None => format!("{tag} {verify_key}"),
    }
}

fn render_governance_list_rows(entries: &[GovernanceListEntry], compact_keys: bool) -> Vec<String> {
    entries
        .iter()
        .map(|entry| render_governance_list_row(entry, compact_keys))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccd_wallet_core::store::{governance as governance_store, migrations};
    use concordium_rust_sdk::{
        base::base::UpdateKeyPair,
        types::chain_parameters::{ChainParameters, Level2Keys, UpdateKeys},
    };
    use rand::thread_rng;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    fn key() -> UpdateKeyPair {
        UpdateKeyPair::generate(&mut thread_rng())
    }

    #[test]
    fn import_target_and_collect_files_validate_inputs() {
        assert!(resolve_import_target(None, None, true).is_err());
        let temp = std::env::temp_dir().join(format!("gov-keys-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::write(temp.join("governance-keys.json"), "{}").unwrap();
        std::fs::write(temp.join("root-key-0.json"), "{}").unwrap();
        let files = collect_import_files(&temp).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name().unwrap(), "root-key-0.json");
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn match_logic_derives_levels_and_unauthorized_state() {
        let root = key();
        let level1 = key();
        let level2_used = key();
        let level2_unused = key();
        let local = vec![
            governance_store::DecryptedGovernanceKey {
                record: governance_store::GovernanceKeyRecord {
                    id: 1,
                    network_genesis_hash: "g".to_owned(),
                    vault_id: "v".to_owned(),
                    created_at: 0,
                    updated_at: 0,
                },
                raw_json: serde_json::to_string(&root).unwrap(),
                public_key: concordium_rust_sdk::base::base::UpdatePublicKey::from(&root),
                key_pair: root.clone(),
            },
            governance_store::DecryptedGovernanceKey {
                record: governance_store::GovernanceKeyRecord {
                    id: 2,
                    network_genesis_hash: "g".to_owned(),
                    vault_id: "v".to_owned(),
                    created_at: 0,
                    updated_at: 0,
                },
                raw_json: serde_json::to_string(&level1).unwrap(),
                public_key: concordium_rust_sdk::base::base::UpdatePublicKey::from(&level1),
                key_pair: level1.clone(),
            },
            governance_store::DecryptedGovernanceKey {
                record: governance_store::GovernanceKeyRecord {
                    id: 3,
                    network_genesis_hash: "g".to_owned(),
                    vault_id: "v".to_owned(),
                    created_at: 0,
                    updated_at: 0,
                },
                raw_json: serde_json::to_string(&level2_used).unwrap(),
                public_key: concordium_rust_sdk::base::base::UpdatePublicKey::from(&level2_used),
                key_pair: level2_used.clone(),
            },
            governance_store::DecryptedGovernanceKey {
                record: governance_store::GovernanceKeyRecord {
                    id: 4,
                    network_genesis_hash: "g".to_owned(),
                    vault_id: "v".to_owned(),
                    created_at: 0,
                    updated_at: 0,
                },
                raw_json: serde_json::to_string(&level2_unused).unwrap(),
                public_key: concordium_rust_sdk::base::base::UpdatePublicKey::from(&level2_unused),
                key_pair: level2_unused.clone(),
            },
        ];
        let params = ChainParameters {
            keys: UpdateKeys {
                root_keys: Some(
                    concordium_rust_sdk::base::updates::HigherLevelAccessStructure {
                        keys: vec![concordium_rust_sdk::base::base::UpdatePublicKey::from(
                            &root,
                        )],
                        threshold: 1u16.try_into().unwrap(),
                        _phantom: Default::default(),
                    },
                ),
                level_1_keys: Some(
                    concordium_rust_sdk::base::updates::HigherLevelAccessStructure {
                        keys: vec![concordium_rust_sdk::base::base::UpdatePublicKey::from(
                            &level1,
                        )],
                        threshold: 1u16.try_into().unwrap(),
                        _phantom: Default::default(),
                    },
                ),
                level_2_keys: Some(Level2Keys {
                    keys: vec![
                        concordium_rust_sdk::base::base::UpdatePublicKey::from(&level2_used),
                        concordium_rust_sdk::base::base::UpdatePublicKey::from(&level2_unused),
                    ],
                    protocol: Some(concordium_rust_sdk::base::updates::AccessStructure {
                        authorized_keys: [0u16.into()].into_iter().collect(),
                        threshold: 1u16.try_into().unwrap(),
                    }),
                    emergency: None,
                    consensus: None,
                    euro_per_energy: None,
                    micro_ccd_per_euro: None,
                    foundation_account: None,
                    mint_distribution: None,
                    transaction_fee_distribution: None,
                    param_gas_rewards: None,
                    pool_parameters: None,
                    add_anonymity_revoker: None,
                    add_identity_provider: None,
                    cooldown_parameters: None,
                    time_parameters: None,
                    create_plt: Some(concordium_rust_sdk::base::updates::AccessStructure {
                        authorized_keys: [0u16.into()].into_iter().collect(),
                        threshold: 1u16.try_into().unwrap(),
                    }),
                }),
            },
            ..Default::default()
        };
        let matched = match_governance_keys(&local, &params);
        assert_eq!(matched[0].authorization, GovernanceAuthorization::Level2);
        assert_eq!(
            matched[0].verify_key,
            governance_store::public_key_hex(&local[2].public_key)
        );
        assert_eq!(
            matched[0].capabilities,
            vec![
                GovernanceCapability::CreatePlt,
                GovernanceCapability::Protocol
            ]
        );
        assert_eq!(matched[1].authorization, GovernanceAuthorization::Level1);
        assert_eq!(
            matched[1].verify_key,
            governance_store::public_key_hex(&local[1].public_key)
        );
        assert_eq!(matched[2].authorization, GovernanceAuthorization::Root);
        assert_eq!(
            matched[2].verify_key,
            governance_store::public_key_hex(&local[0].public_key)
        );
        assert_eq!(
            matched[3].authorization,
            GovernanceAuthorization::NotAuthorized
        );
        assert_eq!(
            matched[3].verify_key,
            governance_store::public_key_hex(&local[3].public_key)
        );
    }

    #[test]
    fn list_preflight_requires_existing_vault() {
        let conn = conn();
        let err =
            ensure_governance_keys_available_for_listing(&conn, "local", "genesis").unwrap_err();
        assert!(
            err.to_string()
                .contains("no governance keys are stored for 'local'")
        );
    }

    #[test]
    fn governance_rows_render_tag_first_without_alignment_padding() {
        let rows = render_governance_list_rows(
            &[
                GovernanceListEntry {
                    verify_key: "key-level2".to_owned(),
                    authorization: GovernanceAuthorization::Level2,
                    capabilities: vec![
                        GovernanceCapability::CreatePlt,
                        GovernanceCapability::Protocol,
                    ],
                },
                GovernanceListEntry {
                    verify_key: "key-level1".to_owned(),
                    authorization: GovernanceAuthorization::Level1,
                    capabilities: Vec::new(),
                },
                GovernanceListEntry {
                    verify_key: "key-root".to_owned(),
                    authorization: GovernanceAuthorization::Root,
                    capabilities: Vec::new(),
                },
                GovernanceListEntry {
                    verify_key: "key-stale".to_owned(),
                    authorization: GovernanceAuthorization::NotAuthorized,
                    capabilities: Vec::new(),
                },
            ],
            false,
        );
        assert_eq!(
            rows,
            vec![
                "[level 2] key-level2 - create plt, protocol",
                "[level 1] key-level1 - update governance keys (level 1, level 2)",
                "[root] key-root - update governance keys (root, level 1, level 2)",
                "[not authorized] key-stale",
            ]
        );
    }

    #[test]
    fn compact_verify_key_abbreviates_long_keys() {
        assert_eq!(abbreviate_verify_key("1234567890abcdef"), "1234...cdef");
        assert_eq!(abbreviate_verify_key("1234567"), "1234567");
    }

    #[test]
    fn list_rows_can_render_compact_keys() {
        let row = render_governance_list_row(
            &GovernanceListEntry {
                verify_key: "1234567890abcdef".to_owned(),
                authorization: GovernanceAuthorization::Level2,
                capabilities: vec![
                    GovernanceCapability::CreatePlt,
                    GovernanceCapability::Protocol,
                ],
            },
            true,
        );
        assert_eq!(row, "[level 2] 1234...cdef - create plt, protocol");
    }

    #[test]
    fn compact_remove_rows_reuse_list_display() {
        let row = render_governance_list_row(
            &GovernanceListEntry {
                verify_key: "1234567890abcdef".to_owned(),
                authorization: GovernanceAuthorization::Level2,
                capabilities: vec![
                    GovernanceCapability::CreatePlt,
                    GovernanceCapability::Protocol,
                ],
            },
            true,
        );
        assert_eq!(row, "[level 2] 1234...cdef - create plt, protocol");
    }

    #[tokio::test]
    async fn fetch_chain_parameters_surfaces_node_failure_actionably() {
        let err = fetch_chain_parameters("http://127.0.0.1:1", "http://127.0.0.1:1")
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("failed to connect")
                || err.to_string().contains("failed to query chain parameters")
        );
    }

    #[test]
    fn governance_vault_password_prompt_only_required_on_first_setup() {
        let conn = conn();
        assert!(!governance_store::governance_vault_exists(&conn, "genesis").unwrap());
        governance_store::create_or_unlock_vault(&conn, "genesis", "").unwrap();
        assert!(governance_store::governance_vault_exists(&conn, "genesis").unwrap());
    }
}
