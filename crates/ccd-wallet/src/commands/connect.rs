use crate::{
    cli::ConnectArgs,
    commands::ui::{SelectItem, select_or_single},
};
use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use ccd_wallet_connect::{
    AccountRequest, AccountRequestApproval, ConnectServer, ContractExecutionRejection,
    ContractInitApproval, ContractInitRequest, ContractUpdateApproval, ContractUpdateRequest,
    PairingApproval, PairingRejection, PairingRequest,
};
use ccd_wallet_core::{
    config as node_config,
    store::{
        accounts::{self, AccountRecord, AccountSourceKind, AccountStatus},
        config::{self, NetworkEntry, load},
        seeds,
    },
    wallet::{ConcordiumHdWallet, Net},
};
use cliclack::{input, password};
use concordium_rust_sdk::{
    common::types::{AccountAddress, Amount, KeyIndex, KeyPair, TransactionTime},
    contract_client::ContractInitBuilder,
    id::types::{AccountKeys, CredentialData, SignatureThreshold},
    smart_contracts::common::{
        ContractAddress as SdkContractAddress, ModuleReference, OwnedContractName, OwnedParameter,
        OwnedReceiveName,
        schema::{Type as SchemaType, VersionedModuleSchema},
    },
    types::{
        Energy, WalletAccount,
        smart_contracts::ContractContext,
        transactions::{InitContractPayload, UpdateContractPayload, send},
    },
    v2,
};
use futures_util::FutureExt;
use rusqlite::Connection;
use std::{
    collections::BTreeMap,
    str::FromStr,
    sync::{Arc, Mutex},
};
use tokio::sync::oneshot;

pub async fn run(conn: Connection, args: ConnectArgs) -> Result<()> {
    let conn = Arc::new(Mutex::new(conn));
    let server_conn = Arc::clone(&conn);
    let account_conn = Arc::clone(&conn);
    let init_conn = Arc::clone(&conn);
    let update_conn = Arc::clone(&conn);
    let server = ConnectServer::new(
        move |request| {
            let conn = Arc::clone(&server_conn);
            async move {
                let conn = conn.lock().map_err(|_| {
                    PairingRejection::new("wallet database connection is unavailable")
                })?;
                match approve_pairing(&conn, request) {
                    Ok(approval) => Ok(approval),
                    Err(err) => {
                        let message = err.to_string();
                        let _ = cliclack::log::error(&message);
                        Err(PairingRejection::new(message))
                    }
                }
            }
            .boxed()
        },
        move |request| {
            let conn = Arc::clone(&account_conn);
            async move {
                let conn = conn.lock().map_err(|_| {
                    PairingRejection::new("wallet database connection is unavailable")
                })?;
                match approve_account_request(&conn, request) {
                    Ok(approval) => Ok(approval),
                    Err(err) => {
                        let message = err.to_string();
                        let _ = cliclack::log::error(&message);
                        Err(PairingRejection::new(message))
                    }
                }
            }
            .boxed()
        },
        move |request| {
            let conn = Arc::clone(&init_conn);
            async move {
                let prepared = prepare_contract_init_request(request)?;
                submit_contract_init_request(Arc::clone(&conn), prepared).await
            }
            .boxed()
        },
        move |request| {
            let conn = Arc::clone(&update_conn);
            async move {
                let prepared = prepare_contract_update_request(request)?;
                submit_contract_update_request(Arc::clone(&conn), prepared).await
            }
            .boxed()
        },
    );

    println!(
        "Starting ccd-wallet browser pairing session on ws://{}",
        args.bind
    );
    println!("Press Ctrl-C to stop the connect session.");

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        let _ = shutdown_tx.send(());
    });

    server.serve(args.bind, shutdown_rx).await
}

fn approve_pairing(_conn: &Connection, request: PairingRequest) -> Result<PairingApproval> {
    cliclack::log::info(format!(
        "Browser pairing request\norigin: {}",
        request.origin
    ))?;

    let expected_challenge = request.challenge.clone();
    let confirmation: String =
        input("Enter the six-digit challenge shown in the web application to approve pairing:")
            .validate(move |value: &String| {
                if value == &expected_challenge {
                    Ok(())
                } else {
                    Err("Challenge does not match.")
                }
            })
            .interact()?;
    if confirmation != request.challenge {
        bail!("pairing rejected because challenge confirmation did not match");
    }

    let (network_name, network_entry) = select_network()?;

    cliclack::log::success(format!(
        "Paired {} on network {}.",
        request.origin, network_name
    ))?;

    Ok(PairingApproval {
        network_genesis_hash: network_entry.genesis_hash,
    })
}

