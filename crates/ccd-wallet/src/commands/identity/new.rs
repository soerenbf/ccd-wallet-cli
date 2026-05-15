use crate::cli::IdentityNewArgs;
use anyhow::{Context, Result, bail};
use ccd_wallet_core::{
    store::{
        config::{NetworkEntry, load},
        identities, seeds, wallet_state,
    },
    wallet::{ConcordiumHdWallet, Net},
};
use ccd_wallet_identity_provider::{
    self as identity_provider,
    callback::{CallbackSession, LoopbackCallbackSession, ManualPasteSession},
    client::{self, PollResult, WalletProxyIpEntry},
};
use cliclack::{password, select, spinner};
use concordium_rust_sdk::{
    id::types::{ArIdentity, ArInfo, IpInfo},
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

pub async fn run(conn: &mut Connection, args: IdentityNewArgs) -> Result<()> {
    run_with_callback_session(conn, args).await
}

async fn run_with_callback_session(conn: &mut Connection, args: IdentityNewArgs) -> Result<()> {
    validate_identity_label(&args.label)?;

    let seed_label = resolve_seed_label(conn, args.seed.as_deref())?;
    let (network_name, network_entry, endpoint, endpoint_label) =
        resolve_identity_network_context(conn, args.network.as_deref(), args.node.clone()).await?;

    if identities::find_by_network_and_label(conn, &network_entry.genesis_hash, &args.label)?
        .is_some()
    {
        bail!(
            "identity label '{}' already exists on network '{}'",
            args.label,
            network_name
        );
    }

    cliclack::log::info(format!("Using seed: {seed_label}"))?;
    let unlocked_seed = unlock_seed(conn, &seed_label)?;
    let seed_phrase = std::str::from_utf8(&unlocked_seed.secret)
        .context("stored seed phrase is not UTF-8")?
        .to_owned();

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
    let ip_info = select_provider(&ip_infos, args.provider, args.interactive)?;

    let spin = spinner();
    spin.start("Fetching wallet proxy metadata...");
    let wallet_proxy_entries =
        client::fetch_wallet_proxy_ip_info(&network_entry.wallet_proxy).await?;
    spin.clear();
    let wallet_proxy_entry =
        select_wallet_proxy_entry(&wallet_proxy_entries, ip_info.ip_identity.0)?;

    let spin = spinner();
    spin.start("Fetching anonymity revokers...");
    let ar_infos = fetch_anonymity_revokers(&mut client).await?;
    spin.clear();

    let net = infer_net(&network_name, &network_entry.wallet_proxy, &endpoint_label);
    let wallet = ConcordiumHdWallet::from_seed_phrase(&seed_phrase, net)?;
    let identity_index = identities::next_index(
        conn,
        &network_entry.genesis_hash,
        &unlocked_seed.record.id,
        ip_info.ip_identity.0,
    )?;
    let spin = spinner();
    spin.start("Constructing identity request...");
    let request_json = identity_provider::build_request(
        &wallet,
        ip_info,
        &ar_infos,
        &global_context,
        identity_index,
    )?;
    spin.clear();

    let callback_session = prepare_callback_session(args.manual_callback).await?;
    let redirect_uri = callback_session.redirect_uri().to_owned();

    let spin = spinner();
    spin.start("Contacting identity provider...");
    let browser_url = client::start_issuance(
        &wallet_proxy_entry.metadata.issuance_start,
        &redirect_uri,
        &request_json,
    )
    .await?;
    spin.clear();
    if let Err(err) = open::that(&browser_url) {
        cliclack::log::warning(format!("failed to open browser automatically: {err}"))?;
    }
    let code_uri = callback_session.receive(&browser_url).await?;

    let record_id = identities::insert_pending(
        conn,
        &unlocked_seed.dek,
        identities::PendingIdentity {
            network_genesis_hash: &network_entry.genesis_hash,
            seed_id: &unlocked_seed.record.id,
            ip_identity: ip_info.ip_identity.0,
            identity_index,
            label: &args.label,
            code_uri: &code_uri,
        },
    )?;

    poll_identity(conn, record_id, &unlocked_seed.dek, &code_uri, &args.label).await
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
) -> Result<&IpInfo<concordium_rust_sdk::id::constants::IpPairing>> {
    match (provider, interactive) {
        (Some(id), false) => providers
            .iter()
            .find(|ip| ip.ip_identity.0 == id)
            .with_context(|| {
                format!("identity provider {id} is not registered on the selected network")
            }),
        (None, true) => {
            let mut prompt = select("Select identity provider");
            for ip in providers {
                prompt = prompt.item(
                    ip.ip_identity.0,
                    ip.ip_description.name.clone(),
                    format!("provider id: {}", ip.ip_identity.0),
                );
            }
            let selected_id = prompt.interact()?;
            providers
                .iter()
                .find(|ip| ip.ip_identity.0 == selected_id)
                .context("selected identity provider was not found")
        }
        (Some(_), true) => bail!("--provider and --interactive are mutually exclusive"),
        (None, false) => bail!("specify either --provider <ID> or --interactive"),
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

fn resolve_seed_label(conn: &Connection, explicit: Option<&str>) -> Result<String> {
    match explicit {
        Some(label) => seeds::find_by_label(conn, label)?
            .map(|s| s.label)
            .with_context(|| format!("seed '{}' is not configured", label)),
        None => wallet_state::get(conn, wallet_state::ACTIVE_SEED_KEY)?.with_context(
            || "No active seed. Run `ccd-wallet seed use <LABEL>` or supply `--seed <LABEL>`.",
        ),
    }
}

fn unlock_seed(conn: &Connection, seed_label: &str) -> Result<seeds::UnlockedSeed> {
    let password: String = password(format!("Password for seed '{seed_label}': ")).interact()?;
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

fn infer_net(network_name: &str, wallet_proxy: &str, endpoint_label: &str) -> Net {
    let haystack = format!("{network_name} {wallet_proxy} {endpoint_label}").to_ascii_lowercase();
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
) -> Result<(String, NetworkEntry, v2::Endpoint, String)> {
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

            if entry.wallet_proxy.trim().is_empty() {
                bail!("network '{}' has no wallet_proxy configured", network_name);
            }

            return Ok((network_name.to_owned(), entry, endpoint, endpoint_label));
        }

        let active_network = wallet_state::get(conn, wallet_state::ACTIVE_NETWORK_KEY)?;
        if let Some(active_network) = active_network
            && let Some(entry) = app_config.networks.get(&active_network)
            && entry.genesis_hash == node_genesis_hash
        {
            let entry = entry.clone();
            if entry.wallet_proxy.trim().is_empty() {
                bail!(
                    "network '{}' has no wallet_proxy configured",
                    active_network
                );
            }
            return Ok((active_network, entry, endpoint, endpoint_label));
        }

        let (matched_name, matched_entry) = app_config
            .networks
            .iter()
            .find(|(_, entry)| entry.genesis_hash == node_genesis_hash)
            .with_context(|| {
                format!(
                    "no configured network matches the supplied node at {} (genesis hash: {})",
                    endpoint_label, node_genesis_hash
                )
            })?;

        let matched_entry = matched_entry.clone();
        if matched_entry.wallet_proxy.trim().is_empty() {
            bail!("network '{}' has no wallet_proxy configured", matched_name);
        }

        return Ok((
            matched_name.clone(),
            matched_entry,
            endpoint,
            endpoint_label,
        ));
    }

    let selected_network = match network {
        Some(name) => name.to_owned(),
        None => wallet_state::get(conn, wallet_state::ACTIVE_NETWORK_KEY)?.with_context(
            || "no active network is set; provide `--network` or run `ccd-wallet network use <NAME>`",
        )?,
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

    if entry.wallet_proxy.trim().is_empty() {
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

    Ok((selected_network, entry, endpoint, endpoint_label))
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
