use crate::{
    cli::IdentityNewArgs,
    commands::{
        ledger,
        ledger_construction::{self, LedgerIdentityIssuanceInput},
        ui::{ContextLine, ResolutionSource, SelectItem, log_resolved_context, select_or_single},
    },
};
use anyhow::{Context, Result, bail};
use ccd_wallet_core::{
    store::{
        config::{NetworkEntry, load},
        identities, seeds,
        signer_owners::{self, SignerOwnerKind},
        wallet_state,
    },
    wallet::{ConcordiumHdWallet, Net},
};
use ccd_wallet_identity_provider::{
    self as identity_provider, IdentityRequestInput,
    callback::{CallbackSession, LoopbackCallbackSession, ManualPasteSession, parse_callback_url},
    client::{self, PollResult, WalletProxyIpEntry},
};
use ccd_wallet_ledger::{ConcordiumLedgerApp, ExportPrivateKeyNetwork, HidTransport};
use cliclack::{confirm, input, password, spinner};
use concordium_rust_sdk::{
    id::{
        constants::{ArCurve, IpPairing},
        types::{ArIdentity, ArInfo, GlobalContext, IpInfo},
    },
    v2,
};
use futures_util::StreamExt;
use rusqlite::Connection;
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

const CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);
const POLL_INTERVAL: Duration = Duration::from_secs(2);
const POLL_TIMEOUT: Duration = Duration::from_secs(300);

type OwnerDek = zeroize::Zeroizing<[u8; ccd_wallet_core::store::crypto::KEY_LEN]>;

struct PreparedIdentityRequest {
    signer_owner_id: String,
    signer_owner_dek: OwnerDek,
    identity_index: u32,
    request_json: String,
}

struct IdentityRequestContext<'a> {
    conn: &'a Connection,
    network_entry: &'a NetworkEntry,
    net: Net,
    ip_info: &'a IpInfo<IpPairing>,
    ar_infos: &'a BTreeMap<ArIdentity, ArInfo<ArCurve>>,
    global_context: &'a GlobalContext<ArCurve>,
}

pub async fn run(conn: &mut Connection, args: IdentityNewArgs) -> Result<()> {
    run_with_callback_session(conn, args).await
}