fn approve_account_request(
    conn: &Connection,
    request: AccountRequest,
) -> Result<AccountRequestApproval> {
    let network_name = resolve_network_display_name(&request.network_genesis_hash)?;
    cliclack::log::info(format!(
        "Account authority request\norigin: {}\nnetwork: {}",
        request.origin, network_name
    ))?;

    let account = select_account(conn, &request.network_genesis_hash)?;
    let account_address = read_account_address(conn, &account, &network_name)?;

    cliclack::log::success(format!(
        "Approved account authority {} for {} on network {}.",
        account_address, request.origin, network_name
    ))?;

    Ok(AccountRequestApproval { account_address })
}

fn resolve_network_display_name(network_genesis_hash: &str) -> Result<String> {
    let config = load()?;
    let matches = config::aliases_by_genesis_hash(&config, network_genesis_hash);
    if matches.is_empty() {
        Ok(network_genesis_hash.to_owned())
    } else {
        Ok(matches.join(", "))
    }
}

struct PreparedContractInit {
    request: ContractInitRequest,
    endpoint: v2::Endpoint,
    endpoint_label: String,
    network_name: String,
    sender: AccountAddress,
    payload: InitContractPayload,
    energy: Energy,
}

fn prepare_contract_init_request(
    request: ContractInitRequest,
) -> std::result::Result<PreparedContractInit, ContractExecutionRejection> {
    let (network_name, network_entry) = resolve_network_entry(&request.network_genesis_hash)
        .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let endpoint = v2::Endpoint::from_str(&network_entry.node_endpoint).map_err(|err| {
        ContractExecutionRejection::other(format!(
            "invalid node endpoint for {network_name}: {err}"
        ))
    })?;
    let endpoint_label = node_config::endpoint_label(&endpoint);
    let module_ref = ModuleReference::from_str(&request.module_ref)
        .map_err(|err| ContractExecutionRejection::other(format!("invalid moduleRef: {err}")))?;
    let payload = InitContractPayload {
        amount: parse_amount_micro_ccd(&request.amount_micro_ccd)
            .map_err(|err| ContractExecutionRejection::other(err.to_string()))?,
        mod_ref: module_ref,
        init_name: OwnedContractName::new(request.init_name.clone())
            .map_err(|err| ContractExecutionRejection::other(format!("invalid initName: {err}")))?,
        param: parse_parameter_hex(&request.parameter_hex)
            .map_err(|err| ContractExecutionRejection::other(err.to_string()))?,
    };
    let energy = Energy::from(request.max_contract_execution_energy);
    let sender = AccountAddress::from_str(&request.account_address).map_err(|err| {
        ContractExecutionRejection::other(format!("invalid session account address: {err}"))
    })?;
    Ok(PreparedContractInit {
        request,
        endpoint,
        endpoint_label,
        network_name,
        sender,
        payload,
        energy,
    })
}

