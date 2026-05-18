use crate::{
    cli::{
        GovernanceKeysCommand, GovernanceKeysImportArgs, GovernanceKeysListArgs,
        GovernanceKeysRemoveArgs, GovernanceKeysSubcommand, GovernanceSubcommand,
        GovernanceUpdateArgs,
    },
    commands::ui::{
        ContextLine, FuzzySelectItem, ResolutionSource, SelectItem, fuzzy_multiselect_or_single,
        fuzzy_multiselect_or_single_with_initial, log_resolved_context, select_or_single,
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
use chrono::{DateTime, Utc};
use cliclack::{input, password, spinner};
use concordium_rust_sdk::{
    base::{
        base::{UpdateKeyPair, UpdateKeysIndex, UpdatePublicKey, UpdateSequenceNumber},
        transactions::{BlockItem, Payload},
        updates::{
            EncodedUpdatePayload, UpdateHeader, UpdateInstruction, UpdateSigner,
            update as update_instruction,
        },
    },
    common::{Serial, cbor, types::TransactionTime},
    protocol_level_tokens::{
        CborHolderAccount, MetadataUrl, TokenAmount, TokenModuleInitializationParameters,
    },
    types::{
        UpdatePayload, UpdateType, chain_parameters::ChainParameters,
        queries::NextUpdateSequenceNumbers,
    },
    v2,
};
use rusqlite::Connection;
use sha2::Digest;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
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
        GovernanceSubcommand::Update(args) => update(conn, *args).await,
    }
}