async fn run_with_callback_session(conn: &mut Connection, args: IdentityNewArgs) -> Result<()> {
    let (key_source_label, key_source_source) = resolve_seed_label(
        conn,
        args.seed.as_deref(),
        args.non_interactive,
        args.no_defaults,
    )?;
    let (network_name, network_entry, endpoint, endpoint_label, network_source) =
        resolve_identity_network_context(
            conn,
            args.network.as_deref(),
            args.node.clone(),
            args.non_interactive,
            args.no_defaults,
        )
        .await?;

    log_resolved_context(&[
        ContextLine {
            label: "key source:",
            value: key_source_label.clone(),
            source: key_source_source,
        },
        ContextLine {
            label: "network:",
            value: format!("{network_name} @ {endpoint_label}"),
            source: network_source,
        },
    ])?;

    let label = resolve_identity_label(args.label, args.non_interactive)?;
    validate_identity_label(&label)?;
    if identities::find_by_network_and_label(conn, &network_entry.genesis_hash, &label)?.is_some() {
        bail!(
            "identity label '{}' already exists on network '{}'",
            label,
            network_name
        );
    }

    let spin = spinner();
    spin.start(format!("Connecting to node: {endpoint_label}"));
    let mut client = ccd_wallet_core::config::connect_v2_client(endpoint)
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
    let ip_info = select_provider(
        &ip_infos,
        args.provider,
        args.interactive,
        args.non_interactive,
    )?;

    let spin = spinner();
    spin.start("Fetching wallet proxy metadata...");
    let wallet_proxy = network_entry
        .wallet_proxy
        .as_deref()
        .context("selected network has no wallet_proxy configured")?;
    let wallet_proxy_entries = client::fetch_wallet_proxy_ip_info(wallet_proxy).await?;
    spin.clear();
    let wallet_proxy_entry =
        select_wallet_proxy_entry(&wallet_proxy_entries, ip_info.ip_identity.0)?;

    let spin = spinner();
    spin.start("Fetching anonymity revokers...");
    let ar_infos = fetch_anonymity_revokers(&mut client).await?;
    spin.clear();

    let key_source = signer_owners::find_by_label(conn, &key_source_label)?
        .with_context(|| format!("key source '{}' is not configured", key_source_label))?;
    let net = infer_net(
        &network_name,
        network_entry.wallet_proxy.as_deref(),
        &endpoint_label,
    );
    let request_context = IdentityRequestContext {
        conn,
        network_entry: &network_entry,
        net,
        ip_info,
        ar_infos: &ar_infos,
        global_context: &global_context,
    };
    let prepared = match key_source.kind {
        SignerOwnerKind::Ledger => prepare_ledger_identity_request(
            &request_context,
            &key_source,
            args.allow_ledger_secret_export,
            args.non_interactive,
        )?,
        SignerOwnerKind::Seed => {
            prepare_seed_identity_request(&request_context, &key_source_label)?
        }
    };

    let callback_session = prepare_callback_session(args.manual_callback).await?;
    let redirect_uri = callback_session.redirect_uri().to_owned();

    let spin = spinner();
    spin.start("Contacting identity provider...");
    let browser_url = client::start_issuance(
        &wallet_proxy_entry.metadata.issuance_start,
        &redirect_uri,
        &prepared.request_json,
    )
    .await?;
    spin.clear();
    if let Err(err) = open::that(&browser_url) {
        cliclack::log::warning(format!("failed to open browser automatically: {err}"))?;
    }
    let code_uri = receive_callback(callback_session, &browser_url).await?;

    let record_id = identities::insert_pending(
        conn,
        &prepared.signer_owner_dek,
        identities::PendingIdentity {
            network_genesis_hash: &network_entry.genesis_hash,
            signer_owner_id: &prepared.signer_owner_id,
            ip_identity: ip_info.ip_identity.0,
            identity_index: prepared.identity_index,
            label: &label,
            code_uri: &code_uri,
        },
    )?;

    if args.no_wait {
        cliclack::log::success(format!(
            "Identity '{label}' is pending. It will be checked again when used for account creation."
        ))?;
        return Ok(());
    }

    poll_identity(
        conn,
        record_id,
        &prepared.signer_owner_dek,
        &code_uri,
        &label,
    )
    .await
}

fn prepare_seed_identity_request(
    context: &IdentityRequestContext<'_>,
    key_source_label: &str,
) -> Result<PreparedIdentityRequest> {
    let unlocked_seed = unlock_seed(context.conn, key_source_label)?;
    let seed_phrase = std::str::from_utf8(&unlocked_seed.secret)
        .context("stored seed phrase is not UTF-8")?
        .to_owned();
    let wallet = ConcordiumHdWallet::from_seed_phrase(&seed_phrase, context.net)?;
    let identity_index = identities::next_index(
        context.conn,
        &context.network_entry.genesis_hash,
        &unlocked_seed.record.id,
        context.ip_info.ip_identity.0,
    )?;
    let spin = spinner();
    spin.start("Constructing identity request...");
    let request_json = identity_provider::build_request(
        &wallet,
        context.ip_info,
        context.ar_infos,
        context.global_context,
        identity_index,
    )?;
    spin.clear();

    Ok(PreparedIdentityRequest {
        signer_owner_id: unlocked_seed.record.id,
        signer_owner_dek: unlocked_seed.dek,
        identity_index,
        request_json,
    })
}