async fn submit_contract_init_request(
    conn: Arc<Mutex<Connection>>,
    prepared: PreparedContractInit,
) -> std::result::Result<ContractInitApproval, ContractExecutionRejection> {
    let mut client = node_config::connect_v2_client(prepared.endpoint.clone())
        .await
        .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let simulation = if prepared.request.validate {
        let contract_name = prepared
            .request
            .init_name
            .strip_prefix("init_")
            .unwrap_or(&prepared.request.init_name);
        Some(
            match ContractInitBuilder::<()>::dry_run_new_instance_raw(
                client.clone(),
                prepared.sender,
                prepared.payload.mod_ref,
                contract_name,
                prepared.payload.amount,
                prepared.payload.param.clone(),
            )
            .await
            {
                Ok(builder) => format!(
                    "Simulation: contract init succeeded (estimated energy: {})",
                    builder.current_energy().energy
                ),
                Err(err) => format!("Simulation warning: {err}"),
            },
        )
    } else {
        None
    };
    print_contract_init_prompt(
        &prepared.request,
        &prepared.network_name,
        &prepared.endpoint_label,
        simulation.as_deref(),
    )
    .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let confirmation: String = input("Approve and submit this contract init? Type y to approve:")
        .default_input("n")
        .interact()
        .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    if !confirmation.eq_ignore_ascii_case("y") && !confirmation.eq_ignore_ascii_case("yes") {
        return Err(ContractExecutionRejection::user_declined(
            "contract init declined by user",
        ));
    }
    let (resolved_network_name, network_entry) =
        resolve_network_entry(&prepared.request.network_genesis_hash)
            .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let wallet = {
        let conn = conn.lock().map_err(|_| {
            ContractExecutionRejection::other("wallet database connection is unavailable")
        })?;
        unlock_wallet_account(
            &conn,
            &resolved_network_name,
            &network_entry,
            &prepared.request.account_address,
        )
        .map_err(|err| ContractExecutionRejection::other(err.to_string()))?
    };
    let nonce = client
        .get_next_account_sequence_number(&wallet.address)
        .await
        .map_err(|err| ContractExecutionRejection::submission_failed(err.to_string()))?
        .nonce;
    let expiry = TransactionTime::from_seconds((chrono::Utc::now().timestamp() + 300) as u64);
    let tx = send::init_contract(
        &wallet,
        wallet.address,
        nonce,
        expiry,
        prepared.payload,
        prepared.energy,
    );
    let transaction_hash = client
        .send_account_transaction(tx)
        .await
        .map_err(|err| ContractExecutionRejection::submission_failed(err.to_string()))?;
    let transaction_hash_label = transaction_hash.to_string();
    cliclack::log::success(format!(
        "Submitted contract init transaction on {} ({}): {transaction_hash_label}",
        prepared.network_name, prepared.endpoint_label
    ))
    .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let endpoint = prepared.endpoint;
    tokio::spawn(async move {
        match node_config::connect_v2_client(endpoint).await {
            Ok(mut client) => match client.wait_until_finalized(&transaction_hash).await {
                Ok((block_hash, summary)) => {
                    if let Some(event) = summary.contract_init() {
                        println!(
                            "Contract init finalized in block {block_hash}. New contract address: <{}, {}>. Outcome: {:#?}",
                            event.address.index, event.address.subindex, summary.details
                        );
                    } else {
                        println!(
                            "Contract init finalized in block {block_hash}. Outcome: {:#?}",
                            summary.details
                        );
                    }
                }
                Err(err) => eprintln!("Failed while waiting for contract init finalization: {err}"),
            },
            Err(err) => eprintln!("Failed to reconnect for contract init finalization: {err}"),
        }
    });
    Ok(ContractInitApproval {
        transaction_hash: transaction_hash_label,
    })
}

struct PreparedContractUpdate {
    request: ContractUpdateRequest,
    endpoint: v2::Endpoint,
    endpoint_label: String,
    network_name: String,
    sender: AccountAddress,
    payload: UpdateContractPayload,
    energy: Energy,
}

fn prepare_contract_update_request(
    request: ContractUpdateRequest,
) -> std::result::Result<PreparedContractUpdate, ContractExecutionRejection> {
    let (network_name, network_entry) = resolve_network_entry(&request.network_genesis_hash)
        .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let endpoint = v2::Endpoint::from_str(&network_entry.node_endpoint).map_err(|err| {
        ContractExecutionRejection::other(format!(
            "invalid node endpoint for {network_name}: {err}"
        ))
    })?;
    let endpoint_label = node_config::endpoint_label(&endpoint);
    let sender = AccountAddress::from_str(&request.account_address).map_err(|err| {
        ContractExecutionRejection::other(format!("invalid session account address: {err}"))
    })?;
    let amount = parse_amount_micro_ccd(&request.amount_micro_ccd)
        .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let parameter = parse_parameter_hex(&request.parameter_hex)
        .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let payload = UpdateContractPayload {
        amount,
        address: SdkContractAddress::new(
            request.contract_address.index,
            request.contract_address.subindex,
        ),
        receive_name: OwnedReceiveName::new(request.receive_name.clone()).map_err(|err| {
            ContractExecutionRejection::other(format!("invalid receive name: {err}"))
        })?,
        message: parameter,
    };
    let energy = Energy::from(request.max_contract_execution_energy);
    Ok(PreparedContractUpdate {
        request,
        endpoint,
        endpoint_label,
        network_name,
        sender,
        payload,
        energy,
    })
}

