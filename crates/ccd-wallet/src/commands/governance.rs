use crate::{
    cli::{
        GovernanceKeysCommand, GovernanceKeysImportArgs, GovernanceKeysListArgs,
        GovernanceKeysRemoveArgs, GovernanceKeysSubcommand, GovernanceProposalCreateArgs,
        GovernanceProposalSignArgs, GovernanceProposalSubcommand, GovernanceProposalSubmitArgs,
        GovernanceSubcommand, GovernanceUpdateArgs,
    },
    commands::{
        input::{Defaultable, FinalizationPolicy, InputMode, Promptable},
        ui::{
            ContextLine, FuzzySelectItem, ResolutionSource, SelectItem,
            fuzzy_multiselect_or_single, fuzzy_multiselect_or_single_with_initial,
            log_resolved_context, select_or_single,
        },
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
use ccd_wallet_ledger_governance as governance_ledger;
use chrono::{DateTime, Utc};
use cliclack::{confirm, input, password, spinner};
use concordium_rust_sdk::{
    base::{
        base::{UpdateKeyPair, UpdateKeysIndex, UpdatePublicKey, UpdateSequenceNumber},
        transactions::{BlockItem, Payload, PayloadSize},
        updates::{
            EncodedUpdatePayload, UpdateHeader, UpdateInstruction, UpdateInstructionSignature,
            UpdateSigner,
        },
    },
    common::types::Signature,
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
use serde::{Deserialize, Serialize};
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
        GovernanceSubcommand::Proposal(args) => match args.command {
            GovernanceProposalSubcommand::Create(args) => create_proposal(conn, *args).await,
            GovernanceProposalSubcommand::Sign(args) => sign_proposal(conn, *args).await,
            GovernanceProposalSubcommand::Submit(args) => submit_proposal(conn, *args).await,
        },
        GovernanceSubcommand::Update(args) => update(conn, *args).await,
    }
}

#[derive(Clone, Debug)]
struct PreparedGovernanceContext {
    network: Option<String>,
    input_mode: InputMode,
    finalization: FinalizationPolicy,
}

impl PreparedGovernanceContext {
    fn from_flags(
        network: Option<String>,
        non_interactive: bool,
        no_defaults: bool,
        no_wait: bool,
    ) -> Self {
        Self {
            network,
            input_mode: InputMode::from_flags(non_interactive, no_defaults),
            finalization: FinalizationPolicy::from_no_wait(no_wait),
        }
    }

    async fn resolve_network(
        &self,
        conn: &mut Connection,
    ) -> Result<(String, NetworkEntry, String, ResolutionSource)> {
        resolve_governance_network(
            conn,
            self.network.as_deref(),
            self.input_mode.defaults_allowed(),
            !self.input_mode.prompts_allowed(),
        )
        .await
    }
}

async fn update(conn: &mut Connection, args: GovernanceUpdateArgs) -> Result<()> {
    let prepared_context = PreparedGovernanceContext::from_flags(
        args.network.clone(),
        args.non_interactive,
        args.no_defaults,
        args.no_wait,
    );
    let (network_name, network_entry, endpoint_label, source) =
        prepared_context.resolve_network(conn).await?;
    log_resolved_context(&[ContextLine {
        label: "network:",
        value: format!("{network_name} @ {endpoint_label}"),
        source,
    }])?;

    let payload_input = resolve_update_payload_input(&args)?;
    let payload = resolve_update_payload(payload_input, &args)?;
    if args.ledger {
        validate_ledger_signing_context(&payload)?;
    } else {
        validate_blind_signing_context(&payload, &args)?;
    }
    let timing = resolve_update_timing(&args)?;
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
    let prepared = prepare_governance_update(&payload, &chain_context, timing)?;

    let block_item = if args.ledger {
        let key_index = resolve_ledger_key_index(&args)?;
        let signer_context = ledger_update_signer_context(key_index);
        if !approve_governance_update_review(
            "submission",
            &network_name,
            &endpoint_label,
            &prepared,
            &signer_context,
            args.non_interactive,
        )? {
            return Ok(());
        }
        build_ledger_signed_prepared_update_instruction(&prepared, key_index, args.non_interactive)?
    } else {
        ensure_governance_keys_available_for_listing(
            conn,
            &network_name,
            &network_entry.genesis_hash,
        )?;
        let password_value = password(format!("Governance vault password for '{}':", network_name))
            .allow_empty()
            .interact()?;
        let vault = governance::unlock_vault(conn, &network_entry.genesis_hash, &password_value)?;
        let decrypted = governance::decrypted_keys(conn, &network_entry.genesis_hash, &vault.dek)?;
        let signers = resolve_update_signers(&args, &decrypted, &chain_context, &payload)?;
        let signer_context = local_update_signer_context(&signers);
        if !approve_governance_update_review(
            "submission",
            &network_name,
            &endpoint_label,
            &prepared,
            &signer_context,
            args.non_interactive,
        )? {
            return Ok(());
        }
        let signer_outputs = sign_prepared_update_with_local_keys(&prepared, &signers)?;
        assemble_signed_update_instruction(&prepared, signer_outputs)?
    };
    let transaction_hash =
        submit_governance_update(&network_entry.node_endpoint, &endpoint_label, &block_item)
            .await?;
    let message = format!("Submitted governance update: {transaction_hash}");
    if !prepared_context.finalization.should_wait() {
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

#[derive(Clone, Debug)]
struct PreparedGovernanceUpdate {
    payload: ResolvedGovernanceUpdatePayload,
    chain_context: GovernanceUpdateChainContext,
    header: UpdateHeader,
    encoded_payload: EncodedUpdatePayload,
    timing: GovernanceUpdateTiming,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct GovernanceProposalFile {
    version: u32,
    genesis_hash: String,
    header: GovernanceProposalHeaderJson,
    payload: serde_json::Value,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct GovernanceProposalHeaderJson {
    seq_number: u64,
    effective_time: u64,
    timeout: u64,
    payload_size: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct GovernanceSignatureFile {
    version: u32,
    verify_key: String,
    signature: GovernanceUpdateInstructionSignatureJson,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct GovernanceUpdateInstructionSignatureJson {
    signatures: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GovernanceSignerOutput {
    index: UpdateKeysIndex,
    signature: Signature,
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

    fn ledger_governance_purpose(self) -> u32 {
        match self {
            GovernanceAuthFamily::Root => 0,
            GovernanceAuthFamily::Level1 => 1,
            GovernanceAuthFamily::Level2(_) => 2,
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

fn build_ledger_signed_prepared_update_instruction(
    prepared: &PreparedGovernanceUpdate,
    key_index: u32,
    non_interactive: bool,
) -> Result<BlockItem<Payload>> {
    let spin = spinner();
    spin.start("Opening Governance Ledger app and resolving signer key...");
    let result = (|| {
        let transport = governance_ledger::HidTransport::open_first().map_err(ledger_error)?;
        let mut app = governance_ledger::GovernanceLedgerApp::new(transport);
        let outputs =
            sign_prepared_update_with_ledger(&mut app, prepared, key_index, non_interactive)?;
        assemble_signed_update_instruction(prepared, outputs)
    })();
    spin.clear();
    result
}

fn prepare_governance_update(
    payload: &ResolvedGovernanceUpdatePayload,
    chain_context: &GovernanceUpdateChainContext,
    timing: GovernanceUpdateTiming,
) -> Result<PreparedGovernanceUpdate> {
    let sequence_number = chain_context
        .sequence_number
        .context("governance update sequence number could not be resolved")?;
    let encoded_payload = match payload {
        ResolvedGovernanceUpdatePayload::Known { payload, .. } => {
            EncodedUpdatePayload::encode(payload)
        }
        ResolvedGovernanceUpdatePayload::Blind { bytes, .. } => {
            EncodedUpdatePayload::from(bytes.clone())
        }
    };
    let header = UpdateHeader {
        seq_number: sequence_number,
        effective_time: TransactionTime::from_seconds(timing.effective_time_seconds),
        timeout: TransactionTime::from_seconds(timing.timeout_seconds),
        payload_size: encoded_payload.size(),
    };
    Ok(PreparedGovernanceUpdate {
        payload: payload.clone(),
        chain_context: chain_context.clone(),
        header,
        encoded_payload,
        timing,
    })
}

fn proposal_file_from_prepared(
    prepared: &PreparedGovernanceUpdate,
    genesis_hash: &str,
) -> Result<GovernanceProposalFile> {
    let payload = match &prepared.payload {
        ResolvedGovernanceUpdatePayload::Known { payload, .. } => serde_json::to_value(payload)
            .context("failed to serialize governance update payload for proposal file")?,
        ResolvedGovernanceUpdatePayload::Blind { .. } => {
            bail!("detached governance proposals require JSON payloads")
        }
    };
    Ok(GovernanceProposalFile {
        version: 1,
        genesis_hash: genesis_hash.to_owned(),
        header: GovernanceProposalHeaderJson::from_update_header(&prepared.header),
        payload,
    })
}

impl GovernanceProposalHeaderJson {
    fn from_update_header(header: &UpdateHeader) -> Self {
        Self {
            seq_number: header.seq_number.number,
            effective_time: header.effective_time.seconds,
            timeout: header.timeout.seconds,
            payload_size: header.payload_size.into(),
        }
    }

    fn to_update_header(self) -> UpdateHeader {
        UpdateHeader {
            seq_number: UpdateSequenceNumber {
                number: self.seq_number,
            },
            effective_time: TransactionTime::from_seconds(self.effective_time),
            timeout: TransactionTime::from_seconds(self.timeout),
            payload_size: PayloadSize::from(self.payload_size),
        }
    }
}

fn read_governance_proposal_file(path: &Path) -> Result<GovernanceProposalFile> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read governance proposal file {}", path.display()))?;
    let proposal: GovernanceProposalFile = serde_json::from_str(&raw).with_context(|| {
        format!(
            "failed to parse governance proposal file {}",
            path.display()
        )
    })?;
    if proposal.version != 1 {
        bail!(
            "unsupported governance proposal file version {}; expected 1",
            proposal.version
        );
    }
    Ok(proposal)
}

fn write_governance_proposal_file(path: &Path, proposal: &GovernanceProposalFile) -> Result<()> {
    let raw = serde_json::to_string_pretty(proposal)
        .context("failed to serialize governance proposal file")?;
    write_json_file(path, &raw)
}

fn read_governance_signature_file(path: &Path) -> Result<GovernanceSignatureFile> {
    let raw = fs::read_to_string(path).with_context(|| {
        format!(
            "failed to read governance signature file {}",
            path.display()
        )
    })?;
    let signature: GovernanceSignatureFile = serde_json::from_str(&raw).with_context(|| {
        format!(
            "failed to parse governance signature file {}",
            path.display()
        )
    })?;
    if signature.version != 1 {
        bail!(
            "unsupported governance signature file version {}; expected 1",
            signature.version
        );
    }
    Ok(signature)
}

fn write_governance_signature_file(path: &Path, signature: &GovernanceSignatureFile) -> Result<()> {
    let raw = serde_json::to_string_pretty(signature)
        .context("failed to serialize governance signature file")?;
    write_json_file(path, &raw)
}

fn write_json_file(path: &Path, content: &str) -> Result<()> {
    if path.exists() {
        bail!("refusing to overwrite existing file {}", path.display());
    }
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        bail!("output directory does not exist: {}", parent.display());
    }
    fs::write(path, format!("{content}\n"))
        .with_context(|| format!("failed to write JSON file {}", path.display()))
}

fn resolved_payload_from_proposal(
    proposal: &GovernanceProposalFile,
) -> Result<ResolvedGovernanceUpdatePayload> {
    let payload: UpdatePayload = serde_json::from_value(proposal.payload.clone())
        .context("failed to parse governance proposal payload JSON")?;
    let update_type = payload.update_type();
    Ok(ResolvedGovernanceUpdatePayload::Known {
        payload,
        auth_family: auth_family_for_update_type(update_type),
        sequence_queue: GovernanceSequenceQueue::from_update_type(update_type),
    })
}

fn prepared_update_from_proposal(
    proposal: &GovernanceProposalFile,
    chain_context: GovernanceUpdateChainContext,
) -> Result<PreparedGovernanceUpdate> {
    let payload = resolved_payload_from_proposal(proposal)?;
    let encoded_payload = match &payload {
        ResolvedGovernanceUpdatePayload::Known { payload, .. } => {
            EncodedUpdatePayload::encode(payload)
        }
        ResolvedGovernanceUpdatePayload::Blind { .. } => unreachable!("proposals are typed JSON"),
    };
    let header = proposal.header.to_update_header();
    if header.payload_size != encoded_payload.size() {
        bail!(
            "governance proposal payload size mismatch: header says {}, encoded payload is {}",
            Into::<u32>::into(header.payload_size),
            Into::<u32>::into(encoded_payload.size())
        );
    }
    Ok(PreparedGovernanceUpdate {
        payload,
        chain_context,
        header,
        encoded_payload,
        timing: GovernanceUpdateTiming {
            effective_time_seconds: proposal.header.effective_time,
            timeout_seconds: proposal.header.timeout,
        },
    })
}

fn signature_file_from_signer_output(
    verify_key: String,
    output: GovernanceSignerOutput,
) -> GovernanceSignatureFile {
    let mut signatures = BTreeMap::new();
    signatures.insert(
        output.index.index.to_string(),
        hex::encode(&output.signature.sig),
    );
    GovernanceSignatureFile {
        version: 1,
        verify_key,
        signature: GovernanceUpdateInstructionSignatureJson { signatures },
    }
}

fn signer_output_from_signature_file(
    signature_file: &GovernanceSignatureFile,
) -> Result<GovernanceSignerOutput> {
    let entries = signature_file
        .signature
        .signatures
        .iter()
        .collect::<Vec<_>>();
    let [(index, signature)] = entries.as_slice() else {
        bail!("detached governance signature files must contain exactly one signature entry");
    };
    let index = index
        .parse::<u16>()
        .with_context(|| format!("invalid governance signature index '{index}'"))?;
    let signature = hex::decode(signature)
        .with_context(|| format!("invalid hex governance signature for index {index}"))?;
    Ok(GovernanceSignerOutput {
        index: UpdateKeysIndex { index },
        signature: Signature { sig: signature },
    })
}

fn validate_ledger_signing_context(payload: &ResolvedGovernanceUpdatePayload) -> Result<()> {
    if matches!(payload, ResolvedGovernanceUpdatePayload::Blind { .. }) {
        bail!(
            "Ledger governance signing does not support blind signing unknown serialized governance update payloads"
        );
    }
    if payload.auth_family_hint().is_none() {
        bail!(
            "Ledger governance signing requires a decoded governance update authorization family"
        );
    }
    Ok(())
}

fn resolve_ledger_key_index(args: &GovernanceUpdateArgs) -> Result<u32> {
    match args.ledger_key_index {
        Some(index) => Ok(index),
        None if args.non_interactive => Ok(0),
        None => {
            let value: String = input("Governance Ledger key index:")
                .default_input("0")
                .interact()?;
            value
                .parse::<u32>()
                .with_context(|| format!("invalid Governance Ledger key index '{value}'"))
        }
    }
}

fn ledger_fallback_signature_index(key_index: u32) -> Result<UpdateKeysIndex> {
    let index = key_index.try_into().with_context(|| {
        format!("Ledger key index {key_index} cannot be used as a governance signature index")
    })?;
    Ok(UpdateKeysIndex { index })
}

fn ledger_auth_family(prepared: &PreparedGovernanceUpdate) -> Result<GovernanceAuthFamily> {
    prepared
        .payload
        .auth_family_hint()
        .context("Ledger governance signing requires a known authorization family")
}

fn ledger_derivation_path_for_prepared_update(
    prepared: &PreparedGovernanceUpdate,
    key_index: u32,
) -> Result<governance_ledger::DerivationPath> {
    let purpose = ledger_auth_family(prepared)?.ledger_governance_purpose();
    governance_ledger::DerivationPath::new([1105, 0, 1, purpose, key_index])
        .map_err(|err| anyhow::anyhow!(err.to_string()))
}

fn ledger_public_key_from_bytes(public_key: [u8; 32]) -> Result<UpdatePublicKey> {
    let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&public_key)
        .context("Governance Ledger returned an invalid Ed25519 public key")?;
    Ok(UpdatePublicKey {
        public: verifying_key.into(),
    })
}

fn signer_index_for_verify_key(
    prepared: &PreparedGovernanceUpdate,
    verify_key: &str,
) -> Result<UpdateKeysIndex> {
    let family = prepared
        .payload
        .auth_family_hint()
        .context("governance proposal requires a known authorization family")?;
    let authorization =
        governance_authorization_for_family(&prepared.chain_context.chain_parameters, family)?;
    let normalized = verify_key.trim().to_ascii_lowercase();
    let matches = authorization
        .keys
        .iter()
        .enumerate()
        .filter_map(|(index, public_key)| {
            (governance::public_key_hex(public_key) == normalized).then_some(UpdateKeysIndex {
                index: index as u16,
            })
        })
        .filter(|index| authorization.authorized_indices.contains(index))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [index] => Ok(*index),
        [] => bail!(
            "governance key '{}' is not authorized for '{}' updates",
            verify_key,
            family.label()
        ),
        _ => bail!(
            "governance key '{}' maps to multiple governance key indices for '{}' updates",
            verify_key,
            family.label()
        ),
    }
}

fn ledger_signer_index_for_public_key(
    prepared: &PreparedGovernanceUpdate,
    public_key: &UpdatePublicKey,
) -> Result<UpdateKeysIndex> {
    let family = ledger_auth_family(prepared)?;
    let authorization =
        governance_authorization_for_family(&prepared.chain_context.chain_parameters, family)?;
    if authorization.threshold > 1 {
        bail!(
            "all-in-one Ledger governance signing currently supports 1 signer, but '{}' updates require threshold {}",
            family.label(),
            authorization.threshold
        );
    }
    let matches = indices_for_public_key(authorization.keys, public_key)
        .into_iter()
        .filter(|index| authorization.authorized_indices.contains(index))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [index] => Ok(*index),
        [] => bail!(
            "Governance Ledger key '{}' is not authorized for '{}' updates",
            governance::public_key_hex(public_key),
            family.label()
        ),
        _ => bail!(
            "Governance Ledger key '{}' maps to multiple governance key indices for '{}' updates",
            governance::public_key_hex(public_key),
            family.label()
        ),
    }
}

fn resolve_ledger_signer_index_or_prompt(
    prepared: &PreparedGovernanceUpdate,
    public_key: &UpdatePublicKey,
    key_index: u32,
    non_interactive: bool,
) -> Result<UpdateKeysIndex> {
    match ledger_signer_index_for_public_key(prepared, public_key) {
        Ok(index) => Ok(index),
        Err(err) if non_interactive => Err(err),
        Err(err) => prompt_unvalidated_ledger_signature_index(err, key_index),
    }
}

fn resolve_detached_ledger_signer_index_or_prompt(
    prepared: &PreparedGovernanceUpdate,
    verify_key: &str,
    key_index: u32,
    non_interactive: bool,
) -> Result<UpdateKeysIndex> {
    match signer_index_for_verify_key(prepared, verify_key) {
        Ok(index) => Ok(index),
        Err(err) if non_interactive => Err(err),
        Err(err) => prompt_unvalidated_ledger_signature_index(err, key_index),
    }
}

fn prompt_unvalidated_ledger_signature_index(
    validation_error: anyhow::Error,
    key_index: u32,
) -> Result<UpdateKeysIndex> {
    let fallback_index = ledger_fallback_signature_index(key_index)?;
    cliclack::log::warning(format!(
        "Governance Ledger signer validation failed: {validation_error}"
    ))?;
    let approved = confirm(format!(
        "Continue signing anyway and use governance signature index {}? The node may reject the update.",
        fallback_index.index
    ))
    .initial_value(false)
    .interact()?;
    if !approved {
        bail!("governance Ledger signing aborted after validation failure");
    }
    Ok(fallback_index)
}

fn sign_prepared_update_with_ledger<T: governance_ledger::GovernanceLedgerTransport>(
    app: &mut governance_ledger::GovernanceLedgerApp<T>,
    prepared: &PreparedGovernanceUpdate,
    key_index: u32,
    non_interactive: bool,
) -> Result<Vec<GovernanceSignerOutput>> {
    let path = ledger_derivation_path_for_prepared_update(prepared, key_index)?;
    let public_key_response = app
        .get_public_key(path.clone(), governance_ledger::PublicKeyOptions::default())
        .map_err(ledger_error)?;
    let public_key = ledger_public_key_from_bytes(public_key_response.public_key)?;
    let signer_index =
        resolve_ledger_signer_index_or_prompt(prepared, &public_key, key_index, non_interactive)?;
    let signature = sign_prepared_update_with_ledger_path(app, prepared, path)?;
    Ok(vec![GovernanceSignerOutput {
        index: signer_index,
        signature: Signature {
            sig: signature.0.to_vec(),
        },
    }])
}

fn sign_detached_prepared_update_with_ledger<T: governance_ledger::GovernanceLedgerTransport>(
    app: &mut governance_ledger::GovernanceLedgerApp<T>,
    prepared: &PreparedGovernanceUpdate,
    key_index: u32,
    non_interactive: bool,
) -> Result<(String, GovernanceSignerOutput)> {
    let path = ledger_derivation_path_for_prepared_update(prepared, key_index)?;
    let public_key_response = app
        .get_public_key(path.clone(), governance_ledger::PublicKeyOptions::default())
        .map_err(ledger_error)?;
    let public_key = ledger_public_key_from_bytes(public_key_response.public_key)?;
    let verify_key = governance::public_key_hex(&public_key);
    let signer_index = resolve_detached_ledger_signer_index_or_prompt(
        prepared,
        &verify_key,
        key_index,
        non_interactive,
    )?;
    let signature = sign_prepared_update_with_ledger_path(app, prepared, path)?;
    Ok((
        verify_key,
        GovernanceSignerOutput {
            index: signer_index,
            signature: Signature {
                sig: signature.0.to_vec(),
            },
        },
    ))
}

fn sign_prepared_update_with_ledger_path<T: governance_ledger::GovernanceLedgerTransport>(
    app: &mut governance_ledger::GovernanceLedgerApp<T>,
    prepared: &PreparedGovernanceUpdate,
    path: governance_ledger::DerivationPath,
) -> Result<governance_ledger::RawSignature> {
    let prefix = ledger_update_prefix(prepared, path)?;
    let ResolvedGovernanceUpdatePayload::Known { payload, .. } = &prepared.payload else {
        bail!(
            "Ledger governance signing does not support blind signing unknown serialized governance update payloads"
        );
    };
    match payload {
        UpdatePayload::Protocol(update) => app
            .sign_protocol_update(&governance_ledger::ProtocolUpdateRequest::from((
                prefix,
                update.clone(),
            )))
            .map_err(ledger_error),
        UpdatePayload::EuroPerEnergy(update) => app
            .sign_exchange_rate(&fixed_ledger_request(prefix, *update))
            .map_err(ledger_error),
        UpdatePayload::MicroGTUPerEuro(update) => app
            .sign_exchange_rate(&fixed_ledger_request(prefix, *update))
            .map_err(ledger_error),
        UpdatePayload::FoundationAccount(update) => app
            .sign_foundation_account(&fixed_ledger_request(prefix, *update))
            .map_err(ledger_error),
        UpdatePayload::MintDistribution(update) => app
            .sign_mint_distribution(&fixed_ledger_request(prefix, update.clone()))
            .map_err(ledger_error),
        UpdatePayload::MintDistributionCPV1(update) => app
            .sign_mint_distribution(&fixed_ledger_request(prefix, update.clone()))
            .map_err(ledger_error),
        UpdatePayload::TransactionFeeDistribution(update) => app
            .sign_transaction_fee_distribution(&fixed_ledger_request(prefix, update.clone()))
            .map_err(ledger_error),
        UpdatePayload::GASRewards(update) => app
            .sign_gas_rewards(&fixed_ledger_request(prefix, update.clone()))
            .map_err(ledger_error),
        UpdatePayload::GASRewardsCPV2(update) => app
            .sign_gas_rewards(&fixed_ledger_request(prefix, update.clone()))
            .map_err(ledger_error),
        UpdatePayload::BakerStakeThreshold(update) => app
            .sign_baker_stake_threshold(&fixed_ledger_request(prefix, update.clone()))
            .map_err(ledger_error),
        UpdatePayload::CooldownParametersCPV1(update) => app
            .sign_cooldown_parameters(&fixed_ledger_request(prefix, *update))
            .map_err(ledger_error),
        UpdatePayload::PoolParametersCPV1(update) => app
            .sign_pool_parameters(&fixed_ledger_request(prefix, update.clone()))
            .map_err(ledger_error),
        UpdatePayload::TimeParametersCPV1(update) => app
            .sign_time_parameters(&fixed_ledger_request(prefix, *update))
            .map_err(ledger_error),
        UpdatePayload::TimeoutParametersCPV2(update) => app
            .sign_timeout_parameters(&fixed_ledger_request(prefix, *update))
            .map_err(ledger_error),
        UpdatePayload::MinBlockTimeCPV2(update) => app
            .sign_min_block_time(&fixed_ledger_request(prefix, *update))
            .map_err(ledger_error),
        UpdatePayload::BlockEnergyLimitCPV2(update) => app
            .sign_block_energy_limit(&fixed_ledger_request(prefix, *update))
            .map_err(ledger_error),
        UpdatePayload::FinalizationCommitteeParametersCPV2(update) => app
            .sign_finalization_committee_parameters(&fixed_ledger_request(prefix, *update))
            .map_err(ledger_error),
        UpdatePayload::ValidatorScoreParametersCPV3(update) => app
            .sign_validator_score_parameters(&fixed_ledger_request(prefix, *update))
            .map_err(ledger_error),
        UpdatePayload::AddAnonymityRevoker(update) => app
            .sign_add_anonymity_revoker(&governance_ledger::AddAnonymityRevokerRequest::from((
                prefix,
                (**update).clone(),
            )))
            .map_err(ledger_error),
        UpdatePayload::AddIdentityProvider(update) => app
            .sign_add_identity_provider(&governance_ledger::AddIdentityProviderRequest::from((
                prefix,
                (**update).clone(),
            )))
            .map_err(ledger_error),
        UpdatePayload::CreatePlt(update) => app
            .sign_create_plt(&governance_ledger::CreatePltRequest::from((
                prefix,
                update.clone(),
            )))
            .map_err(ledger_error),
        UpdatePayload::Root(update) => sign_root_update_with_ledger(app, prefix, update),
        UpdatePayload::Level1(update) => sign_level1_update_with_ledger(app, prefix, update),
        UpdatePayload::ElectionDifficulty(_) => bail!(
            "Ledger governance signing does not support election-difficulty governance updates"
        ),
    }
}

fn sign_root_update_with_ledger<T: governance_ledger::GovernanceLedgerTransport>(
    app: &mut governance_ledger::GovernanceLedgerApp<T>,
    prefix: governance_ledger::GovernanceUpdatePrefix,
    update: &concordium_rust_sdk::base::updates::RootUpdate,
) -> Result<governance_ledger::RawSignature> {
    use concordium_rust_sdk::base::updates::RootUpdate;
    match update {
        RootUpdate::RootKeysUpdate(update) => app
            .sign_update_root_keys(&governance_ledger::HigherLevelKeyUpdateRequest::from((
                prefix,
                governance_ledger::HigherLevelKeyUpdateType::RootKeys,
                update.clone(),
            )))
            .map_err(ledger_error),
        RootUpdate::Level1KeysUpdate(update) => app
            .sign_update_level1_keys_with_root_keys(
                &governance_ledger::HigherLevelKeyUpdateRequest::from((
                    prefix,
                    governance_ledger::HigherLevelKeyUpdateType::RootKeys,
                    update.clone(),
                )),
            )
            .map_err(ledger_error),
        RootUpdate::Level2KeysUpdate(update) => app
            .sign_update_authorizations_with_root_keys(
                &governance_ledger::AuthorizationsUpdateRequest::from((
                    prefix,
                    governance_ledger::AuthorizationsKeyUpdateType::RootKeys,
                    governance_ledger::AuthorizationsVersion::V0,
                    (**update).clone(),
                )),
            )
            .map_err(ledger_error),
        RootUpdate::Level2KeysUpdateV1(update) => app
            .sign_update_authorizations_with_root_keys(
                &governance_ledger::AuthorizationsUpdateRequest::from((
                    prefix,
                    governance_ledger::AuthorizationsKeyUpdateType::RootKeys,
                    governance_ledger::AuthorizationsVersion::V1,
                    (**update).clone(),
                )),
            )
            .map_err(ledger_error),
        RootUpdate::Level2KeysUpdateV2(update) => app
            .sign_update_authorizations_with_root_keys(
                &governance_ledger::AuthorizationsUpdateRequest::from((
                    prefix,
                    governance_ledger::AuthorizationsKeyUpdateType::RootKeys,
                    governance_ledger::AuthorizationsVersion::V2,
                    (**update).clone(),
                )),
            )
            .map_err(ledger_error),
    }
}

fn sign_level1_update_with_ledger<T: governance_ledger::GovernanceLedgerTransport>(
    app: &mut governance_ledger::GovernanceLedgerApp<T>,
    prefix: governance_ledger::GovernanceUpdatePrefix,
    update: &concordium_rust_sdk::base::updates::Level1Update,
) -> Result<governance_ledger::RawSignature> {
    use concordium_rust_sdk::base::updates::Level1Update;
    match update {
        Level1Update::Level1KeysUpdate(update) => app
            .sign_update_level1_keys_with_level1_keys(
                &governance_ledger::HigherLevelKeyUpdateRequest::from((
                    prefix,
                    governance_ledger::HigherLevelKeyUpdateType::Level1Keys,
                    update.clone(),
                )),
            )
            .map_err(ledger_error),
        Level1Update::Level2KeysUpdate(update) => app
            .sign_update_authorizations_with_level1_keys(
                &governance_ledger::AuthorizationsUpdateRequest::from((
                    prefix,
                    governance_ledger::AuthorizationsKeyUpdateType::Level1Keys,
                    governance_ledger::AuthorizationsVersion::V0,
                    (**update).clone(),
                )),
            )
            .map_err(ledger_error),
        Level1Update::Level2KeysUpdateV1(update) => app
            .sign_update_authorizations_with_level1_keys(
                &governance_ledger::AuthorizationsUpdateRequest::from((
                    prefix,
                    governance_ledger::AuthorizationsKeyUpdateType::Level1Keys,
                    governance_ledger::AuthorizationsVersion::V1,
                    (**update).clone(),
                )),
            )
            .map_err(ledger_error),
        Level1Update::Level2KeysUpdateV2(update) => app
            .sign_update_authorizations_with_level1_keys(
                &governance_ledger::AuthorizationsUpdateRequest::from((
                    prefix,
                    governance_ledger::AuthorizationsKeyUpdateType::Level1Keys,
                    governance_ledger::AuthorizationsVersion::V2,
                    (**update).clone(),
                )),
            )
            .map_err(ledger_error),
    }
}

fn fixed_ledger_request<P: Serial>(
    prefix: governance_ledger::GovernanceUpdatePrefix,
    payload: P,
) -> governance_ledger::FixedUpdateRequest {
    governance_ledger::FixedUpdateRequest::from((prefix, payload))
}

fn ledger_update_prefix(
    prepared: &PreparedGovernanceUpdate,
    path: governance_ledger::DerivationPath,
) -> Result<governance_ledger::GovernanceUpdatePrefix> {
    let update_type = prepared
        .encoded_payload
        .as_ref()
        .first()
        .copied()
        .context("prepared governance update payload is empty")?;
    let header = governance_ledger::UpdateHeaderBytes::try_from(prepared.header)
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    Ok(governance_ledger::GovernanceUpdatePrefix {
        path,
        header,
        update_type,
    })
}

fn ledger_error(err: governance_ledger::GovernanceLedgerError) -> anyhow::Error {
    match err {
        governance_ledger::GovernanceLedgerError::UserDeclined => {
            anyhow::anyhow!("Ledger governance signing was declined on the device")
        }
        other => anyhow::anyhow!("Governance Ledger operation failed: {other}"),
    }
}

fn sign_detached_prepared_update_with_local_key(
    prepared: &PreparedGovernanceUpdate,
    signer: &governance::DecryptedGovernanceKey,
) -> Result<GovernanceSignerOutput> {
    let verify_key = governance::public_key_hex(&signer.public_key);
    let index = signer_index_for_verify_key(prepared, &verify_key)?;
    let signer = BTreeMap::from([(index, signer.key_pair.clone())]);
    let signature_map = signer.sign_update_hash(&compute_update_sign_hash(
        &prepared.header,
        &prepared.encoded_payload,
    ));
    let signature = signature_map
        .signatures
        .into_iter()
        .next()
        .map(|(_, signature)| signature)
        .context("local governance signing produced no signature")?;
    Ok(GovernanceSignerOutput { index, signature })
}

fn sign_prepared_update_with_local_keys(
    prepared: &PreparedGovernanceUpdate,
    signers: &[governance::DecryptedGovernanceKey],
) -> Result<Vec<GovernanceSignerOutput>> {
    let signer = signer_map_for_payload(
        &prepared.payload,
        &prepared.chain_context.chain_parameters,
        signers,
    )?;
    let signature_map = signer.sign_update_hash(&compute_update_sign_hash(
        &prepared.header,
        &prepared.encoded_payload,
    ));
    Ok(signature_map
        .signatures
        .into_iter()
        .map(|(index, signature)| GovernanceSignerOutput { index, signature })
        .collect())
}

fn assemble_signed_update_instruction(
    prepared: &PreparedGovernanceUpdate,
    signer_outputs: Vec<GovernanceSignerOutput>,
) -> Result<BlockItem<Payload>> {
    let _timing = prepared.timing;
    if signer_outputs.is_empty() {
        bail!("at least one governance signer output is required");
    }
    let signatures = signer_outputs
        .into_iter()
        .map(|output| (output.index, output.signature))
        .collect::<BTreeMap<_, _>>();
    let update_instruction = UpdateInstruction {
        header: prepared.header,
        payload: prepared.encoded_payload.clone(),
        signatures: UpdateInstructionSignature { signatures },
    };
    Ok(update_instruction.into())
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
    let authorization = governance_authorization_for_family(chain_parameters, family)?;
    let signer = signer_map_from_key_list(
        authorization.keys,
        Some(&authorization.authorized_indices),
        signers,
    )?;
    let threshold = authorization.threshold;
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

struct GovernanceAuthorizationContext<'a> {
    keys: &'a [UpdatePublicKey],
    authorized_indices: BTreeSet<UpdateKeysIndex>,
    threshold: usize,
}

fn governance_authorization_for_family(
    chain_parameters: &ChainParameters,
    family: GovernanceAuthFamily,
) -> Result<GovernanceAuthorizationContext<'_>> {
    match family {
        GovernanceAuthFamily::Root => {
            let keys = chain_parameters
                .keys
                .root_keys
                .as_ref()
                .context("root governance keys are not present in chain parameters")?;
            let authorized_indices = (0..keys.keys.len())
                .map(|index| UpdateKeysIndex {
                    index: index as u16,
                })
                .collect::<BTreeSet<_>>();
            Ok(GovernanceAuthorizationContext {
                keys: &keys.keys,
                authorized_indices,
                threshold: usize::from(u16::from(keys.threshold)),
            })
        }
        GovernanceAuthFamily::Level1 => {
            let keys = chain_parameters
                .keys
                .level_1_keys
                .as_ref()
                .context("level 1 governance keys are not present in chain parameters")?;
            let authorized_indices = (0..keys.keys.len())
                .map(|index| UpdateKeysIndex {
                    index: index as u16,
                })
                .collect::<BTreeSet<_>>();
            Ok(GovernanceAuthorizationContext {
                keys: &keys.keys,
                authorized_indices,
                threshold: usize::from(u16::from(keys.threshold)),
            })
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
            Ok(GovernanceAuthorizationContext {
                keys: &level2.keys,
                authorized_indices: access.authorized_keys.clone(),
                threshold: usize::from(u16::from(access.threshold)),
            })
        }
    }
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct GovernanceReviewSignerContext {
    heading: &'static str,
    lines: Vec<String>,
}

impl GovernanceReviewSignerContext {
    fn new(heading: &'static str, lines: Vec<String>) -> Self {
        Self { heading, lines }
    }
}

fn render_governance_update_review(
    action: &str,
    network_name: &str,
    endpoint_label: &str,
    prepared: &PreparedGovernanceUpdate,
    signer_context: &GovernanceReviewSignerContext,
) -> Result<String> {
    let mut lines = vec![
        format!("Governance update {action}"),
        String::new(),
        "Network:".to_owned(),
        format!("  {network_name} @ {endpoint_label}"),
        String::new(),
        "Update:".to_owned(),
        format!("  payload: {}", prepared.payload.context_label()),
        format!(
            "  payload size: {} bytes",
            prepared.encoded_payload.as_ref().len()
        ),
        format!("  sequence number: {}", prepared.header.seq_number.number),
        format!(
            "  effective time: {}",
            format_update_review_time(prepared.timing.effective_time_seconds)?
        ),
        format!(
            "  timeout: {}",
            format_update_review_time(prepared.timing.timeout_seconds)?
        ),
    ];

    lines.push(String::new());
    lines.push(format!("{}:", signer_context.heading));
    for line in &signer_context.lines {
        lines.push(format!("  {line}"));
    }

    lines.push(String::new());
    match render_payload_details(&prepared.payload)? {
        Some(details) => {
            lines.push("Payload details:".to_owned());
            lines.extend(indent_multiline(&details, "  "));
        }
        None => {
            lines.push("Payload details:".to_owned());
            lines.push("  blind serialized payload; semantic details are unavailable".to_owned());
            lines.push(format!(
                "  raw payload bytes: {}",
                prepared.encoded_payload.as_ref().len()
            ));
            lines.push(
                "  warning: approve only if trusted tooling produced this payload and it was independently reviewed"
                    .to_owned(),
            );
        }
    }

    Ok(lines.join("\n"))
}

fn format_update_review_time(seconds: u64) -> Result<String> {
    if seconds == 0 {
        return Ok("0 (immediate)".to_owned());
    }
    Ok(format!(
        "{} ({})",
        seconds,
        format_unix_seconds_rfc3339(seconds)?
    ))
}

fn render_payload_details(payload: &ResolvedGovernanceUpdatePayload) -> Result<Option<String>> {
    match payload {
        ResolvedGovernanceUpdatePayload::Known { payload, .. } => {
            serde_json::to_string_pretty(payload)
                .map(Some)
                .context("failed to render governance update payload details")
        }
        ResolvedGovernanceUpdatePayload::Blind { .. } => Ok(None),
    }
}

fn indent_multiline(value: &str, indent: &str) -> Vec<String> {
    value
        .lines()
        .map(|line| format!("{indent}{line}"))
        .collect()
}

fn local_update_signer_context(
    signers: &[governance::DecryptedGovernanceKey],
) -> GovernanceReviewSignerContext {
    GovernanceReviewSignerContext::new(
        "Signing",
        std::iter::once("mode: local governance vault".to_owned())
            .chain(signers.iter().map(|signer| {
                format!(
                    "selected key: {}",
                    governance::public_key_hex(&signer.public_key)
                )
            }))
            .collect(),
    )
}

fn ledger_update_signer_context(key_index: u32) -> GovernanceReviewSignerContext {
    GovernanceReviewSignerContext::new(
        "Signing",
        vec![
            "mode: Governance Ledger".to_owned(),
            format!("Ledger key index: {key_index}"),
        ],
    )
}

fn local_detached_signer_context(verify_key: &str) -> GovernanceReviewSignerContext {
    GovernanceReviewSignerContext::new(
        "Detached signing",
        vec![
            "mode: local governance vault".to_owned(),
            format!("selected key: {verify_key}"),
        ],
    )
}

fn ledger_detached_signer_context(key_index: u32) -> GovernanceReviewSignerContext {
    GovernanceReviewSignerContext::new(
        "Detached signing",
        vec![
            "mode: Governance Ledger".to_owned(),
            format!("Ledger key index: {key_index}"),
        ],
    )
}

fn detached_submission_signer_context(
    outputs: &[GovernanceSignerOutput],
) -> GovernanceReviewSignerContext {
    GovernanceReviewSignerContext::new(
        "Detached signatures",
        outputs
            .iter()
            .map(|output| format!("accepted signature index: {}", output.index.index))
            .collect(),
    )
}

fn approve_governance_update_review(
    action: &str,
    network_name: &str,
    endpoint_label: &str,
    prepared: &PreparedGovernanceUpdate,
    signer_context: &GovernanceReviewSignerContext,
    non_interactive: bool,
) -> Result<bool> {
    if non_interactive {
        return Ok(true);
    }
    let review = render_governance_update_review(
        action,
        network_name,
        endpoint_label,
        prepared,
        signer_context,
    )?;
    cliclack::log::info(review)?;
    let approved = confirm(format!("Approve governance update {action}?"))
        .initial_value(false)
        .interact()?;
    if !approved {
        cliclack::log::warning(format!("governance update {action} declined by user"))?;
    }
    Ok(approved)
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
                None => Promptable::Missing {
                    value_name: "JSON file",
                }
                .resolve_with(InputMode::from_flags(args.non_interactive, false), || {
                    Ok(input("Paste governance update JSON:").interact()?)
                })
                .with_context(
                    || "JSON file must be provided with `--json <FILE>` in --non-interactive mode",
                )?
                .into_value(),
            };
            if raw_json.trim().is_empty() {
                bail!("governance update JSON payload cannot be empty");
            }
            Ok(GovernanceUpdatePayloadInput::Json(raw_json))
        }
        (None, Some(serialized)) => {
            let raw_hex = match serialized {
                Some(hex) => hex.clone(),
                None => Promptable::Missing {
                    value_name: "serialized payload",
                }
                    .resolve_with(InputMode::from_flags(args.non_interactive, false), || {
                        Ok(input("Paste serialized governance update hex:").interact()?)
                    })
                    .with_context(
                        || "serialized payload must be provided with `--serialized <HEX>` in --non-interactive mode",
                    )?
                    .into_value(),
            };
            let bytes = decode_hex_payload(&raw_hex)?;
            if bytes.is_empty() {
                bail!("serialized governance update payload cannot be empty");
            }
            Ok(GovernanceUpdatePayloadInput::Serialized(bytes))
        }
        (None, None) => {
            Promptable::<()>::Missing {
                value_name: "governance update payload",
            }
            .resolve_with(
                InputMode::from_flags(args.non_interactive, false),
                || Ok(()),
            )
            .with_context(
                || "provide either `--json` or `--serialized` in --non-interactive mode",
            )?;
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

fn resolve_proposal_timing(
    effective_time: Option<&str>,
    timeout: Option<&str>,
    non_interactive: bool,
) -> Result<GovernanceUpdateTiming> {
    let now = now_unix_seconds()?;
    let effective_time_seconds = match effective_time {
        Some(value) => parse_effective_time_input(value, now)?,
        None => {
            let value = Promptable::<String>::Missing {
                value_name: "effective time",
            }
            .resolve_with(InputMode::from_flags(non_interactive, false), || {
                Ok(input("Effective time:").interact()?)
            })
            .with_context(
                || "`--effective-time <TIME>` is required in --non-interactive proposal creation",
            )?
            .into_value();
            parse_effective_time_input(&value, now)?
        }
    };
    let timeout_seconds = match timeout {
        Some(value) => parse_time_input(value, now)?,
        None => {
            let value = Promptable::<String>::Missing {
                value_name: "timeout",
            }
            .resolve_with(InputMode::from_flags(non_interactive, false), || {
                Ok(input("Timeout:").interact()?)
            })
            .with_context(
                || "`--timeout <TIME>` is required in --non-interactive proposal creation",
            )?
            .into_value();
            parse_time_input(&value, now)?
        }
    };
    validate_update_timing(effective_time_seconds, timeout_seconds, now)?;
    Ok(GovernanceUpdateTiming {
        effective_time_seconds,
        timeout_seconds,
    })
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

async fn create_proposal(conn: &mut Connection, args: GovernanceProposalCreateArgs) -> Result<()> {
    let prepared_context = PreparedGovernanceContext::from_flags(
        args.network.clone(),
        args.non_interactive,
        args.no_defaults,
        false,
    );
    let (network_name, network_entry, endpoint_label, source) =
        prepared_context.resolve_network(conn).await?;
    log_resolved_context(&[ContextLine {
        label: "network:",
        value: format!("{network_name} @ {endpoint_label}"),
        source,
    }])?;

    let raw_json = fs::read_to_string(&args.json).with_context(|| {
        format!(
            "failed to read governance update JSON file {}",
            args.json.display()
        )
    })?;
    let payload = resolve_update_payload(
        GovernanceUpdatePayloadInput::Json(raw_json),
        &proposal_payload_args(),
    )?;
    let timing = resolve_proposal_timing(
        args.effective_time.as_deref(),
        args.timeout.as_deref(),
        args.non_interactive,
    )?;
    let chain_context = resolve_update_chain_context(
        &network_entry.node_endpoint,
        &endpoint_label,
        &payload,
        None,
    )
    .await?;
    let prepared = prepare_governance_update(&payload, &chain_context, timing)?;
    let proposal = proposal_file_from_prepared(&prepared, &network_entry.genesis_hash)?;
    write_governance_proposal_file(&args.out, &proposal)?;
    println!("Wrote governance proposal to {}.", args.out.display());
    Ok(())
}

fn proposal_payload_args() -> GovernanceUpdateArgs {
    GovernanceUpdateArgs {
        json: None,
        serialized: None,
        blind: false,
        keys: Vec::new(),
        ledger: false,
        ledger_key_index: None,
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

async fn proposal_chain_context(
    node_endpoint: &str,
    endpoint_label: &str,
    proposal: &GovernanceProposalFile,
) -> Result<GovernanceUpdateChainContext> {
    let payload = resolved_payload_from_proposal(proposal)?;
    let chain_parameters = fetch_chain_parameters(node_endpoint, endpoint_label).await?;
    let queue = payload
        .sequence_queue_hint()
        .context("governance proposal payload has no update sequence queue")?;
    let next = fetch_next_update_sequence_numbers(node_endpoint, endpoint_label).await?;
    let live_sequence = queue.next_sequence_number(&next);
    let proposal_sequence = validate_proposal_sequence(proposal, queue, live_sequence)?;
    Ok(GovernanceUpdateChainContext {
        chain_parameters,
        sequence_number: Some(proposal_sequence),
        sequence_number_source: Some(ResolutionSource::Explicit),
    })
}

async fn load_and_prepare_proposal(
    node_endpoint: &str,
    endpoint_label: &str,
    expected_genesis_hash: &str,
    proposal_path: &Path,
) -> Result<(GovernanceProposalFile, PreparedGovernanceUpdate)> {
    let proposal = read_governance_proposal_file(proposal_path)?;
    if proposal.genesis_hash != expected_genesis_hash {
        bail!(
            "governance proposal targets genesis hash '{}', but selected network has genesis hash '{}'",
            proposal.genesis_hash,
            expected_genesis_hash
        );
    }
    let chain_context = proposal_chain_context(node_endpoint, endpoint_label, &proposal).await?;
    let prepared = prepared_update_from_proposal(&proposal, chain_context)?;
    Ok((proposal, prepared))
}

async fn sign_proposal(conn: &mut Connection, args: GovernanceProposalSignArgs) -> Result<()> {
    let prepared_context = PreparedGovernanceContext::from_flags(
        args.network.clone(),
        args.non_interactive,
        args.no_defaults,
        false,
    );
    let (network_name, network_entry, endpoint_label, source) =
        prepared_context.resolve_network(conn).await?;
    log_resolved_context(&[ContextLine {
        label: "network:",
        value: format!("{network_name} @ {endpoint_label}"),
        source,
    }])?;
    let (_proposal, prepared) = load_and_prepare_proposal(
        &network_entry.node_endpoint,
        &endpoint_label,
        &network_entry.genesis_hash,
        &args.proposal,
    )
    .await?;

    let signature = if args.ledger {
        let key_index = match args.ledger_key_index {
            Some(index) => index,
            None if args.non_interactive => 0,
            None => {
                let value: String = input("Governance Ledger key index:")
                    .default_input("0")
                    .interact()?;
                value
                    .parse::<u32>()
                    .with_context(|| format!("invalid Governance Ledger key index '{value}'"))?
            }
        };
        let signer_context = ledger_detached_signer_context(key_index);
        if !approve_governance_update_review(
            "detached signing",
            &network_name,
            &endpoint_label,
            &prepared,
            &signer_context,
            args.non_interactive,
        )? {
            return Ok(());
        }
        let spin = spinner();
        spin.start("Opening Governance Ledger app and signing proposal...");
        let result: Result<GovernanceSignatureFile> = (|| {
            let transport = governance_ledger::HidTransport::open_first().map_err(ledger_error)?;
            let mut app = governance_ledger::GovernanceLedgerApp::new(transport);
            let (verify_key, output) = sign_detached_prepared_update_with_ledger(
                &mut app,
                &prepared,
                key_index,
                args.non_interactive,
            )?;
            Ok(signature_file_from_signer_output(verify_key, output))
        })();
        spin.clear();
        result?
    } else {
        ensure_governance_keys_available_for_listing(
            conn,
            &network_name,
            &network_entry.genesis_hash,
        )?;
        let password_value = password(format!("Governance vault password for '{}':", network_name))
            .allow_empty()
            .interact()?;
        let vault = governance::unlock_vault(conn, &network_entry.genesis_hash, &password_value)?;
        let decrypted = governance::decrypted_keys(conn, &network_entry.genesis_hash, &vault.dek)?;
        let selected = resolve_proposal_local_signer(
            args.key.as_deref(),
            args.non_interactive,
            &decrypted,
            &prepared,
        )?;
        let verify_key = governance::public_key_hex(&selected.public_key);
        let signer_context = local_detached_signer_context(&verify_key);
        if !approve_governance_update_review(
            "detached signing",
            &network_name,
            &endpoint_label,
            &prepared,
            &signer_context,
            args.non_interactive,
        )? {
            return Ok(());
        }
        let output = sign_detached_prepared_update_with_local_key(&prepared, &selected)?;
        signature_file_from_signer_output(verify_key, output)
    };
    write_governance_signature_file(&args.out, &signature)?;
    println!("Wrote governance signature to {}.", args.out.display());
    Ok(())
}

fn resolve_proposal_local_signer(
    key: Option<&str>,
    non_interactive: bool,
    decrypted: &[governance::DecryptedGovernanceKey],
    prepared: &PreparedGovernanceUpdate,
) -> Result<governance::DecryptedGovernanceKey> {
    if decrypted.is_empty() {
        bail!("no governance keys are stored for the selected network");
    }
    if let Some(verify_key) = key {
        return find_decrypted_key_by_verify_key(decrypted, verify_key)
            .with_context(|| format!("governance key '{verify_key}' is not stored locally"));
    }
    if non_interactive {
        bail!("`--key <VERIFY_KEY>` must be provided in --non-interactive mode");
    }
    let entries = governance_signer_entries(
        decrypted,
        &prepared.chain_context.chain_parameters,
        prepared.payload.auth_family_hint(),
    );
    let items = entries
        .iter()
        .map(|entry| SelectItem {
            value: entry.list_entry.verify_key.clone(),
            label: render_governance_list_row(&entry.list_entry, true),
            hint: entry.list_entry.detail().unwrap_or_default(),
        })
        .collect::<Vec<_>>();
    let selected = select_or_single("Select governance proposal signer", &items, None)?;
    find_decrypted_key_by_verify_key(decrypted, &selected)
        .with_context(|| format!("selected governance key '{selected}' is not stored locally"))
}

async fn submit_proposal(conn: &mut Connection, args: GovernanceProposalSubmitArgs) -> Result<()> {
    let prepared_context = PreparedGovernanceContext::from_flags(
        args.network.clone(),
        args.non_interactive,
        args.no_defaults,
        args.no_wait,
    );
    let (network_name, network_entry, endpoint_label, source) =
        prepared_context.resolve_network(conn).await?;
    log_resolved_context(&[ContextLine {
        label: "network:",
        value: format!("{network_name} @ {endpoint_label}"),
        source,
    }])?;
    let (_proposal, prepared) = load_and_prepare_proposal(
        &network_entry.node_endpoint,
        &endpoint_label,
        &network_entry.genesis_hash,
        &args.proposal,
    )
    .await?;
    let signature_paths = collect_signature_files(&args.signatures, args.signature_dir.as_deref())?;
    if signature_paths.is_empty() {
        bail!("at least one detached governance signature file is required");
    }
    let mut outputs = Vec::new();
    let mut seen_indices = BTreeSet::new();
    for path in signature_paths {
        let signature = read_governance_signature_file(&path)?;
        let output = signer_output_from_signature_file(&signature)?;
        let expected_index = signer_index_for_verify_key(&prepared, &signature.verify_key)?;
        if output.index != expected_index {
            bail!(
                "detached signature {} stores index {}, but verify key '{}' currently maps to index {}",
                path.display(),
                output.index,
                signature.verify_key,
                expected_index
            );
        }
        verify_detached_signature(&prepared, &output)?;
        if !seen_indices.insert(output.index) {
            bail!(
                "duplicate detached governance signature for key index {}",
                output.index
            );
        }
        outputs.push(output);
    }
    ensure_signature_threshold(&prepared, outputs.len())?;
    let signer_context = detached_submission_signer_context(&outputs);
    if !approve_governance_update_review(
        "submission",
        &network_name,
        &endpoint_label,
        &prepared,
        &signer_context,
        args.non_interactive,
    )? {
        return Ok(());
    }
    let block_item = assemble_signed_update_instruction(&prepared, outputs)?;
    let transaction_hash =
        submit_governance_update(&network_entry.node_endpoint, &endpoint_label, &block_item)
            .await?;
    let message = format!("Submitted governance update: {transaction_hash}");
    if !prepared_context.finalization.should_wait() {
        println!("{message}");
        return Ok(());
    }
    let _ = cliclack::log::success(message);
    wait_for_governance_update_finalization(
        &network_entry.node_endpoint,
        &endpoint_label,
        &transaction_hash,
    )
    .await?;
    Ok(())
}

fn collect_signature_files(explicit: &[PathBuf], dir: Option<&Path>) -> Result<Vec<PathBuf>> {
    let mut files = explicit.to_vec();
    if let Some(dir) = dir {
        if !dir.is_dir() {
            bail!("signature directory does not exist: {}", dir.display());
        }
        let mut directory_files = fs::read_dir(dir)
            .with_context(|| format!("failed to read signature directory {}", dir.display()))?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .with_context(|| format!("failed to read entry in {}", dir.display()))
            })
            .collect::<Result<Vec<_>>>()?;
        directory_files
            .retain(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"));
        directory_files.sort();
        files.extend(directory_files);
    }
    Ok(files)
}

fn validate_proposal_sequence(
    proposal: &GovernanceProposalFile,
    queue: GovernanceSequenceQueue,
    live_sequence: UpdateSequenceNumber,
) -> Result<UpdateSequenceNumber> {
    let proposal_sequence = UpdateSequenceNumber {
        number: proposal.header.seq_number,
    };
    if live_sequence != proposal_sequence {
        bail!(
            "governance proposal is stale for '{}' updates: proposal sequence {}, current next sequence {}",
            queue.label(),
            proposal_sequence,
            live_sequence
        );
    }
    Ok(proposal_sequence)
}

fn verify_detached_signature(
    prepared: &PreparedGovernanceUpdate,
    output: &GovernanceSignerOutput,
) -> Result<()> {
    let family = prepared
        .payload
        .auth_family_hint()
        .context("governance proposal requires a known authorization family")?;
    let authorization =
        governance_authorization_for_family(&prepared.chain_context.chain_parameters, family)?;
    let public_key = authorization
        .keys
        .get(usize::from(output.index.index))
        .with_context(|| {
            format!(
                "governance signature index {} is out of range",
                output.index
            )
        })?;
    if !authorization.authorized_indices.contains(&output.index) {
        bail!(
            "governance signature index {} is not authorized for '{}' updates",
            output.index,
            family.label()
        );
    }
    let signature = ed25519_dalek::Signature::from_slice(&output.signature.sig)
        .context("detached governance signature is not a valid Ed25519 signature")?;
    let hash = compute_update_sign_hash(&prepared.header, &prepared.encoded_payload);
    match &public_key.public {
        concordium_rust_sdk::id::types::VerifyKey::Ed25519VerifyKey(key) => {
            ed25519_dalek::Verifier::verify(key, hash.as_ref(), &signature)
                .context("detached governance signature is not valid for the proposal")?;
        }
    }
    Ok(())
}

fn ensure_signature_threshold(prepared: &PreparedGovernanceUpdate, count: usize) -> Result<()> {
    let family = prepared
        .payload
        .auth_family_hint()
        .context("governance proposal requires a known authorization family")?;
    let authorization =
        governance_authorization_for_family(&prepared.chain_context.chain_parameters, family)?;
    if count < authorization.threshold {
        bail!(
            "provided {} valid governance signature(s), but '{}' updates require threshold {}",
            count,
            family.label(),
            authorization.threshold
        );
    }
    Ok(())
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
    let prepared_context = PreparedGovernanceContext::from_flags(
        args.network.clone(),
        args.non_interactive,
        true,
        false,
    );
    let (network_name, network_entry, endpoint_label, source) =
        prepared_context.resolve_network(conn).await?;
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

    let verify_keys = Promptable::from_option(
        args.verify_key.map(|verify_key| vec![verify_key]),
        "verify key",
    )
    .resolve_with_async(prepared_context.input_mode, || async {
        let decrypted = governance::decrypted_keys(conn, &network_entry.genesis_hash, &vault.dek)?;
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
        Ok(selected)
    })
    .await
    .with_context(
        || "verify key must be provided in --non-interactive mode unless `--all` is used",
    )?
    .into_value();

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
        None if allow_active_default => Defaultable::Missing {
                value_name: "network",
            }
            .resolve_with_default_or_prompt(
                InputMode::from_flags(non_interactive, false),
                || wallet_state::get(conn, wallet_state::ACTIVE_NETWORK_KEY),
                || prompt_for_network_name(&app_config, None),
            )
            .map(|resolved| {
                let source = match resolved.source {
                    crate::commands::input::ResolvedSource::Default => ResolutionSource::ActiveDefault,
                    crate::commands::input::ResolvedSource::Prompt => ResolutionSource::Prompted,
                    crate::commands::input::ResolvedSource::Explicit => ResolutionSource::Explicit,
                };
                (resolved.value, source)
            })
            .with_context(
                || "no active network is set; provide `--network` or run `ccd-wallet network use <NAME>`",
            )?,
        None => {
            let active = wallet_state::get(conn, wallet_state::ACTIVE_NETWORK_KEY)?;
            let selected_name = Promptable::Missing {
                value_name: "network",
            }
                .resolve_with(InputMode::from_flags(non_interactive, false), || {
                    prompt_for_network_name(&app_config, active.as_deref())
                })
                .with_context(|| "network must be provided with `--network <NAME>` in --non-interactive mode")?
                .into_value();
            (selected_name, ResolutionSource::Prompted)
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
            ledger: false,
            ledger_key_index: None,
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

    fn prepared_protocol_update() -> PreparedGovernanceUpdate {
        let payload = ResolvedGovernanceUpdatePayload::Known {
            payload: protocol_payload(),
            auth_family: GovernanceAuthFamily::Level2(GovernanceCapability::Protocol),
            sequence_queue: GovernanceSequenceQueue::Protocol,
        };
        prepare_governance_update(
            &payload,
            &GovernanceUpdateChainContext {
                chain_parameters: ChainParameters::default(),
                sequence_number: Some(7u64.into()),
                sequence_number_source: Some(ResolutionSource::Inferred),
            },
            GovernanceUpdateTiming {
                effective_time_seconds: 0,
                timeout_seconds: 1_800_000_000,
            },
        )
        .unwrap()
    }

    #[test]
    fn governance_review_renders_parsed_payload_details() {
        let prepared = prepared_protocol_update();
        let signer_context = ledger_update_signer_context(3);
        let review = render_governance_update_review(
            "detached signing",
            "testnet",
            "https://node.example",
            &prepared,
            &signer_context,
        )
        .unwrap();
        assert!(review.contains("Governance update detached signing"));
        assert!(review.contains("testnet @ https://node.example"));
        assert!(review.contains("payload: UpdateProtocol"));
        assert!(review.contains("sequence number: 7"));
        assert!(review.contains("Ledger key index: 3"));
        assert!(review.contains("Payload details:"));
        assert!(review.contains("https://example.com/update"));
    }

    #[test]
    fn governance_review_warns_for_blind_payloads() {
        let payload = ResolvedGovernanceUpdatePayload::Blind {
            bytes: vec![0xaa, 0xbb],
            auth_family_hint: Some(GovernanceAuthFamily::Level2(
                GovernanceCapability::CreatePlt,
            )),
            sequence_queue_hint: Some(GovernanceSequenceQueue::ProtocolLevelTokens),
        };
        let prepared = prepare_governance_update(
            &payload,
            &GovernanceUpdateChainContext {
                chain_parameters: ChainParameters::default(),
                sequence_number: Some(9u64.into()),
                sequence_number_source: Some(ResolutionSource::Explicit),
            },
            GovernanceUpdateTiming {
                effective_time_seconds: 0,
                timeout_seconds: 1_800_000_000,
            },
        )
        .unwrap();
        let signer_context = local_detached_signer_context("abcd");
        let review = render_governance_update_review(
            "submission",
            "testnet",
            "endpoint",
            &prepared,
            &signer_context,
        )
        .unwrap();
        assert!(review.contains("blind serialized"));
        assert!(review.contains("semantic details are unavailable"));
        assert!(review.contains("raw payload bytes: 2"));
        assert!(review.contains("independently reviewed"));
    }

    #[test]
    fn non_interactive_review_approval_skips_prompt() {
        let prepared = prepared_protocol_update();
        let signer_context = detached_submission_signer_context(&[GovernanceSignerOutput {
            index: UpdateKeysIndex { index: 1 },
            signature: Signature { sig: vec![0; 64] },
        }]);
        let approved = approve_governance_update_review(
            "submission",
            "testnet",
            "endpoint",
            &prepared,
            &signer_context,
            true,
        )
        .unwrap();
        assert!(approved);
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
    fn proposal_file_round_trips_as_canonical_json() {
        let payload = ResolvedGovernanceUpdatePayload::Known {
            payload: protocol_payload(),
            auth_family: GovernanceAuthFamily::Level2(GovernanceCapability::Protocol),
            sequence_queue: GovernanceSequenceQueue::Protocol,
        };
        let prepared = prepare_governance_update(
            &payload,
            &GovernanceUpdateChainContext {
                chain_parameters: ChainParameters::default(),
                sequence_number: Some(7u64.into()),
                sequence_number_source: None,
            },
            GovernanceUpdateTiming {
                effective_time_seconds: 0,
                timeout_seconds: 1_800_000_000,
            },
        )
        .unwrap();
        let proposal = proposal_file_from_prepared(&prepared, "genesis").unwrap();
        let json = serde_json::to_string_pretty(&proposal).unwrap();
        assert!(json.contains("\n  \"version\": 1,"));
        let decoded: GovernanceProposalFile = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.version, 1);
        assert_eq!(decoded.genesis_hash, "genesis");
        let round_tripped = prepared_update_from_proposal(
            &decoded,
            GovernanceUpdateChainContext {
                chain_parameters: ChainParameters::default(),
                sequence_number: Some(7u64.into()),
                sequence_number_source: None,
            },
        )
        .unwrap();
        assert_eq!(round_tripped.header.seq_number, prepared.header.seq_number);
        assert_eq!(
            round_tripped.encoded_payload.as_ref(),
            prepared.encoded_payload.as_ref()
        );
    }

    #[test]
    fn proposal_payload_size_mismatch_is_rejected() {
        let payload = ResolvedGovernanceUpdatePayload::Known {
            payload: protocol_payload(),
            auth_family: GovernanceAuthFamily::Level2(GovernanceCapability::Protocol),
            sequence_queue: GovernanceSequenceQueue::Protocol,
        };
        let prepared = prepare_governance_update(
            &payload,
            &GovernanceUpdateChainContext {
                chain_parameters: ChainParameters::default(),
                sequence_number: Some(7u64.into()),
                sequence_number_source: None,
            },
            GovernanceUpdateTiming {
                effective_time_seconds: 0,
                timeout_seconds: 1_800_000_000,
            },
        )
        .unwrap();
        let mut proposal = proposal_file_from_prepared(&prepared, "genesis").unwrap();
        proposal.header.payload_size += 1;
        let err = prepared_update_from_proposal(
            &proposal,
            GovernanceUpdateChainContext {
                chain_parameters: ChainParameters::default(),
                sequence_number: Some(7u64.into()),
                sequence_number_source: None,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("payload size mismatch"));
    }

    #[test]
    fn stale_proposal_sequence_is_rejected() {
        let proposal = GovernanceProposalFile {
            version: 1,
            genesis_hash: "genesis".to_owned(),
            header: GovernanceProposalHeaderJson {
                seq_number: 7,
                effective_time: 0,
                timeout: 1_800_000_000,
                payload_size: 1,
            },
            payload: serde_json::json!({}),
        };
        let err = validate_proposal_sequence(
            &proposal,
            GovernanceSequenceQueue::Protocol,
            UpdateSequenceNumber { number: 8 },
        )
        .unwrap_err();
        assert!(err.to_string().contains("stale"));
    }

    #[test]
    fn detached_signature_file_converts_to_single_signer_output() {
        let output = GovernanceSignerOutput {
            index: UpdateKeysIndex { index: 3 },
            signature: Signature { sig: vec![1, 2, 3] },
        };
        let file = signature_file_from_signer_output("abcd".to_owned(), output.clone());
        let json = serde_json::to_string_pretty(&file).unwrap();
        assert!(json.contains("\"verifyKey\": \"abcd\""));
        assert_eq!(signer_output_from_signature_file(&file).unwrap(), output);
    }

    #[test]
    fn signature_directory_collects_json_files_after_explicit_files() {
        let temp = std::env::temp_dir().join(format!("gov-sigs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();
        let explicit = temp.join("explicit.sig");
        let a = temp.join("a.json");
        let b = temp.join("b.json");
        std::fs::write(&explicit, "{}").unwrap();
        std::fs::write(&b, "{}").unwrap();
        std::fs::write(&a, "{}").unwrap();
        std::fs::write(temp.join("ignore.txt"), "{}").unwrap();
        let files = collect_signature_files(std::slice::from_ref(&explicit), Some(&temp)).unwrap();
        assert_eq!(files, vec![explicit, a, b]);
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn ledger_signing_rejects_blind_payloads() {
        let payload = ResolvedGovernanceUpdatePayload::Blind {
            bytes: vec![0xff],
            auth_family_hint: Some(GovernanceAuthFamily::Level2(GovernanceCapability::Protocol)),
            sequence_queue_hint: Some(GovernanceSequenceQueue::Protocol),
        };
        let err = validate_ledger_signing_context(&payload).unwrap_err();
        assert!(err.to_string().contains("does not support blind signing"));
    }

    #[test]
    fn ledger_path_is_derived_from_update_family_and_key_index() {
        let payload = ResolvedGovernanceUpdatePayload::Known {
            payload: protocol_payload(),
            auth_family: GovernanceAuthFamily::Level2(GovernanceCapability::Protocol),
            sequence_queue: GovernanceSequenceQueue::Protocol,
        };
        let prepared = prepare_governance_update(
            &payload,
            &GovernanceUpdateChainContext {
                chain_parameters: ChainParameters::default(),
                sequence_number: Some(7u64.into()),
                sequence_number_source: None,
            },
            GovernanceUpdateTiming {
                effective_time_seconds: 0,
                timeout_seconds: 1_800_000_000,
            },
        )
        .unwrap();
        let path = ledger_derivation_path_for_prepared_update(&prepared, 9).unwrap();
        assert_eq!(path.indices(), &[1105, 0, 1, 2, 9]);
    }

    #[test]
    fn detached_local_signing_allows_threshold_above_one_and_verifies_signature() {
        let signer = key();
        let other = key();
        let local = governance_store::DecryptedGovernanceKey {
            record: governance_store::GovernanceKeyRecord {
                id: 1,
                network_genesis_hash: "g".to_owned(),
                vault_id: "v".to_owned(),
                created_at: 0,
                updated_at: 0,
            },
            raw_json: serde_json::to_string(&signer).unwrap(),
            public_key: UpdatePublicKey::from(&signer),
            key_pair: signer.clone(),
        };
        let params = ChainParameters {
            keys: UpdateKeys {
                level_2_keys: Some(Level2Keys {
                    keys: vec![
                        UpdatePublicKey::from(&signer),
                        UpdatePublicKey::from(&other),
                    ],
                    protocol: Some(concordium_rust_sdk::base::updates::AccessStructure {
                        authorized_keys: [0u16.into(), 1u16.into()].into_iter().collect(),
                        threshold: 2u16.try_into().unwrap(),
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
        let payload = ResolvedGovernanceUpdatePayload::Known {
            payload: protocol_payload(),
            auth_family: GovernanceAuthFamily::Level2(GovernanceCapability::Protocol),
            sequence_queue: GovernanceSequenceQueue::Protocol,
        };
        let prepared = prepare_governance_update(
            &payload,
            &GovernanceUpdateChainContext {
                chain_parameters: params,
                sequence_number: Some(7u64.into()),
                sequence_number_source: None,
            },
            GovernanceUpdateTiming {
                effective_time_seconds: 0,
                timeout_seconds: 1_800_000_000,
            },
        )
        .unwrap();
        let output = sign_detached_prepared_update_with_local_key(&prepared, &local).unwrap();
        assert_eq!(output.index, UpdateKeysIndex { index: 0 });
        verify_detached_signature(&prepared, &output).unwrap();
        let err = ensure_signature_threshold(&prepared, 1).unwrap_err();
        assert!(err.to_string().contains("require threshold 2"));
    }

    #[test]
    fn detached_signature_verification_rejects_tampering() {
        let signer = key();
        let local = governance_store::DecryptedGovernanceKey {
            record: governance_store::GovernanceKeyRecord {
                id: 1,
                network_genesis_hash: "g".to_owned(),
                vault_id: "v".to_owned(),
                created_at: 0,
                updated_at: 0,
            },
            raw_json: serde_json::to_string(&signer).unwrap(),
            public_key: UpdatePublicKey::from(&signer),
            key_pair: signer.clone(),
        };
        let params = ChainParameters {
            keys: UpdateKeys {
                level_2_keys: Some(Level2Keys {
                    keys: vec![UpdatePublicKey::from(&signer)],
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
        let payload = ResolvedGovernanceUpdatePayload::Known {
            payload: protocol_payload(),
            auth_family: GovernanceAuthFamily::Level2(GovernanceCapability::Protocol),
            sequence_queue: GovernanceSequenceQueue::Protocol,
        };
        let prepared = prepare_governance_update(
            &payload,
            &GovernanceUpdateChainContext {
                chain_parameters: params,
                sequence_number: Some(7u64.into()),
                sequence_number_source: None,
            },
            GovernanceUpdateTiming {
                effective_time_seconds: 0,
                timeout_seconds: 1_800_000_000,
            },
        )
        .unwrap();
        let mut output = sign_detached_prepared_update_with_local_key(&prepared, &local).unwrap();
        output.signature.sig[0] ^= 0xff;
        let err = verify_detached_signature(&prepared, &output).unwrap_err();
        assert!(err.to_string().contains("not valid"));
    }

    #[test]
    fn ledger_threshold_above_one_is_rejected_before_signing() {
        let ledger_key = key();
        let public_key = UpdatePublicKey::from(&ledger_key);
        let params = ChainParameters {
            keys: UpdateKeys {
                level_2_keys: Some(Level2Keys {
                    keys: vec![public_key.clone()],
                    protocol: Some(concordium_rust_sdk::base::updates::AccessStructure {
                        authorized_keys: [0u16.into()].into_iter().collect(),
                        threshold: 2u16.try_into().unwrap(),
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
        let payload = ResolvedGovernanceUpdatePayload::Known {
            payload: protocol_payload(),
            auth_family: GovernanceAuthFamily::Level2(GovernanceCapability::Protocol),
            sequence_queue: GovernanceSequenceQueue::Protocol,
        };
        let prepared = prepare_governance_update(
            &payload,
            &GovernanceUpdateChainContext {
                chain_parameters: params,
                sequence_number: Some(7u64.into()),
                sequence_number_source: None,
            },
            GovernanceUpdateTiming {
                effective_time_seconds: 0,
                timeout_seconds: 1_800_000_000,
            },
        )
        .unwrap();
        let err = ledger_signer_index_for_public_key(&prepared, &public_key).unwrap_err();
        assert!(err.to_string().contains("currently supports 1 signer"));
    }

    #[test]
    fn ledger_user_decline_error_is_actionable() {
        let err = ledger_error(governance_ledger::GovernanceLedgerError::UserDeclined);
        assert!(err.to_string().contains("declined on the device"));
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

        let payload = ResolvedGovernanceUpdatePayload::Known {
            payload: protocol_payload(),
            auth_family: GovernanceAuthFamily::Level2(GovernanceCapability::Protocol),
            sequence_queue: GovernanceSequenceQueue::Protocol,
        };
        let chain_context = GovernanceUpdateChainContext {
            chain_parameters: params,
            sequence_number: Some(7u64.into()),
            sequence_number_source: None,
        };
        let prepared = prepare_governance_update(
            &payload,
            &chain_context,
            GovernanceUpdateTiming {
                effective_time_seconds: 0,
                timeout_seconds: 1_800_000_000,
            },
        )
        .unwrap();
        let outputs = sign_prepared_update_with_local_keys(&prepared, &[local[1].clone()]).unwrap();
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].index, UpdateKeysIndex { index: 0 });
        let block_item = assemble_signed_update_instruction(&prepared, outputs).unwrap();
        assert!(matches!(block_item, BlockItem::UpdateInstruction(_)));
    }
}