fn prepare_ledger_identity_request(
    context: &IdentityRequestContext<'_>,
    key_source: &signer_owners::SignerOwnerRecord,
    allow_ledger_secret_export: bool,
    non_interactive: bool,
) -> Result<PreparedIdentityRequest> {
    let details = signer_owners::find_ledger_details_by_owner_id(context.conn, &key_source.id)?
        .with_context(|| {
            format!(
                "Ledger key source '{}' has no enrollment details",
                key_source.label
            )
        })?;
    let identity_index = identities::next_index(
        context.conn,
        &context.network_entry.genesis_hash,
        &key_source.id,
        context.ip_info.ip_identity.0,
    )?;
    approve_ledger_secret_export(
        &key_source.label,
        non_interactive,
        allow_ledger_secret_export,
    )?;

    let password: String = password(format!(
        "Local password for Ledger key source '{}': ",
        key_source.label
    ))
    .allow_empty()
    .interact()?;
    let unlocked_owner = signer_owners::unlock_by_id(context.conn, &key_source.id, &password)?;

    let transport = HidTransport::open_first()
        .context("failed to open Ledger device; connect a Ledger with the Concordium app open")?;
    let mut app = ConcordiumLedgerApp::new(transport);
    let spin = spinner();
    spin.start("Verifying connected Ledger key source...");
    ledger::verify_connected_ledger_owner(context.conn, &key_source.id, &mut app)?;
    spin.clear();

    let spin = spinner();
    spin.start("Exporting identity issuance material from Ledger...");
    let material = ledger_construction::construct_identity_issuance(
        LedgerIdentityIssuanceInput {
            owner_details: &details,
            network_genesis_hash: &context.network_entry.genesis_hash,
            export_network: ledger_export_network(context.net),
            ip_identity: context.ip_info.ip_identity.0,
            identity_index,
            approved_secret_export: true,
        },
        &mut app,
    )?;
    spin.clear();

    let spin = spinner();
    spin.start("Constructing identity request...");
    let request_json = identity_provider::build_request_from_material(IdentityRequestInput {
        ip_info: context.ip_info,
        ar_infos: context.ar_infos,
        global_context: context.global_context,
        material,
    })?;
    spin.clear();

    Ok(PreparedIdentityRequest {
        signer_owner_id: unlocked_owner.record.id,
        signer_owner_dek: unlocked_owner.dek,
        identity_index,
        request_json,
    })
}

fn approve_ledger_secret_export(
    key_source_label: &str,
    non_interactive: bool,
    allow_ledger_secret_export: bool,
) -> Result<()> {
    if allow_ledger_secret_export {
        return Ok(());
    }
    if non_interactive {
        bail!(
            "Ledger identity issuance for key source '{key_source_label}' requires secret export; rerun with --allow-ledger-secret-export to explicitly allow this non-interactive flow"
        );
    }

    let approved = confirm(format!(
        "Ledger identity issuance for key source '{key_source_label}' must export identity issuance secrets into this process temporarily. This is not an on-device signing flow. Continue?"
    ))
    .initial_value(false)
    .interact()?;
    if !approved {
        bail!("Ledger secret export was declined; no local identity state was written");
    }
    Ok(())
}

fn ledger_export_network(net: Net) -> ExportPrivateKeyNetwork {
    match net {
        Net::Mainnet => ExportPrivateKeyNetwork::Mainnet,
        Net::Testnet => ExportPrivateKeyNetwork::Testnet,
    }
}

async fn fetch_identity_providers(
    client: &mut v2::Client,
) -> Result<Vec<IpInfo<concordium_rust_sdk::id::constants::IpPairing>>> {
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
) -> Result<BTreeMap<ArIdentity, ArInfo<concordium_rust_sdk::id::constants::ArCurve>>> {
    let mut stream = client
        .get_anonymity_revokers(v2::BlockIdentifier::LastFinal)
        .await
        .context("failed to fetch anonymity revokers from the node")?
        .response;

    let mut revokers = BTreeMap::new();
    while let Some(item) = stream.next().await {
        let ar_info = item.context("failed to read anonymity revoker from stream")?;
        revokers.insert(ar_info.ar_identity, ar_info);
    }
    if revokers.is_empty() {
        bail!("no anonymity revokers are available on the selected network");
    }
    Ok(revokers)
}

fn select_provider(
    providers: &[IpInfo<concordium_rust_sdk::id::constants::IpPairing>],
    provider: Option<u32>,
    interactive: bool,
    non_interactive: bool,
) -> Result<&IpInfo<concordium_rust_sdk::id::constants::IpPairing>> {
    match (provider, interactive) {
        (Some(id), false) => providers
            .iter()
            .find(|ip| ip.ip_identity.0 == id)
            .with_context(|| {
                format!("identity provider {id} is not registered on the selected network")
            }),
        (None, true) => select_provider_interactively(providers),
        (Some(_), true) => bail!("--provider and --interactive are mutually exclusive"),
        (None, false) if non_interactive => {
            bail!("specify either --provider <ID> or --interactive in --non-interactive mode")
        }
        (None, false) => select_provider_interactively(providers),
    }
}