async fn submit_contract_update_request(
    conn: Arc<Mutex<Connection>>,
    prepared: PreparedContractUpdate,
) -> std::result::Result<ContractUpdateApproval, ContractExecutionRejection> {
    let mut client = node_config::connect_v2_client(prepared.endpoint.clone())
        .await
        .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let simulation = if prepared.request.validate {
        let context = ContractContext::new_from_payload(
            prepared.sender,
            prepared.energy,
            prepared.payload.clone(),
        );
        Some(
            match client
                .invoke_instance(v2::BlockIdentifier::Best, &context)
                .await
            {
                Ok(response) => format!(
                    "Simulation: {:?} (used energy: {})",
                    response.response,
                    response.response.used_energy().energy
                ),
                Err(err) => format!("Simulation warning: {err}"),
            },
        )
    } else {
        None
    };
    print_contract_update_prompt(
        &prepared.request,
        &prepared.network_name,
        &prepared.endpoint_label,
        simulation.as_deref(),
    )
    .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let confirmation: String = input("Approve and submit this contract update? Type y to approve:")
        .default_input("n")
        .interact()
        .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    if !confirmation.eq_ignore_ascii_case("y") && !confirmation.eq_ignore_ascii_case("yes") {
        return Err(ContractExecutionRejection::user_declined(
            "contract update declined by user",
        ));
    }
    let (resolved_network_name, network_entry) =
        resolve_network_entry(&prepared.request.network_genesis_hash)
            .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;
    let wallet = {
        let conn = conn.lock().map_err(|_| {
            ContractExecutionRejection::other("wallet database connection is unavailable")
        })?;
        unlock_wallet_account(
            &conn,
            &resolved_network_name,
            &network_entry,
            &prepared.request.account_address,
        )
        .map_err(|err| ContractExecutionRejection::other(err.to_string()))?
    };
    let nonce = client
        .get_next_account_sequence_number(&wallet.address)
        .await
        .map_err(|err| ContractExecutionRejection::submission_failed(err.to_string()))?
        .nonce;
    let expiry = TransactionTime::from_seconds((chrono::Utc::now().timestamp() + 300) as u64);
    let tx = send::update_contract(
        &wallet,
        wallet.address,
        nonce,
        expiry,
        prepared.payload,
        prepared.energy,
    );
    let transaction_hash = client
        .send_account_transaction(tx)
        .await
        .map_err(|err| ContractExecutionRejection::submission_failed(err.to_string()))?;
    let transaction_hash_label = transaction_hash.to_string();
    cliclack::log::success(format!(
        "Submitted contract update transaction on {} ({}): {transaction_hash_label}",
        prepared.network_name, prepared.endpoint_label
    ))
    .map_err(|err| ContractExecutionRejection::other(err.to_string()))?;

    let endpoint = prepared.endpoint;
    tokio::spawn(async move {
        match node_config::connect_v2_client(endpoint).await {
            Ok(mut client) => match client.wait_until_finalized(&transaction_hash).await {
                Ok((block_hash, summary)) => println!(
                    "Contract update finalized in block {block_hash}. Outcome: {:#?}",
                    summary.details
                ),
                Err(err) => {
                    eprintln!("Failed while waiting for contract update finalization: {err}")
                }
            },
            Err(err) => eprintln!("Failed to reconnect for contract update finalization: {err}"),
        }
    });

    Ok(ContractUpdateApproval {
        transaction_hash: transaction_hash_label,
    })
}

fn print_contract_init_prompt(
    request: &ContractInitRequest,
    network_name: &str,
    endpoint_label: &str,
    simulation: Option<&str>,
) -> Result<()> {
    let parameter_display = display_init_parameter(request);
    cliclack::log::info(format!(
        "Contract init request\norigin: {}\nnetwork: {} ({})\naccount: {}\nmodule: {}\ninit: {}\namount: {} microCCD\nmax energy: {}\nparameter: {}{}",
        request.origin,
        network_name,
        endpoint_label,
        request.account_address,
        request.module_ref,
        request.init_name,
        request.amount_micro_ccd,
        request.max_contract_execution_energy,
        parameter_display,
        simulation
            .map(|value| format!("\n{value}"))
            .unwrap_or_default()
    ))?;
    Ok(())
}