async fn update(conn: &mut Connection, args: GovernanceUpdateArgs) -> Result<()> {
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

    let payload_input = resolve_update_payload_input(&args)?;
    let payload = resolve_update_payload(payload_input, &args)?;
    validate_blind_signing_context(&payload, &args)?;
    let _timing = resolve_update_timing(&args)?;
    let chain_resolution = spinner();
    chain_resolution.start("Resolving governance authorization context from chain...");
    let chain_context = match resolve_update_chain_context(
        &network_entry.node_endpoint,
        &endpoint_label,
        &payload,
        args.sequence_number,
    )
    .await
    {
        Ok(chain_context) => {
            chain_resolution.stop("Resolved governance authorization context from chain.");
            chain_context
        }
        Err(err) => {
            chain_resolution.clear();
            return Err(err);
        }
    };
    log_resolved_context(&payload_context_lines(&payload, &chain_context))?;

    ensure_governance_keys_available_for_listing(conn, &network_name, &network_entry.genesis_hash)?;
    let password_value = password(format!("Governance vault password for '{}':", network_name))
        .allow_empty()
        .interact()?;
    let vault = governance::unlock_vault(conn, &network_entry.genesis_hash, &password_value)?;
    let decrypted = governance::decrypted_keys(conn, &network_entry.genesis_hash, &vault.dek)?;
    let signers = resolve_update_signers(&args, &decrypted, &chain_context, &payload)?;
    let block_item = build_signed_update_instruction(&payload, &chain_context, &signers, _timing)?;
    let transaction_hash =
        submit_governance_update(&network_entry.node_endpoint, &endpoint_label, &block_item)
            .await?;
    let message = format!("Submitted governance update: {transaction_hash}");
    if args.no_wait {
        println!("{message}");
        return Ok(());
    } else {
        let _ = cliclack::log::success(message);
    }

    wait_for_governance_update_finalization(
        &network_entry.node_endpoint,
        &endpoint_label,
        &transaction_hash,
    )
    .await?;

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum GovernanceUpdatePayloadInput {
    Json(String),
    Serialized(Vec<u8>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GovernanceUpdateTiming {
    effective_time_seconds: u64,
    timeout_seconds: u64,
}

#[derive(Clone, Debug)]
struct GovernanceUpdateChainContext {
    chain_parameters: ChainParameters,
    sequence_number: Option<UpdateSequenceNumber>,
    sequence_number_source: Option<ResolutionSource>,
}

#[derive(Clone, Debug)]
enum ResolvedGovernanceUpdatePayload {
    Known {
        payload: UpdatePayload,
        auth_family: GovernanceAuthFamily,
        sequence_queue: GovernanceSequenceQueue,
    },
    Blind {
        bytes: Vec<u8>,
        auth_family_hint: Option<GovernanceAuthFamily>,
        sequence_queue_hint: Option<GovernanceSequenceQueue>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GovernanceAuthFamily {
    Root,
    Level1,
    Level2(GovernanceCapability),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GovernanceSequenceQueue {
    RootKeys,
    Level1Keys,
    Level2Keys,
    Protocol,
    ElectionDifficulty,
    EuroPerEnergy,
    MicroCcdPerEuro,
    FoundationAccount,
    MintDistribution,
    TransactionFeeDistribution,
    GasRewards,
    PoolParameters,
    AddAnonymityRevoker,
    AddIdentityProvider,
    CooldownParameters,
    TimeParameters,
    TimeoutParameters,
    MinBlockTime,
    BlockEnergyLimit,
    FinalizationCommitteeParameters,
    ValidatorScoreParameters,
    ProtocolLevelTokens,
}

impl GovernanceSequenceQueue {
    fn label(self) -> &'static str {
        match self {
            GovernanceSequenceQueue::RootKeys => "root keys",
            GovernanceSequenceQueue::Level1Keys => "level 1 keys",
            GovernanceSequenceQueue::Level2Keys => "level 2 keys",
            GovernanceSequenceQueue::Protocol => "protocol",
            GovernanceSequenceQueue::ElectionDifficulty => "election difficulty",
            GovernanceSequenceQueue::EuroPerEnergy => "euro/energy",
            GovernanceSequenceQueue::MicroCcdPerEuro => "ccd/euro",
            GovernanceSequenceQueue::FoundationAccount => "foundation account",
            GovernanceSequenceQueue::MintDistribution => "mint distribution",
            GovernanceSequenceQueue::TransactionFeeDistribution => "transaction fee distribution",
            GovernanceSequenceQueue::GasRewards => "gas rewards",
            GovernanceSequenceQueue::PoolParameters => "pool parameters",
            GovernanceSequenceQueue::AddAnonymityRevoker => "add anonymity revoker",
            GovernanceSequenceQueue::AddIdentityProvider => "add identity provider",
            GovernanceSequenceQueue::CooldownParameters => "cooldown parameters",
            GovernanceSequenceQueue::TimeParameters => "time parameters",
            GovernanceSequenceQueue::TimeoutParameters => "timeout parameters",
            GovernanceSequenceQueue::MinBlockTime => "minimum block time",
            GovernanceSequenceQueue::BlockEnergyLimit => "block energy limit",
            GovernanceSequenceQueue::FinalizationCommitteeParameters => {
                "finalization committee parameters"
            }
            GovernanceSequenceQueue::ValidatorScoreParameters => "validator score parameters",
            GovernanceSequenceQueue::ProtocolLevelTokens => "protocol level tokens",
        }
    }

    fn from_update_type(update_type: UpdateType) -> Self {
        match update_type {
            UpdateType::UpdateRootKeys => Self::RootKeys,
            UpdateType::UpdateLevel1Keys => Self::Level1Keys,
            UpdateType::UpdateLevel2Keys => Self::Level2Keys,
            UpdateType::UpdateProtocol => Self::Protocol,
            UpdateType::UpdateElectionDifficulty => Self::ElectionDifficulty,
            UpdateType::UpdateEuroPerEnergy => Self::EuroPerEnergy,
            UpdateType::UpdateMicroGTUPerEuro => Self::MicroCcdPerEuro,
            UpdateType::UpdateFoundationAccount => Self::FoundationAccount,
            UpdateType::UpdateMintDistribution => Self::MintDistribution,
            UpdateType::UpdateTransactionFeeDistribution => Self::TransactionFeeDistribution,
            UpdateType::UpdateGASRewards | UpdateType::UpdateGASRewardsCPV2 => Self::GasRewards,
            UpdateType::UpdatePoolParameters => Self::PoolParameters,
            UpdateType::UpdateAddAnonymityRevoker => Self::AddAnonymityRevoker,
            UpdateType::UpdateAddIdentityProvider => Self::AddIdentityProvider,
            UpdateType::UpdateCooldownParameters => Self::CooldownParameters,
            UpdateType::UpdateTimeParameters => Self::TimeParameters,
            UpdateType::UpdateTimeoutParameters => Self::TimeoutParameters,
            UpdateType::UpdateMinBlockTime => Self::MinBlockTime,
            UpdateType::UpdateBlockEnergyLimit => Self::BlockEnergyLimit,
            UpdateType::UpdateFinalizationCommitteeParameters => {
                Self::FinalizationCommitteeParameters
            }
            UpdateType::UpdateValidatorScoreParameters => Self::ValidatorScoreParameters,
            UpdateType::UpdateCreatePLT => Self::ProtocolLevelTokens,
        }
    }

    fn next_sequence_number(self, next: &NextUpdateSequenceNumbers) -> UpdateSequenceNumber {
        match self {
            GovernanceSequenceQueue::RootKeys => next.root_keys,
            GovernanceSequenceQueue::Level1Keys => next.level_1_keys,
            GovernanceSequenceQueue::Level2Keys => next.level_2_keys,
            GovernanceSequenceQueue::Protocol => next.protocol,
            GovernanceSequenceQueue::ElectionDifficulty => next.election_difficulty,
            GovernanceSequenceQueue::EuroPerEnergy => next.euro_per_energy,
            GovernanceSequenceQueue::MicroCcdPerEuro => next.micro_ccd_per_euro,
            GovernanceSequenceQueue::FoundationAccount => next.foundation_account,
            GovernanceSequenceQueue::MintDistribution => next.mint_distribution,
            GovernanceSequenceQueue::TransactionFeeDistribution => {
                next.transaction_fee_distribution
            }
            GovernanceSequenceQueue::GasRewards => next.gas_rewards,
            GovernanceSequenceQueue::PoolParameters => next.pool_parameters,
            GovernanceSequenceQueue::AddAnonymityRevoker => next.add_anonymity_revoker,
            GovernanceSequenceQueue::AddIdentityProvider => next.add_identity_provider,
            GovernanceSequenceQueue::CooldownParameters => next.cooldown_parameters,
            GovernanceSequenceQueue::TimeParameters => next.time_parameters,
            GovernanceSequenceQueue::TimeoutParameters => next.timeout_parameters,
            GovernanceSequenceQueue::MinBlockTime => next.min_block_time,
            GovernanceSequenceQueue::BlockEnergyLimit => next.block_energy_limit,
            GovernanceSequenceQueue::FinalizationCommitteeParameters => {
                next.finalization_committee_parameters
            }
            GovernanceSequenceQueue::ValidatorScoreParameters => next.validator_score_parameters,
            GovernanceSequenceQueue::ProtocolLevelTokens => next.protocol_level_tokens,
        }
    }
}

impl GovernanceAuthFamily {
    fn label(self) -> &'static str {
        match self {
            GovernanceAuthFamily::Root => "root",
            GovernanceAuthFamily::Level1 => "level1",
            GovernanceAuthFamily::Level2(capability) => capability.label(),
        }
    }

    fn default_sequence_queue(self) -> GovernanceSequenceQueue {
        match self {
            GovernanceAuthFamily::Root => GovernanceSequenceQueue::RootKeys,
            GovernanceAuthFamily::Level1 => GovernanceSequenceQueue::Level1Keys,
            GovernanceAuthFamily::Level2(GovernanceCapability::AddAr) => {
                GovernanceSequenceQueue::AddAnonymityRevoker
            }
            GovernanceAuthFamily::Level2(GovernanceCapability::AddIp) => {
                GovernanceSequenceQueue::AddIdentityProvider
            }
            GovernanceAuthFamily::Level2(GovernanceCapability::CcdEuro) => {
                GovernanceSequenceQueue::MicroCcdPerEuro
            }
            GovernanceAuthFamily::Level2(GovernanceCapability::Consensus) => {
                GovernanceSequenceQueue::TimeoutParameters
            }
            GovernanceAuthFamily::Level2(GovernanceCapability::Cooldown) => {
                GovernanceSequenceQueue::CooldownParameters
            }
            GovernanceAuthFamily::Level2(GovernanceCapability::CreatePlt) => {
                GovernanceSequenceQueue::ProtocolLevelTokens
            }
            GovernanceAuthFamily::Level2(GovernanceCapability::Emergency) => {
                GovernanceSequenceQueue::Protocol
            }
            GovernanceAuthFamily::Level2(GovernanceCapability::EuroEnergy) => {
                GovernanceSequenceQueue::EuroPerEnergy
            }
            GovernanceAuthFamily::Level2(GovernanceCapability::Foundation) => {
                GovernanceSequenceQueue::FoundationAccount
            }
            GovernanceAuthFamily::Level2(GovernanceCapability::GasRewards) => {
                GovernanceSequenceQueue::GasRewards
            }
            GovernanceAuthFamily::Level2(GovernanceCapability::Mint) => {
                GovernanceSequenceQueue::MintDistribution
            }
            GovernanceAuthFamily::Level2(GovernanceCapability::Pool) => {
                GovernanceSequenceQueue::PoolParameters
            }
            GovernanceAuthFamily::Level2(GovernanceCapability::Protocol) => {
                GovernanceSequenceQueue::Protocol
            }
            GovernanceAuthFamily::Level2(GovernanceCapability::Time) => {
                GovernanceSequenceQueue::TimeParameters
            }
            GovernanceAuthFamily::Level2(GovernanceCapability::TxFees) => {
                GovernanceSequenceQueue::TransactionFeeDistribution
            }
        }
    }
}

fn resolve_update_payload(
    input: GovernanceUpdatePayloadInput,
    args: &GovernanceUpdateArgs,
) -> Result<ResolvedGovernanceUpdatePayload> {
    match input {
        GovernanceUpdatePayloadInput::Json(raw_json) => {
            let normalized_json = normalize_create_plt_json_initialization_parameters(&raw_json)?;
            let payload: UpdatePayload = serde_json::from_str(&normalized_json)
                .context("failed to parse governance update JSON payload")?;
            let update_type = payload.update_type();
            let auth_family = auth_family_for_update_type(update_type);
            let sequence_queue = GovernanceSequenceQueue::from_update_type(update_type);
            Ok(ResolvedGovernanceUpdatePayload::Known {
                payload,
                auth_family,
                sequence_queue,
            })
        }
        GovernanceUpdatePayloadInput::Serialized(bytes) => {
            let encoded = EncodedUpdatePayload::from(bytes.clone());
            match encoded.decode() {
                Ok(payload) => {
                    let update_type = payload.update_type();
                    let auth_family = auth_family_for_update_type(update_type);
                    let sequence_queue = GovernanceSequenceQueue::from_update_type(update_type);
                    Ok(ResolvedGovernanceUpdatePayload::Known {
                        payload,
                        auth_family,
                        sequence_queue,
                    })
                }
                Err(err) if args.blind => {
                    cliclack::log::warning(format!(
                        "Blind signing serialized governance update payload. The wallet could not decode this payload: {err}"
                    ))?;
                    let auth_family_hint = parse_optional_auth_family(args.sign_as.as_deref())?;
                    let sequence_queue_hint =
                        auth_family_hint.map(GovernanceAuthFamily::default_sequence_queue);
                    Ok(ResolvedGovernanceUpdatePayload::Blind {
                        bytes,
                        auth_family_hint,
                        sequence_queue_hint,
                    })
                }
                Err(err) => bail!(
                    "failed to decode serialized governance update payload: {err}. Re-run with `--blind` to sign an unknown serialized payload."
                ),
            }
        }
    }
}

fn normalize_create_plt_json_initialization_parameters(raw_json: &str) -> Result<String> {
    let mut value: serde_json::Value =
        serde_json::from_str(raw_json).context("failed to parse governance update JSON payload")?;
    let mut converted = false;
    if let Some(object) = value.as_object_mut() {
        if object.get("updateType").and_then(serde_json::Value::as_str) == Some("createPlt") {
            if let Some(create_plt) = object
                .get_mut("update")
                .and_then(serde_json::Value::as_object_mut)
            {
                converted = convert_create_plt_initialization_parameters(create_plt)?;
            }
        } else if let Some(create_plt) = object
            .get_mut("createPlt")
            .and_then(serde_json::Value::as_object_mut)
        {
            converted = convert_create_plt_initialization_parameters(create_plt)?;
        }
    }
    if converted {
        serde_json::to_string(&value)
            .context("failed to serialize normalized Create PLT JSON payload")
    } else {
        Ok(raw_json.to_owned())
    }
}

fn convert_create_plt_initialization_parameters(
    create_plt: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<bool> {
    if let Some(json_value) = create_plt.remove("initializationParametersJson") {
        let hex = json_value_to_cbor_hex(&json_value)?;
        create_plt.insert(
            "initializationParameters".to_owned(),
            serde_json::Value::String(hex),
        );
        return Ok(true);
    }
    let Some(initialization_parameters) = create_plt.get_mut("initializationParameters") else {
        return Ok(false);
    };
    if initialization_parameters.is_string() {
        return Ok(false);
    }
    let hex = json_value_to_cbor_hex(initialization_parameters)?;
    *initialization_parameters = serde_json::Value::String(hex);
    Ok(true)
}

fn json_value_to_cbor_hex(value: &serde_json::Value) -> Result<String> {
    let bytes = create_plt_initialization_parameters_json_to_cbor(value)?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn create_plt_initialization_parameters_json_to_cbor(value: &serde_json::Value) -> Result<Vec<u8>> {
    let object = value
        .as_object()
        .context("Create PLT initialization parameters JSON must be an object")?;
    let optional = |key: &str| object.get(key).cloned().unwrap_or(serde_json::Value::Null);
    let parameters = TokenModuleInitializationParameters {
        name: serde_json::from_value(optional("name"))
            .context("failed to parse Create PLT initialization field 'name'")?,
        metadata: serde_json::from_value::<Option<MetadataUrl>>(optional("metadata"))
            .context("failed to parse Create PLT initialization field 'metadata'")?,
        governance_account: serde_json::from_value::<Option<CborHolderAccount>>(optional(
            "governanceAccount",
        ))
        .context("failed to parse Create PLT initialization field 'governanceAccount'")?,
        allow_list: serde_json::from_value(optional("allowList"))
            .context("failed to parse Create PLT initialization field 'allowList'")?,
        deny_list: serde_json::from_value(optional("denyList"))
            .context("failed to parse Create PLT initialization field 'denyList'")?,
        initial_supply: serde_json::from_value::<Option<TokenAmount>>(optional("initialSupply"))
            .context("failed to parse Create PLT initialization field 'initialSupply'")?,
        mintable: serde_json::from_value(optional("mintable"))
            .context("failed to parse Create PLT initialization field 'mintable'")?,
        burnable: serde_json::from_value(optional("burnable"))
            .context("failed to parse Create PLT initialization field 'burnable'")?,
    };
    Ok(cbor::cbor_encode(&parameters))
}

fn auth_family_for_update_type(update_type: UpdateType) -> GovernanceAuthFamily {
    match update_type {
        UpdateType::UpdateRootKeys => GovernanceAuthFamily::Root,
        UpdateType::UpdateLevel1Keys | UpdateType::UpdateLevel2Keys => GovernanceAuthFamily::Level1,
        UpdateType::UpdateProtocol => GovernanceAuthFamily::Level2(GovernanceCapability::Protocol),
        UpdateType::UpdateElectionDifficulty
        | UpdateType::UpdateEuroPerEnergy
        | UpdateType::UpdateMicroGTUPerEuro => {
            GovernanceAuthFamily::Level2(GovernanceCapability::EuroEnergy)
        }
        UpdateType::UpdateFoundationAccount => {
            GovernanceAuthFamily::Level2(GovernanceCapability::Foundation)
        }
        UpdateType::UpdateMintDistribution => {
            GovernanceAuthFamily::Level2(GovernanceCapability::Mint)
        }
        UpdateType::UpdateTransactionFeeDistribution => {
            GovernanceAuthFamily::Level2(GovernanceCapability::TxFees)
        }
        UpdateType::UpdateGASRewards | UpdateType::UpdateGASRewardsCPV2 => {
            GovernanceAuthFamily::Level2(GovernanceCapability::GasRewards)
        }
        UpdateType::UpdatePoolParameters => {
            GovernanceAuthFamily::Level2(GovernanceCapability::Pool)
        }
        UpdateType::UpdateAddAnonymityRevoker => {
            GovernanceAuthFamily::Level2(GovernanceCapability::AddAr)
        }
        UpdateType::UpdateAddIdentityProvider => {
            GovernanceAuthFamily::Level2(GovernanceCapability::AddIp)
        }
        UpdateType::UpdateCooldownParameters => {
            GovernanceAuthFamily::Level2(GovernanceCapability::Cooldown)
        }
        UpdateType::UpdateTimeParameters => {
            GovernanceAuthFamily::Level2(GovernanceCapability::Time)
        }
        UpdateType::UpdateTimeoutParameters
        | UpdateType::UpdateMinBlockTime
        | UpdateType::UpdateBlockEnergyLimit
        | UpdateType::UpdateFinalizationCommitteeParameters
        | UpdateType::UpdateValidatorScoreParameters => {
            GovernanceAuthFamily::Level2(GovernanceCapability::Consensus)
        }
        UpdateType::UpdateCreatePLT => {
            GovernanceAuthFamily::Level2(GovernanceCapability::CreatePlt)
        }
    }
}

async fn resolve_update_chain_context(
    node_endpoint: &str,
    endpoint_label: &str,
    payload: &ResolvedGovernanceUpdatePayload,
    explicit_sequence_number: Option<u64>,
) -> Result<GovernanceUpdateChainContext> {
    let chain_parameters = fetch_chain_parameters(node_endpoint, endpoint_label).await?;
    let sequence_queue = payload.sequence_queue_hint();
    let (sequence_number, sequence_number_source) = match explicit_sequence_number {
        Some(number) => (
            Some(UpdateSequenceNumber { number }),
            Some(ResolutionSource::Explicit),
        ),
        None => match sequence_queue {
            Some(queue) => {
                let next =
                    fetch_next_update_sequence_numbers(node_endpoint, endpoint_label).await?;
                (
                    Some(queue.next_sequence_number(&next)),
                    Some(ResolutionSource::Inferred),
                )
            }
            None => (None, None),
        },
    };
    Ok(GovernanceUpdateChainContext {
        chain_parameters,
        sequence_number,
        sequence_number_source,
    })
}

fn build_signed_update_instruction(
    payload: &ResolvedGovernanceUpdatePayload,
    chain_context: &GovernanceUpdateChainContext,
    signers: &[governance::DecryptedGovernanceKey],
    timing: GovernanceUpdateTiming,
) -> Result<BlockItem<Payload>> {
    let sequence_number = chain_context
        .sequence_number
        .context("governance update sequence number could not be resolved")?;
    let signer = signer_map_for_payload(payload, &chain_context.chain_parameters, signers)?;
    let effective_time = TransactionTime::from_seconds(timing.effective_time_seconds);
    let timeout = TransactionTime::from_seconds(timing.timeout_seconds);
    let update_instruction = match payload {
        ResolvedGovernanceUpdatePayload::Known { payload, .. } => update_instruction::update(
            &signer,
            sequence_number,
            effective_time,
            timeout,
            payload.clone(),
        ),
        ResolvedGovernanceUpdatePayload::Blind { bytes, .. } => {
            build_blind_update_instruction(&signer, sequence_number, effective_time, timeout, bytes)
        }
    };
    Ok(update_instruction.into())
}

fn build_blind_update_instruction(
    signer: &BTreeMap<UpdateKeysIndex, UpdateKeyPair>,
    seq_number: UpdateSequenceNumber,
    effective_time: TransactionTime,
    timeout: TransactionTime,
    payload_bytes: &[u8],
) -> UpdateInstruction {
    let payload = EncodedUpdatePayload::from(payload_bytes.to_vec());
    let header = UpdateHeader {
        seq_number,
        effective_time,
        timeout,
        payload_size: payload.size(),
    };
    let signatures = signer.sign_update_hash(&compute_update_sign_hash(&header, &payload));
    UpdateInstruction {
        header,
        payload,
        signatures,
    }
}

fn compute_update_sign_hash(
    header: &UpdateHeader,
    payload: &EncodedUpdatePayload,
) -> concordium_rust_sdk::base::hashes::UpdateSignHash {
    let mut hasher = sha2::Sha256::new();
    header.serial(&mut hasher);
    hasher
        .write_all(payload.as_ref())
        .expect("writing to hasher does not fail");
    <[u8; 32]>::from(hasher.finalize()).into()
}

async fn submit_governance_update(
    node_endpoint: &str,
    endpoint_label: &str,
    block_item: &BlockItem<Payload>,
) -> Result<concordium_rust_sdk::base::hashes::TransactionHash> {
    let mut client = connect_governance_node(node_endpoint, endpoint_label).await?;
    client
        .send_block_item(block_item)
        .await
        .with_context(|| format!("failed to submit governance update to node at {endpoint_label}"))
}

async fn wait_for_governance_update_finalization(
    node_endpoint: &str,
    endpoint_label: &str,
    transaction_hash: &concordium_rust_sdk::base::hashes::TransactionHash,
) -> Result<()> {
    let mut client = connect_governance_node(node_endpoint, endpoint_label).await?;
    let spin = spinner();
    spin.start("Waiting for governance update finalization...");
    let result = tokio::time::timeout(
        tokio::time::Duration::from_secs(30),
        client.wait_until_finalized(transaction_hash),
    )
    .await;

    spin.clear();

    match result {
        Ok(Ok((block_hash, _summary))) => {
            println!("Governance update finalized in block {block_hash}.");
            Ok(())
        }
        Ok(Err(err)) => Err(err).with_context(|| {
            format!("failed while waiting for governance update finalization at {endpoint_label}")
        }),
        Err(_) => {
            bail!(
                "timed out after 30 seconds while waiting for governance update finalization at {endpoint_label}"
            )
        }
    }
}

fn signer_map_for_payload(
    payload: &ResolvedGovernanceUpdatePayload,
    chain_parameters: &ChainParameters,
    signers: &[governance::DecryptedGovernanceKey],
) -> Result<BTreeMap<UpdateKeysIndex, UpdateKeyPair>> {
    match payload.auth_family_hint() {
        Some(family) => signer_map_for_auth_family(chain_parameters, signers, family),
        None => signer_map_without_auth_family(chain_parameters, signers),
    }
}

fn signer_map_for_auth_family(
    chain_parameters: &ChainParameters,
    signers: &[governance::DecryptedGovernanceKey],
    family: GovernanceAuthFamily,
) -> Result<BTreeMap<UpdateKeysIndex, UpdateKeyPair>> {
    let (keys, authorized_indices, threshold) = match family {
        GovernanceAuthFamily::Root => {
            let keys = chain_parameters
                .keys
                .root_keys
                .as_ref()
                .context("root governance keys are not present in chain parameters")?;
            let authorized = (0..keys.keys.len())
                .map(|index| UpdateKeysIndex {
                    index: index as u16,
                })
                .collect::<BTreeSet<_>>();
            (
                &keys.keys,
                authorized,
                usize::from(u16::from(keys.threshold)),
            )
        }
        GovernanceAuthFamily::Level1 => {
            let keys = chain_parameters
                .keys
                .level_1_keys
                .as_ref()
                .context("level 1 governance keys are not present in chain parameters")?;
            let authorized = (0..keys.keys.len())
                .map(|index| UpdateKeysIndex {
                    index: index as u16,
                })
                .collect::<BTreeSet<_>>();
            (
                &keys.keys,
                authorized,
                usize::from(u16::from(keys.threshold)),
            )
        }
        GovernanceAuthFamily::Level2(capability) => {
            let level2 = chain_parameters
                .keys
                .level_2_keys
                .as_ref()
                .context("level 2 governance keys are not present in chain parameters")?;
            let access =
                level2_access_structure_for_capability(level2, capability).with_context(|| {
                    format!(
                        "authorization structure for '{}' is not present",
                        capability.label()
                    )
                })?;
            (
                &level2.keys,
                access.authorized_keys.clone(),
                usize::from(u16::from(access.threshold)),
            )
        }
    };
    let signer = signer_map_from_key_list(keys, Some(&authorized_indices), signers)?;
    if signer.len() < threshold {
        bail!(
            "selected {} governance signer(s), but '{}' updates require threshold {}",
            signer.len(),
            family.label(),
            threshold
        );
    }
    Ok(signer)
}

fn signer_map_without_auth_family(
    chain_parameters: &ChainParameters,
    signers: &[governance::DecryptedGovernanceKey],
) -> Result<BTreeMap<UpdateKeysIndex, UpdateKeyPair>> {
    let mut signer = BTreeMap::new();
    for selected in signers {
        let public_key = UpdatePublicKey::from(&selected.key_pair);
        let mut matches = Vec::new();
        if let Some(root) = chain_parameters.keys.root_keys.as_ref() {
            matches.extend(indices_for_public_key(&root.keys, &public_key));
        }
        if let Some(level1) = chain_parameters.keys.level_1_keys.as_ref() {
            matches.extend(indices_for_public_key(&level1.keys, &public_key));
        }
        if let Some(level2) = chain_parameters.keys.level_2_keys.as_ref() {
            matches.extend(indices_for_public_key(&level2.keys, &public_key));
        }
        matches.sort();
        matches.dedup();
        match matches.as_slice() {
            [index] => {
                if signer.insert(*index, selected.key_pair.clone()).is_some() {
                    bail!("duplicate governance key index {index} selected");
                }
            }
            [] => bail!(
                "governance key '{}' is not currently authorized on chain",
                governance::public_key_hex(&selected.public_key)
            ),
            _ => bail!(
                "governance key '{}' maps to multiple governance key indices; provide `--sign-as`",
                governance::public_key_hex(&selected.public_key)
            ),
        }
    }
    Ok(signer)
}

fn signer_map_from_key_list(
    keys: &[UpdatePublicKey],
    authorized_indices: Option<&BTreeSet<UpdateKeysIndex>>,
    signers: &[governance::DecryptedGovernanceKey],
) -> Result<BTreeMap<UpdateKeysIndex, UpdateKeyPair>> {
    let mut signer = BTreeMap::new();
    for selected in signers {
        let public_key = UpdatePublicKey::from(&selected.key_pair);
        let Some(position) = keys.iter().position(|key| key == &public_key) else {
            bail!(
                "governance key '{}' is not currently authorized for the selected update family",
                governance::public_key_hex(&selected.public_key)
            );
        };
        let index = UpdateKeysIndex {
            index: position as u16,
        };
        if authorized_indices.is_some_and(|authorized| !authorized.contains(&index)) {
            bail!(
                "governance key '{}' is not authorized for the selected update family",
                governance::public_key_hex(&selected.public_key)
            );
        }
        if signer.insert(index, selected.key_pair.clone()).is_some() {
            bail!("duplicate governance key index {index} selected");
        }
    }
    Ok(signer)
}

fn indices_for_public_key(
    keys: &[UpdatePublicKey],
    public_key: &UpdatePublicKey,
) -> Vec<UpdateKeysIndex> {
    keys.iter()
        .enumerate()
        .filter_map(|(index, key)| {
            (key == public_key).then_some(UpdateKeysIndex {
                index: index as u16,
            })
        })
        .collect()
}

fn payload_context_lines(
    payload: &ResolvedGovernanceUpdatePayload,
    chain_context: &GovernanceUpdateChainContext,
) -> Vec<ContextLine<'static>> {
    let mut lines = vec![ContextLine {
        label: "payload:",
        value: payload.context_label(),
        source: ResolutionSource::Inferred,
    }];
    if let (Some(sequence_number), Some(source)) = (
        chain_context.sequence_number,
        chain_context.sequence_number_source,
    ) {
        lines.push(ContextLine {
            label: "sequence:",
            value: sequence_number.to_string(),
            source,
        });
    }
    lines
}

impl ResolvedGovernanceUpdatePayload {
    fn sequence_queue_hint(&self) -> Option<GovernanceSequenceQueue> {
        match self {
            ResolvedGovernanceUpdatePayload::Known { sequence_queue, .. } => Some(*sequence_queue),
            ResolvedGovernanceUpdatePayload::Blind {
                sequence_queue_hint,
                ..
            } => *sequence_queue_hint,
        }
    }

    fn auth_family_hint(&self) -> Option<GovernanceAuthFamily> {
        match self {
            ResolvedGovernanceUpdatePayload::Known { auth_family, .. } => Some(*auth_family),
            ResolvedGovernanceUpdatePayload::Blind {
                auth_family_hint, ..
            } => *auth_family_hint,
        }
    }

    fn context_label(&self) -> String {
        match self {
            ResolvedGovernanceUpdatePayload::Known { payload, .. } => {
                format!("{:?}", payload.update_type())
            }
            ResolvedGovernanceUpdatePayload::Blind {
                auth_family_hint,
                sequence_queue_hint,
                ..
            } => match (auth_family_hint, sequence_queue_hint) {
                (Some(auth_family), Some(sequence_queue)) => format!(
                    "blind serialized (auth: {}, queue: {})",
                    auth_family.label(),
                    sequence_queue.label()
                ),
                _ => "blind serialized".to_owned(),
            },
        }
    }
}

fn resolve_update_signers(
    args: &GovernanceUpdateArgs,
    decrypted: &[governance::DecryptedGovernanceKey],
    chain_context: &GovernanceUpdateChainContext,
    payload: &ResolvedGovernanceUpdatePayload,
) -> Result<Vec<governance::DecryptedGovernanceKey>> {
    if decrypted.is_empty() {
        bail!("no governance keys are stored for the selected network");
    }
    if !args.keys.is_empty() {
        return args
            .keys
            .iter()
            .map(|verify_key| {
                find_decrypted_key_by_verify_key(decrypted, verify_key)
                    .with_context(|| format!("governance key '{verify_key}' is not stored locally"))
            })
            .collect();
    }
    if args.non_interactive {
        bail!("at least one `--key <VERIFY_KEY>` must be provided in --non-interactive mode");
    }

    let auth_family = payload.auth_family_hint();
    let entries =
        governance_signer_entries(decrypted, &chain_context.chain_parameters, auth_family);
    let threshold = auth_family
        .and_then(|family| threshold_for_auth_family(&chain_context.chain_parameters, family));
    let initial = threshold
        .map(|threshold| {
            entries
                .iter()
                .filter(|entry| entry.authorized_for_update)
                .take(threshold)
                .map(|entry| entry.list_entry.verify_key.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let items = entries
        .iter()
        .map(|entry| FuzzySelectItem {
            value: entry.list_entry.verify_key.clone(),
            text: render_governance_list_row(&entry.list_entry, true),
        })
        .collect::<Vec<_>>();
    let selected = fuzzy_multiselect_or_single_with_initial(
        "Select governance update signers",
        &items,
        &initial,
    )?;
    if selected.is_empty() {
        bail!("at least one governance signer must be selected");
    }
    selected
        .iter()
        .map(|verify_key| {
            find_decrypted_key_by_verify_key(decrypted, verify_key).with_context(|| {
                format!("selected governance key '{verify_key}' is not stored locally")
            })
        })
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GovernanceSignerEntry {
    list_entry: GovernanceListEntry,
    authorized_for_update: bool,
}

fn governance_signer_entries(
    decrypted: &[governance::DecryptedGovernanceKey],
    chain_parameters: &ChainParameters,
    auth_family: Option<GovernanceAuthFamily>,
) -> Vec<GovernanceSignerEntry> {
    let mut entries = match_governance_keys(decrypted, chain_parameters)
        .into_iter()
        .map(|list_entry| GovernanceSignerEntry {
            authorized_for_update: auth_family
                .is_some_and(|family| list_entry_authorized_for_family(&list_entry, family)),
            list_entry,
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| {
        b.authorized_for_update
            .cmp(&a.authorized_for_update)
            .then_with(|| {
                a.list_entry
                    .authorization
                    .sort_rank()
                    .cmp(&b.list_entry.authorization.sort_rank())
            })
            .then_with(|| a.list_entry.verify_key.cmp(&b.list_entry.verify_key))
    });
    entries
}

fn list_entry_authorized_for_family(
    entry: &GovernanceListEntry,
    family: GovernanceAuthFamily,
) -> bool {
    match family {
        GovernanceAuthFamily::Root => entry.authorization == GovernanceAuthorization::Root,
        GovernanceAuthFamily::Level1 => entry.authorization == GovernanceAuthorization::Level1,
        GovernanceAuthFamily::Level2(capability) => {
            entry.authorization == GovernanceAuthorization::Level2
                && entry.capabilities.contains(&capability)
        }
    }
}

fn threshold_for_auth_family(
    chain_parameters: &ChainParameters,
    family: GovernanceAuthFamily,
) -> Option<usize> {
    match family {
        GovernanceAuthFamily::Root => chain_parameters
            .keys
            .root_keys
            .as_ref()
            .map(|keys| usize::from(u16::from(keys.threshold))),
        GovernanceAuthFamily::Level1 => chain_parameters
            .keys
            .level_1_keys
            .as_ref()
            .map(|keys| usize::from(u16::from(keys.threshold))),
        GovernanceAuthFamily::Level2(capability) => chain_parameters
            .keys
            .level_2_keys
            .as_ref()
            .and_then(|level2| level2_access_structure_for_capability(level2, capability))
            .map(|access| usize::from(u16::from(access.threshold))),
    }
}

fn find_decrypted_key_by_verify_key(
    keys: &[governance::DecryptedGovernanceKey],
    verify_key: &str,
) -> Option<governance::DecryptedGovernanceKey> {
    let normalized = verify_key.trim().to_ascii_lowercase();
    keys.iter()
        .find(|key| governance::public_key_hex(&key.public_key) == normalized)
        .cloned()
}

fn validate_blind_signing_context(
    payload: &ResolvedGovernanceUpdatePayload,
    args: &GovernanceUpdateArgs,
) -> Result<()> {
    let ResolvedGovernanceUpdatePayload::Blind {
        auth_family_hint, ..
    } = payload
    else {
        return Ok(());
    };

    cliclack::log::warning(
        "Blind signing means this wallet cannot show or verify the semantics of the update payload.",
    )?;
    cliclack::log::warning(
        "Only continue if the serialized payload was produced by trusted tooling and independently reviewed.",
    )?;

    if auth_family_hint.is_none() {
        if args.keys.is_empty() {
            bail!(
                "blind signing without `--sign-as` requires explicit `--key <VERIFY_KEY>` signer selection"
            );
        }
        if args.sequence_number.is_none() {
            bail!("blind signing without `--sign-as` requires explicit `--sequence-number <N>`");
        }
    }
    Ok(())
}

fn parse_optional_auth_family(value: Option<&str>) -> Result<Option<GovernanceAuthFamily>> {
    value.map(parse_auth_family).transpose()
}

fn parse_auth_family(value: &str) -> Result<GovernanceAuthFamily> {
    let normalized = value.trim().to_ascii_lowercase().replace(['_', ' '], "-");
    match normalized.as_str() {
        "root" => Ok(GovernanceAuthFamily::Root),
        "level1" | "level-1" | "l1" => Ok(GovernanceAuthFamily::Level1),
        "protocol" => Ok(GovernanceAuthFamily::Level2(GovernanceCapability::Protocol)),
        "emergency" => Ok(GovernanceAuthFamily::Level2(
            GovernanceCapability::Emergency,
        )),
        "consensus" => Ok(GovernanceAuthFamily::Level2(
            GovernanceCapability::Consensus,
        )),
        "euro-energy" | "euro-per-energy" => Ok(GovernanceAuthFamily::Level2(
            GovernanceCapability::EuroEnergy,
        )),
        "ccd-euro" | "micro-ccd-per-euro" => {
            Ok(GovernanceAuthFamily::Level2(GovernanceCapability::CcdEuro))
        }
        "foundation" | "foundation-account" => Ok(GovernanceAuthFamily::Level2(
            GovernanceCapability::Foundation,
        )),
        "mint" | "mint-distribution" => {
            Ok(GovernanceAuthFamily::Level2(GovernanceCapability::Mint))
        }
        "tx-fees" | "transaction-fee-distribution" => {
            Ok(GovernanceAuthFamily::Level2(GovernanceCapability::TxFees))
        }
        "gas-rewards" => Ok(GovernanceAuthFamily::Level2(
            GovernanceCapability::GasRewards,
        )),
        "pool" | "pool-parameters" => Ok(GovernanceAuthFamily::Level2(GovernanceCapability::Pool)),
        "add-ar" | "add-anonymity-revoker" => {
            Ok(GovernanceAuthFamily::Level2(GovernanceCapability::AddAr))
        }
        "add-ip" | "add-identity-provider" => {
            Ok(GovernanceAuthFamily::Level2(GovernanceCapability::AddIp))
        }
        "cooldown" | "cooldown-parameters" => {
            Ok(GovernanceAuthFamily::Level2(GovernanceCapability::Cooldown))
        }
        "time" | "time-parameters" => Ok(GovernanceAuthFamily::Level2(GovernanceCapability::Time)),
        "create-plt" | "protocol-level-token" | "protocol-level-tokens" => Ok(
            GovernanceAuthFamily::Level2(GovernanceCapability::CreatePlt),
        ),
        other => bail!("unknown governance authorization family '{other}'"),
    }
}

fn resolve_update_payload_input(
    args: &GovernanceUpdateArgs,
) -> Result<GovernanceUpdatePayloadInput> {
    match (&args.json, &args.serialized) {
        (Some(json), None) => {
            let raw_json = match json {
                Some(path) => fs::read_to_string(path).with_context(|| {
                    format!(
                        "failed to read governance update JSON file {}",
                        path.display()
                    )
                })?,
                None if args.non_interactive => {
                    bail!(
                        "JSON file must be provided with `--json <FILE>` in --non-interactive mode"
                    )
                }
                None => input("Paste governance update JSON:").interact()?,
            };
            if raw_json.trim().is_empty() {
                bail!("governance update JSON payload cannot be empty");
            }
            Ok(GovernanceUpdatePayloadInput::Json(raw_json))
        }
        (None, Some(serialized)) => {
            let raw_hex = match serialized {
                Some(hex) => hex.clone(),
                None if args.non_interactive => {
                    bail!(
                        "serialized payload must be provided with `--serialized <HEX>` in --non-interactive mode"
                    )
                }
                None => input("Paste serialized governance update hex:").interact()?,
            };
            let bytes = decode_hex_payload(&raw_hex)?;
            if bytes.is_empty() {
                bail!("serialized governance update payload cannot be empty");
            }
            Ok(GovernanceUpdatePayloadInput::Serialized(bytes))
        }
        (None, None) if args.non_interactive => {
            bail!("provide either `--json` or `--serialized` in --non-interactive mode")
        }
        (None, None) => {
            let mode: String = input("Governance update payload mode (`json` or `serialized`):")
                .default_input("json")
                .interact()?;
            match mode.trim().to_ascii_lowercase().as_str() {
                "json" => {
                    let raw_json: String = input("Paste governance update JSON:").interact()?;
                    if raw_json.trim().is_empty() {
                        bail!("governance update JSON payload cannot be empty");
                    }
                    Ok(GovernanceUpdatePayloadInput::Json(raw_json))
                }
                "serialized" | "hex" => {
                    let raw_hex: String =
                        input("Paste serialized governance update hex:").interact()?;
                    let bytes = decode_hex_payload(&raw_hex)?;
                    if bytes.is_empty() {
                        bail!("serialized governance update payload cannot be empty");
                    }
                    Ok(GovernanceUpdatePayloadInput::Serialized(bytes))
                }
                other => bail!(
                    "unknown governance update payload mode '{other}'; use `json` or `serialized`"
                ),
            }
        }
        (Some(_), Some(_)) => unreachable!("clap enforces --json/--serialized conflict"),
    }
}

fn decode_hex_payload(raw_hex: &str) -> Result<Vec<u8>> {
    let mut cleaned = raw_hex.trim().to_ascii_lowercase();
    if let Some(stripped) = cleaned.strip_prefix("0x") {
        cleaned = stripped.to_owned();
    }
    cleaned.retain(|ch| !ch.is_ascii_whitespace() && ch != '_' && ch != '-');
    if !cleaned.len().is_multiple_of(2) {
        bail!("serialized governance update hex must contain an even number of hex digits");
    }
    (0..cleaned.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&cleaned[index..index + 2], 16)
                .with_context(|| "serialized governance update payload contains non-hex characters")
        })
        .collect()
}

fn resolve_update_timing(args: &GovernanceUpdateArgs) -> Result<GovernanceUpdateTiming> {
    let now = now_unix_seconds()?;
    let effective_time_seconds = match &args.effective_time {
        Some(value) => parse_effective_time_input(value, now)?,
        None if args.non_interactive => 0,
        None => {
            let value: String = input("Effective time:").default_input("0").interact()?;
            parse_effective_time_input(&value, now)?
        }
    };
    let default_timeout = default_timeout_seconds(effective_time_seconds, now);
    let timeout_seconds = match &args.timeout {
        Some(value) => parse_time_input(value, now)?,
        None if args.non_interactive => default_timeout,
        None => {
            let value: String = input("Timeout:")
                .default_input(&format_unix_seconds_rfc3339(default_timeout)?)
                .interact()?;
            parse_time_input(&value, now)?
        }
    };
    validate_update_timing(effective_time_seconds, timeout_seconds, now)?;
    Ok(GovernanceUpdateTiming {
        effective_time_seconds,
        timeout_seconds,
    })
}

fn parse_effective_time_input(input: &str, now: u64) -> Result<u64> {
    if input.trim() == "0" {
        return Ok(0);
    }
    parse_time_input(input, now)
}

fn parse_time_input(input: &str, now: u64) -> Result<u64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        bail!("time input cannot be empty");
    }
    if let Some(duration_seconds) = parse_relative_duration_seconds(trimmed)? {
        return now
            .checked_add(duration_seconds)
            .with_context(|| "relative time overflows unix seconds");
    }
    if let Ok(datetime) = DateTime::parse_from_rfc3339(trimmed) {
        let timestamp = datetime.with_timezone(&Utc).timestamp();
        if timestamp < 0 {
            bail!("time input must not be before the unix epoch");
        }
        return Ok(timestamp as u64);
    }
    trimmed.parse::<u64>().with_context(|| {
        format!("invalid time input '{trimmed}'; use relative duration, RFC3339, or unix seconds")
    })
}

fn parse_relative_duration_seconds(input: &str) -> Result<Option<u64>> {
    let Some(unit) = input.chars().last() else {
        return Ok(None);
    };
    let multiplier = match unit {
        's' | 'S' => 1,
        'm' | 'M' => 60,
        'h' | 'H' => 60 * 60,
        'd' | 'D' => 24 * 60 * 60,
        _ => return Ok(None),
    };
    let number = &input[..input.len() - unit.len_utf8()];
    if number.trim().is_empty() {
        bail!("relative duration '{input}' is missing a number");
    }
    let value = number
        .trim()
        .parse::<u64>()
        .with_context(|| format!("invalid relative duration '{input}'"))?;
    value
        .checked_mul(multiplier)
        .map(Some)
        .with_context(|| format!("relative duration '{input}' overflows seconds"))
}

fn default_timeout_seconds(effective_time_seconds: u64, now: u64) -> u64 {
    if effective_time_seconds == 0 {
        now.saturating_add(5 * 60)
    } else {
        effective_time_seconds.saturating_sub(5 * 60)
    }
}

fn format_unix_seconds_rfc3339(seconds: u64) -> Result<String> {
    let seconds = i64::try_from(seconds).context("timestamp is too large to format as RFC3339")?;
    DateTime::<Utc>::from_timestamp(seconds, 0)
        .with_context(|| format!("timestamp {seconds} cannot be formatted as RFC3339"))
        .map(|datetime| datetime.to_rfc3339())
}

fn validate_update_timing(
    effective_time_seconds: u64,
    timeout_seconds: u64,
    now: u64,
) -> Result<()> {
    if timeout_seconds <= now {
        bail!("governance update timeout must be in the future");
    }
    if effective_time_seconds != 0 {
        if effective_time_seconds <= now {
            bail!("nonzero governance update effective time must be in the future");
        }
        if timeout_seconds > effective_time_seconds {
            bail!("governance update timeout must not be after the effective time");
        }
    }
    Ok(())
}

fn now_unix_seconds() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the unix epoch")?
        .as_secs())
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
    let mut client = connect_governance_node(node_endpoint, endpoint_label).await?;
    client
        .get_block_chain_parameters(&v2::BlockIdentifier::LastFinal)
        .await
        .with_context(|| format!("failed to query chain parameters from node at {endpoint_label}"))
        .map(|response| response.response)
}

async fn fetch_next_update_sequence_numbers(
    node_endpoint: &str,
    endpoint_label: &str,
) -> Result<NextUpdateSequenceNumbers> {
    let mut client = connect_governance_node(node_endpoint, endpoint_label).await?;
    client
        .get_next_update_sequence_numbers(v2::BlockIdentifier::LastFinal)
        .await
        .with_context(|| {
            format!("failed to query next update sequence numbers from node at {endpoint_label}")
        })
        .map(|response| response.response)
}

async fn connect_governance_node(node_endpoint: &str, endpoint_label: &str) -> Result<v2::Client> {
    let endpoint: v2::Endpoint = ccd_wallet_core::config::normalize_url_string(node_endpoint)
        .parse()
        .with_context(|| format!("invalid node endpoint: {node_endpoint}"))?;
    config::connect_v2_client(endpoint)
        .await
        .with_context(|| format!("failed to connect to Concordium node at {endpoint_label}"))
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

fn level2_access_structure_for_capability(
    level2: &concordium_rust_sdk::types::chain_parameters::Level2Keys,
    capability: GovernanceCapability,
) -> Option<&concordium_rust_sdk::base::updates::AccessStructure> {
    match capability {
        GovernanceCapability::AddAr => level2.add_anonymity_revoker.as_ref(),
        GovernanceCapability::AddIp => level2.add_identity_provider.as_ref(),
        GovernanceCapability::CcdEuro => level2.micro_ccd_per_euro.as_ref(),
        GovernanceCapability::Consensus => level2.consensus.as_ref(),
        GovernanceCapability::Cooldown => level2.cooldown_parameters.as_ref(),
        GovernanceCapability::CreatePlt => level2.create_plt.as_ref(),
        GovernanceCapability::Emergency => level2.emergency.as_ref(),
        GovernanceCapability::EuroEnergy => level2.euro_per_energy.as_ref(),
        GovernanceCapability::Foundation => level2.foundation_account.as_ref(),
        GovernanceCapability::GasRewards => level2.param_gas_rewards.as_ref(),
        GovernanceCapability::Mint => level2.mint_distribution.as_ref(),
        GovernanceCapability::Pool => level2.pool_parameters.as_ref(),
        GovernanceCapability::Protocol => level2.protocol.as_ref(),
        GovernanceCapability::Time => level2.time_parameters.as_ref(),
        GovernanceCapability::TxFees => level2.transaction_fee_distribution.as_ref(),
    }
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

    fn update_args() -> GovernanceUpdateArgs {
        GovernanceUpdateArgs {
            json: None,
            serialized: None,
            blind: false,
            keys: Vec::new(),
            sign_as: None,
            sequence_number: None,
            effective_time: None,
            timeout: None,
            network: None,
            no_wait: false,
            non_interactive: true,
            no_defaults: false,
        }
    }

    fn protocol_payload() -> UpdatePayload {
        UpdatePayload::Protocol(concordium_rust_sdk::types::ProtocolUpdate {
            message: "test".to_owned(),
            specification_url: "https://example.com/update".to_owned(),
            specification_hash: "0000000000000000000000000000000000000000000000000000000000000000"
                .parse()
                .unwrap(),
            specification_auxiliary_data: Vec::new(),
        })
    }

    #[test]
    fn update_payload_ingestion_reads_json_files_and_serialized_hex() {
        let temp = std::env::temp_dir().join(format!("gov-update-{}.json", std::process::id()));
        std::fs::write(&temp, serde_json::to_string(&protocol_payload()).unwrap()).unwrap();
        let mut args = update_args();
        args.json = Some(Some(temp.clone()));
        let input = resolve_update_payload_input(&args).unwrap();
        assert!(matches!(input, GovernanceUpdatePayloadInput::Json(_)));
        let _ = std::fs::remove_file(temp);

        let mut args = update_args();
        args.serialized = Some(Some("0x0100".to_owned()));
        let input = resolve_update_payload_input(&args).unwrap();
        assert_eq!(input, GovernanceUpdatePayloadInput::Serialized(vec![1, 0]));
    }

    #[test]
    fn known_payload_maps_to_authorization_and_sequence_queue() {
        let args = update_args();
        let raw_json = serde_json::to_string(&protocol_payload()).unwrap();
        let resolved =
            resolve_update_payload(GovernanceUpdatePayloadInput::Json(raw_json), &args).unwrap();
        let ResolvedGovernanceUpdatePayload::Known {
            auth_family,
            sequence_queue,
            ..
        } = resolved
        else {
            panic!("expected known payload");
        };
        assert_eq!(
            auth_family,
            GovernanceAuthFamily::Level2(GovernanceCapability::Protocol)
        );
        assert_eq!(sequence_queue, GovernanceSequenceQueue::Protocol);
    }

    #[test]
    fn create_plt_json_payload_can_use_json_initialization_parameters() {
        let raw_json = serde_json::json!({
            "updateType": "createPlt",
            "update": {
                "tokenId": "TEST",
                "tokenModule": "0000000000000000000000000000000000000000000000000000000000000000",
                "decimals": 6,
                "initializationParameters": {
                    "name": "Test token",
                    "metadata": { "url": "https://example.com/token.json" }
                }
            }
        })
        .to_string();
        let normalized = normalize_create_plt_json_initialization_parameters(&raw_json).unwrap();
        let value: serde_json::Value = serde_json::from_str(&normalized).unwrap();
        let encoded = value
            .get("update")
            .and_then(|payload| payload.get("initializationParameters"))
            .and_then(serde_json::Value::as_str)
            .unwrap();
        assert!(!encoded.is_empty());
        assert!(encoded.chars().all(|ch| ch.is_ascii_hexdigit()));

        let args = update_args();
        let resolved =
            resolve_update_payload(GovernanceUpdatePayloadInput::Json(raw_json), &args).unwrap();
        assert!(matches!(
            resolved,
            ResolvedGovernanceUpdatePayload::Known {
                auth_family: GovernanceAuthFamily::Level2(GovernanceCapability::CreatePlt),
                sequence_queue: GovernanceSequenceQueue::ProtocolLevelTokens,
                ..
            }
        ));
    }

    #[test]
    fn blind_payload_requires_manual_context_without_sign_as() {
        let mut args = update_args();
        args.serialized = Some(Some("ff".to_owned()));
        args.blind = true;
        let payload =
            resolve_update_payload(GovernanceUpdatePayloadInput::Serialized(vec![0xff]), &args)
                .unwrap();
        let err = validate_blind_signing_context(&payload, &args).unwrap_err();
        assert!(err.to_string().contains("requires explicit `--key"));

        args.keys.push("abcd".to_owned());
        args.sequence_number = Some(7);
        validate_blind_signing_context(&payload, &args).unwrap();
    }

    #[test]
    fn sign_as_hint_derives_default_sequence_queue_for_blind_payloads() {
        let mut args = update_args();
        args.blind = true;
        args.sign_as = Some("create-plt".to_owned());
        let payload =
            resolve_update_payload(GovernanceUpdatePayloadInput::Serialized(vec![0xff]), &args)
                .unwrap();
        let ResolvedGovernanceUpdatePayload::Blind {
            auth_family_hint,
            sequence_queue_hint,
            ..
        } = payload
        else {
            panic!("expected blind payload");
        };
        assert_eq!(
            auth_family_hint,
            Some(GovernanceAuthFamily::Level2(
                GovernanceCapability::CreatePlt
            ))
        );
        assert_eq!(
            sequence_queue_hint,
            Some(GovernanceSequenceQueue::ProtocolLevelTokens)
        );
    }

    #[test]
    fn timing_parses_formats_defaults_and_validates_invariants() {
        let now = 1_700_000_000;
        assert_eq!(parse_effective_time_input("0", now).unwrap(), 0);
        assert_eq!(parse_time_input("5m", now).unwrap(), now + 300);
        assert_eq!(parse_time_input("1h", now).unwrap(), now + 3600);
        assert_eq!(
            parse_time_input("2025-01-01T00:00:00Z", now).unwrap(),
            1_735_689_600
        );
        assert_eq!(parse_time_input("1800000000", now).unwrap(), 1_800_000_000);
        assert_eq!(default_timeout_seconds(0, now), now + 300);
        assert_eq!(default_timeout_seconds(now + 3600, now), now + 3300);
        assert_eq!(
            format_unix_seconds_rfc3339(1_735_689_600).unwrap(),
            "2025-01-01T00:00:00+00:00"
        );
        assert!(validate_update_timing(0, now + 60, now).is_ok());
        assert!(validate_update_timing(now + 60, now + 120, now).is_err());
    }

    #[test]
    fn signer_entries_sort_authorized_keys_first_and_signer_map_uses_indices() {
        let level2_authorized = key();
        let level2_other = key();
        let local = vec![
            governance_store::DecryptedGovernanceKey {
                record: governance_store::GovernanceKeyRecord {
                    id: 1,
                    network_genesis_hash: "g".to_owned(),
                    vault_id: "v".to_owned(),
                    created_at: 0,
                    updated_at: 0,
                },
                raw_json: serde_json::to_string(&level2_other).unwrap(),
                public_key: concordium_rust_sdk::base::base::UpdatePublicKey::from(&level2_other),
                key_pair: level2_other.clone(),
            },
            governance_store::DecryptedGovernanceKey {
                record: governance_store::GovernanceKeyRecord {
                    id: 2,
                    network_genesis_hash: "g".to_owned(),
                    vault_id: "v".to_owned(),
                    created_at: 0,
                    updated_at: 0,
                },
                raw_json: serde_json::to_string(&level2_authorized).unwrap(),
                public_key: concordium_rust_sdk::base::base::UpdatePublicKey::from(
                    &level2_authorized,
                ),
                key_pair: level2_authorized.clone(),
            },
        ];
        let params = ChainParameters {
            keys: UpdateKeys {
                level_2_keys: Some(Level2Keys {
                    keys: vec![
                        concordium_rust_sdk::base::base::UpdatePublicKey::from(&level2_authorized),
                        concordium_rust_sdk::base::base::UpdatePublicKey::from(&level2_other),
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
                    create_plt: None,
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        let entries = governance_signer_entries(
            &local,
            &params,
            Some(GovernanceAuthFamily::Level2(GovernanceCapability::Protocol)),
        );
        assert!(entries[0].authorized_for_update);
        assert_eq!(
            entries[0].list_entry.verify_key,
            governance_store::public_key_hex(&local[1].public_key)
        );
        let signer = signer_map_for_auth_family(
            &params,
            &[local[1].clone()],
            GovernanceAuthFamily::Level2(GovernanceCapability::Protocol),
        )
        .unwrap();
        assert!(signer.contains_key(&UpdateKeysIndex { index: 0 }));
    }
}