fn select_wallet_proxy_entry(
    entries: &[WalletProxyIpEntry],
    provider_id: u32,
) -> Result<&WalletProxyIpEntry> {
    entries
        .iter()
        .find(|entry| entry.ip_info.ip_identity.0 == provider_id)
        .with_context(|| {
            format!("wallet proxy did not provide metadata for identity provider {provider_id}")
        })
}

fn resolve_identity_label(explicit: Option<String>, non_interactive: bool) -> Result<String> {
    match explicit {
        Some(label) => Ok(label),
        None if non_interactive => {
            bail!("identity label must be provided in --non-interactive mode")
        }
        None => Ok(input("Identity label:")
            .validate(|value: &String| {
                if value.is_empty() {
                    Err("Identity label is required.")
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
        Some(label) => signer_owners::find_by_label(conn, label)?
            .map(|owner| (owner.label, ResolutionSource::Explicit))
            .with_context(|| format!("key source '{}' is not configured", label)),
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
                    "No active key source. Run `ccd-wallet seed use <LABEL>` or supply `--key-source <LABEL>`."
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
    let owners = signer_owners::list(conn)?;
    if owners.is_empty() {
        bail!(
            "no key sources are configured; run `ccd-wallet seed add <LABEL>` or `ccd-wallet ledger setup <LABEL>` first"
        )
    }

    let items = owners
        .iter()
        .map(|owner| SelectItem {
            value: owner.label.clone(),
            label: owner.label.clone(),
            hint: match owner.kind {
                SignerOwnerKind::Seed => "seed".to_owned(),
                SignerOwnerKind::Ledger => "ledger".to_owned(),
            },
        })
        .collect::<Vec<_>>();
    let initial = active.map(str::to_owned);
    select_or_single("Select key source", &items, initial.as_ref())
}

fn unlock_seed(conn: &Connection, seed_label: &str) -> Result<seeds::UnlockedSeed> {
    let password: String = password(format!("Password for seed '{seed_label}': "))
        .allow_empty()
        .interact()?;
    seeds::unlock_context(conn, seed_label, &password)
}

async fn prepare_callback_session(manual_callback: bool) -> Result<CallbackSession> {
    if manual_callback {
        return Ok(CallbackSession::Manual(ManualPasteSession));
    }

    Ok(CallbackSession::Loopback(
        LoopbackCallbackSession::bind(CALLBACK_TIMEOUT).await?,
    ))
}

async fn receive_callback(callback_session: CallbackSession, browser_url: &str) -> Result<String> {
    match callback_session {
        CallbackSession::Manual(_) => receive_manual_callback(browser_url),
        CallbackSession::Loopback(session) => session.receive().await,
    }
}

fn receive_manual_callback(browser_url: &str) -> Result<String> {
    cliclack::log::info(format!("Open this URL in your browser:\n\n{browser_url}\n"))?;
    let callback_url: String = input("Paste the final redirect URL:")
        .validate(|value: &String| {
            if value.is_empty() {
                Err("Redirect URL is required.")
            } else {
                Ok(())
            }
        })
        .interact()?;
    parse_callback_url(callback_url.trim())
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
            hint: entry.node_endpoint.to_string(),
        })
        .collect::<Vec<_>>();
    let initial = active.map(str::to_owned);
    select_or_single("Select network", &items, initial.as_ref())
}

fn infer_net(network_name: &str, wallet_proxy: Option<&str>, endpoint_label: &str) -> Net {
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

async fn resolve_identity_network_context(
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
                .with_context(|| {
                    format!(
                        "network '{}' is not registered; run `ccd-wallet network add --name {} --node <ENDPOINT> --wallet-proxy <URL>` first",
                        network_name, network_name
                    )
                })?;

            if entry.genesis_hash != node_genesis_hash {
                bail!(
                    "node at {} belongs to genesis hash {}, which does not match configured network '{}' ({})",
                    endpoint_label,
                    node_genesis_hash,
                    network_name,
                    entry.genesis_hash
                );
            }

            if entry
                .wallet_proxy
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            {
                bail!("network '{}' has no wallet_proxy configured", network_name);
            }

            return Ok((
                network_name.to_owned(),
                entry,
                endpoint,
                endpoint_label,
                ResolutionSource::Explicit,
            ));
        }

        let active_network = wallet_state::get(conn, wallet_state::ACTIVE_NETWORK_KEY)?;
        if !no_defaults
            && let Some(active_network) = active_network.clone()
            && let Some(entry) = app_config.networks.get(&active_network)
            && entry.genesis_hash == node_genesis_hash
        {
            let entry = entry.clone();
            if entry
                .wallet_proxy
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            {
                bail!(
                    "network '{}' has no wallet_proxy configured",
                    active_network
                );
            }
            return Ok((
                active_network,
                entry,
                endpoint,
                endpoint_label,
                ResolutionSource::ActiveDefault,
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

        let (matched_name, matched_entry, resolution_source) = if no_defaults && matches.len() > 1 {
            let selected_name =
                prompt_for_matching_network_name(&matches, active_network.as_deref())?;
            let entry = matches
                .iter()
                .find(|(name, _)| *name == selected_name)
                .map(|(_, entry)| entry.clone())
                .context("selected network was not found")?;
            (selected_name, entry, ResolutionSource::Prompted)
        } else {
            let (matched_name, matched_entry) = matches[0].clone();
            (matched_name, matched_entry, ResolutionSource::Inferred)
        };

        if matched_entry
            .wallet_proxy
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            bail!("network '{}' has no wallet_proxy configured", matched_name);
        }

        return Ok((
            matched_name,
            matched_entry,
            endpoint,
            endpoint_label,
            resolution_source,
        ));
    }

    let (selected_network, resolution_source) = match network {
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

    if entry
        .wallet_proxy
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        bail!(
            "network '{}' has no wallet_proxy configured",
            selected_network
        );
    }

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

    Ok((
        selected_network,
        entry,
        endpoint,
        endpoint_label,
        resolution_source,
    ))
}

fn select_provider_interactively(
    providers: &[IpInfo<concordium_rust_sdk::id::constants::IpPairing>],
) -> Result<&IpInfo<concordium_rust_sdk::id::constants::IpPairing>> {
    let items = providers
        .iter()
        .map(|ip| SelectItem {
            value: ip.ip_identity.0,
            label: ip.ip_description.name.clone(),
            hint: format!("provider id: {}", ip.ip_identity.0),
        })
        .collect::<Vec<_>>();
    let selected_id = select_or_single("Select identity provider", &items, None)?;
    providers
        .iter()
        .find(|ip| ip.ip_identity.0 == selected_id)
        .context("selected identity provider was not found")
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

fn validate_identity_label(label: &str) -> Result<()> {
    if label.is_empty() {
        bail!("identity label must not be empty");
    }
    if !label
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        bail!("identity labels may contain only ASCII letters, digits, dash, and underscore");
    }
    Ok(())
}

async fn poll_identity(
    conn: &mut Connection,
    record_id: i64,
    seed_dek: &[u8; ccd_wallet_core::store::crypto::KEY_LEN],
    code_uri: &str,
    label: &str,
) -> Result<()> {
    let started = Instant::now();
    let spin = spinner();
    spin.start("Polling identity status...");

    loop {
        match client::poll_code_uri(code_uri).await? {
            PollResult::Pending => {
                if started.elapsed() >= POLL_TIMEOUT {
                    spin.clear();
                    bail!(
                        "identity issuance is still pending; retry later with stored code_uri: {code_uri}"
                    );
                }
                tokio::select! {
                    _ = tokio::time::sleep(POLL_INTERVAL) => {}
                    _ = tokio::signal::ctrl_c() => {
                        spin.clear();
                        bail!("identity issuance cancelled; stored code_uri for later retry: {code_uri}");
                    }
                }
            }
            PollResult::Done(token) => {
                spin.clear();
                identities::set_done(conn, record_id, seed_dek, token)?;
                cliclack::log::success(format!("Identity '{label}' issued successfully."))?;
                return Ok(());
            }
            PollResult::ProviderError(detail) => {
                spin.clear();
                identities::delete(conn, record_id)?;
                bail!(detail);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_interactive_ledger_export_requires_allow_flag() {
        let err = approve_ledger_secret_export("ledger", true, false).unwrap_err();
        assert!(err.to_string().contains("--allow-ledger-secret-export"));
    }

    #[test]
    fn explicit_ledger_export_flag_skips_prompt_guard() {
        approve_ledger_secret_export("ledger", true, true).unwrap();
    }
}