fn print_contract_update_prompt(
    request: &ContractUpdateRequest,
    network_name: &str,
    endpoint_label: &str,
    simulation: Option<&str>,
) -> Result<()> {
    let parameter_display = display_update_parameter(request);
    cliclack::log::info(format!(
        "Contract update request\norigin: {}\nnetwork: {} ({})\naccount: {}\ncontract: <{}, {}>\nreceive: {}\namount: {} microCCD\nmax energy: {}\nparameter: {}{}",
        request.origin,
        network_name,
        endpoint_label,
        request.account_address,
        request.contract_address.index,
        request.contract_address.subindex,
        request.receive_name,
        request.amount_micro_ccd,
        request.max_contract_execution_energy,
        parameter_display,
        simulation
            .map(|value| format!("\n{value}"))
            .unwrap_or_default()
    ))?;
    Ok(())
}

fn display_init_parameter(request: &ContractInitRequest) -> String {
    let contract_name = request
        .init_name
        .strip_prefix("init_")
        .unwrap_or(&request.init_name);
    display_parameter_with_schema(&request.parameter_hex, request.schema.as_ref(), |schema| {
        schema
            .get_init_param_schema(contract_name)
            .map_err(anyhow::Error::from)
    })
}

fn display_update_parameter(request: &ContractUpdateRequest) -> String {
    display_parameter_with_schema(&request.parameter_hex, request.schema.as_ref(), |schema| {
        let Some((contract_name, function_name)) = request.receive_name.split_once('.') else {
            bail!("receiveName must be fully qualified as '<contract>.<function>'");
        };
        schema
            .get_receive_param_schema(contract_name, function_name)
            .map_err(anyhow::Error::from)
    })
}

fn display_parameter_with_schema(
    parameter_hex: &str,
    schema: Option<&serde_json::Value>,
    schema_type: impl FnOnce(&VersionedModuleSchema) -> Result<SchemaType>,
) -> String {
    match try_display_parameter_with_schema(parameter_hex, schema, schema_type) {
        Ok(value) => value,
        Err(_) => format!("0x{parameter_hex}"),
    }
}

fn try_display_parameter_with_schema(
    parameter_hex: &str,
    schema: Option<&serde_json::Value>,
    schema_type: impl FnOnce(&VersionedModuleSchema) -> Result<SchemaType>,
) -> Result<String> {
    let schema_base64 = schema
        .and_then(schema_base64_value)
        .context("schema is not a base64-encoded versioned module schema")?;
    let schema_base64 = schema_base64.trim_end_matches('=');
    let module_schema = VersionedModuleSchema::from_base64_str(schema_base64)
        .or_else(|_| {
            let bytes = BASE64.decode(schema_base64_value(schema.unwrap()).unwrap_or_default())?;
            let encoded = BASE64.encode(bytes).trim_end_matches('=').to_owned();
            VersionedModuleSchema::from_base64_str(&encoded).map_err(anyhow::Error::from)
        })
        .context("failed to decode supplied module schema")?;
    let ty = schema_type(&module_schema).context("failed to resolve parameter schema")?;
    let bytes = hex::decode(parameter_hex).context("parameterHex is not valid hex")?;
    ty.to_json_string_pretty(&bytes)
        .map_err(|err| anyhow::anyhow!(err.to_string()))
}

fn schema_base64_value(value: &serde_json::Value) -> Option<&str> {
    value.as_str().or_else(|| {
        value
            .get("base64")
            .or_else(|| value.get("moduleSchema"))
            .or_else(|| value.get("schema"))
            .and_then(serde_json::Value::as_str)
    })
}

fn parse_amount_micro_ccd(value: &str) -> Result<Amount> {
    let micro_ccd = value
        .parse::<u64>()
        .with_context(|| format!("amountMicroCcd must be an unsigned integer, got '{value}'"))?;
    Ok(Amount::from_micro_ccd(micro_ccd))
}

fn parse_parameter_hex(value: &str) -> Result<OwnedParameter> {
    let bytes = if value.is_empty() {
        Vec::new()
    } else {
        hex::decode(value).with_context(|| "parameterHex must be lower- or upper-case hex")?
    };
    Ok(OwnedParameter::new_unchecked(bytes))
}

fn unlock_wallet_account(
    conn: &Connection,
    network_name: &str,
    network_entry: &NetworkEntry,
    account_address: &str,
) -> Result<WalletAccount> {
    let accounts = accounts::list(conn)?
        .into_iter()
        .filter(|record| record.network_genesis_hash == network_entry.genesis_hash)
        .filter(|record| record.status == AccountStatus::Finalized)
        .collect::<Vec<_>>();
    for account in accounts {
        let candidate =
            unlock_wallet_account_candidate(conn, network_name, network_entry, &account)?;
        if candidate.address.to_string() == account_address {
            return Ok(candidate);
        }
    }
    bail!("no finalized wallet account matches the session-bound account address")
}

fn unlock_wallet_account_candidate(
    conn: &Connection,
    network_name: &str,
    network_entry: &NetworkEntry,
    account: &AccountRecord,
) -> Result<WalletAccount> {
    match account.source_kind {
        AccountSourceKind::Derived => {
            unlock_derived_wallet_account(conn, network_name, network_entry, account)
        }
        AccountSourceKind::Imported => unlock_imported_wallet_account(conn, network_name, account),
    }
}

fn unlock_derived_wallet_account(
    conn: &Connection,
    network_name: &str,
    network_entry: &NetworkEntry,
    account: &AccountRecord,
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
    let seed_phrase =
        std::str::from_utf8(&unlocked.secret).context("seed phrase is not valid UTF-8")?;
    let net = infer_net(
        network_name,
        network_entry.wallet_proxy.as_deref(),
        &network_entry.node_endpoint,
    );
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
        address: AccountAddress::from_str(&payload.account_address)?,
        keys: AccountKeys::from(CredentialData {
            keys,
            threshold: SignatureThreshold::ONE,
        }),
    })
}

fn unlock_imported_wallet_account(
    conn: &Connection,
    network_name: &str,
    account: &AccountRecord,
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
    WalletAccount::from_json_value(serde_json::json!({
        "address": payload.account_address,
        "accountKeys": payload.account_keys,
    }))
    .context("failed to build signer for imported account")
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

fn select_network() -> Result<(String, NetworkEntry)> {
    let config = load()?;
    if config.networks.is_empty() {
        bail!("no networks are configured; run `ccd-wallet network add` first");
    }

    let items = config
        .networks
        .iter()
        .map(|(name, entry)| SelectItem {
            value: name.clone(),
            label: name.clone(),
            hint: entry.node_endpoint.clone(),
        })
        .collect::<Vec<_>>();
    let selected = select_or_single("Select network for browser session", &items, None)?;
    let entry = config
        .networks
        .get(&selected)
        .cloned()
        .context("selected network was not found")?;
    Ok((selected, entry))
}

#[allow(dead_code)]
fn resolve_network_entry(network_genesis_hash: &str) -> Result<(String, NetworkEntry)> {
    let config = load()?;
    config
        .networks
        .iter()
        .find(|(_, entry)| entry.genesis_hash == network_genesis_hash)
        .map(|(name, entry)| (name.clone(), entry.clone()))
        .with_context(|| {
            format!(
                "no registered network matches genesis hash {network_genesis_hash}; run `ccd-wallet network add` to register it"
            )
        })
}

fn select_account(conn: &Connection, network_genesis_hash: &str) -> Result<AccountRecord> {
    let accounts = accounts::list(conn)?
        .into_iter()
        .filter(|record| record.network_genesis_hash == network_genesis_hash)
        .filter(|record| record.status == AccountStatus::Finalized)
        .collect::<Vec<_>>();
    if accounts.is_empty() {
        bail!("no finalized accounts are available for the selected network");
    }

    let seed_labels = seeds::list(conn)?
        .into_iter()
        .map(|seed| (seed.id, seed.label))
        .collect::<std::collections::BTreeMap<_, _>>();
    let items = accounts
        .iter()
        .map(|record| SelectItem {
            value: record.id,
            label: render_account_label(record, &seed_labels),
            hint: account_hint(record),
        })
        .collect::<Vec<_>>();
    let selected = select_or_single("Select account authority for browser session", &items, None)?;
    accounts
        .into_iter()
        .find(|record| record.id == selected)
        .context("selected account was not found")
}

fn render_account_label(
    record: &AccountRecord,
    seed_labels: &std::collections::BTreeMap<String, String>,
) -> String {
    if record.source_kind == AccountSourceKind::Imported {
        format!("[imported] {}", record.label)
    } else {
        let seed_label = seed_labels
            .get(&record.seed_id)
            .map(String::as_str)
            .unwrap_or("<unknown-seed>");
        format!("[{seed_label}] {}", record.label)
    }
}

fn account_hint(record: &AccountRecord) -> String {
    match record.source_kind {
        AccountSourceKind::Imported => "imported account".to_owned(),
        AccountSourceKind::Derived => format!(
            "provider:{} identity:{} credential:{}",
            record.ip_identity, record.identity_index, record.credential_counter
        ),
    }
}

fn read_account_address(
    conn: &Connection,
    account: &AccountRecord,
    network_name: &str,
) -> Result<String> {
    match account.source_kind {
        AccountSourceKind::Derived => {
            let seed = seeds::list(conn)?
                .into_iter()
                .find(|seed| seed.id == account.seed_id)
                .context("selected account references unknown seed")?;
            let password: String = password(format!("Password for seed '{}':", seed.label))
                .allow_empty()
                .interact()?;
            let unlocked = seeds::unlock_context(conn, &seed.label, &password)?;
            let payload = accounts::decrypt_private_payload(conn, account.id, &unlocked.dek)?;
            Ok(payload.account_address)
        }
        AccountSourceKind::Imported => {
            let vault_password: String = password(format!(
                "Imported accounts vault password for '{}':",
                network_name
            ))
            .allow_empty()
            .interact()?;
            let unlocked = accounts::unlock_imported_vault(
                conn,
                &account.network_genesis_hash,
                &vault_password,
            )?;
            let payload = accounts::decrypt_imported_payload(conn, account.id, &unlocked.dek)?;
            Ok(payload.account_address)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use concordium_rust_sdk::smart_contracts::common::{
        schema::{ContractV1, FunctionV1, ModuleV1, Type, VersionedModuleSchema},
        to_bytes,
    };

    fn isolated_home(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "ccd-wallet-connect-test-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(path.join(".config/ccd-wallet")).unwrap();
        unsafe {
            std::env::set_var("HOME", &path);
        }
        path
    }

    fn write_config(home: &std::path::Path, body: &str) {
        std::fs::write(home.join(".config/ccd-wallet/config.json"), body).unwrap();
    }

    #[test]
    fn resolves_network_entry_by_genesis_hash() {
        let home = isolated_home("resolve-network");
        write_config(
            &home,
            r#"{
                "version": 1,
                "networks": {
                    "alpha": { "node_endpoint": "http://alpha.example", "genesis_hash": "same", "wallet_proxy": null },
                    "beta": { "node_endpoint": "http://beta.example", "genesis_hash": "same", "wallet_proxy": null },
                    "gamma": { "node_endpoint": "http://gamma.example", "genesis_hash": "other", "wallet_proxy": null }
                }
            }"#,
        );

        let (name, entry) = resolve_network_entry("same").unwrap();
        assert_eq!(name, "alpha");
        assert_eq!(entry.node_endpoint, "http://alpha.example");
        assert!(resolve_network_entry("missing").is_err());
    }

    #[test]
    fn parameter_display_decodes_with_schema_and_falls_back_to_hex() {
        let mut receive = BTreeMap::new();
        receive.insert("set".to_owned(), FunctionV1::Parameter(Type::U8));
        let mut contracts = BTreeMap::new();
        contracts.insert(
            "my".to_owned(),
            ContractV1 {
                init: Some(FunctionV1::Parameter(Type::U8)),
                receive,
            },
        );
        let schema = VersionedModuleSchema::V1(ModuleV1 { contracts });
        let schema_base64 = BASE64.encode(to_bytes(&schema));

        let update = ContractUpdateRequest {
            origin: "https://example.com".to_owned(),
            network_genesis_hash: "genesis".to_owned(),
            account_address: "addr".to_owned(),
            contract_address: ccd_wallet_connect::ContractAddress {
                index: 1,
                subindex: 0,
            },
            receive_name: "my.set".to_owned(),
            amount_micro_ccd: "0".to_owned(),
            max_contract_execution_energy: 1000,
            parameter_hex: "2a".to_owned(),
            schema: Some(serde_json::json!({ "base64": schema_base64 })),
            validate: false,
        };
        assert_eq!(display_update_parameter(&update), "42");

        let mut no_schema = update;
        no_schema.schema = None;
        assert_eq!(display_update_parameter(&no_schema), "0x2a");
    }
}
