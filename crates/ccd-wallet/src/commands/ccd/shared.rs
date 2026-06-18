//! Shared native CCD command helpers.

use crate::{
    commands::{
        account::{
            self, AccountReferenceContext, AccountReferenceUnlocks, ResolvedAccountSelection,
            ResolvedNetworkContext, build_export_wallet_account_with_unlocks,
            local_account_context_lines, resolve_account_reference,
            resolve_signing_account_context,
        },
        input::{AccountLabel, AccountReference, FinalizationPolicy, InputMode, NetworkName},
        ledger,
        transaction::render::render_finalized_summary,
        ui::{ContextLine, log_resolved_context},
    },
    smart_contracts::shared::parse_decimal_ccd_amount,
};
use anyhow::{Context, Result};
use ccd_wallet_core::{
    config as node_config,
    store::{
        accounts,
        signer_owners::{self, SignerOwnerKind},
    },
};
use ccd_wallet_ledger::{
    ChunkedSigningRequest, ConcordiumLedgerApp, DerivationPath, HidTransport, RawSignature,
    ScheduledTransferSigningRequest, ScheduledTransferWithMemoSigningRequest,
    TransferWithMemoSigningRequest,
};
use chrono::SecondsFormat;
use cliclack::{confirm, input, spinner};
use concordium_rust_sdk::{
    base::{
        common::{
            cbor,
            types::{
                Amount, CredentialIndex, KeyIndex, Signature, Timestamp, TransactionSignature,
                TransactionTime,
            },
        },
        transactions::{self, BlockItem, TransactionSigner, construct, send},
    },
    common::Serial,
    types::{WalletAccount, hashes::TransactionHash},
    v2::{self},
};
use rusqlite::Connection;
use std::collections::BTreeMap;

const CCD_COIN_TYPE: u32 = 919;
const DEFAULT_EXPIRY_SECONDS: u32 = 300;

#[derive(Clone, Debug)]
pub(crate) struct PreparedCcdMutationContext {
    pub(crate) sender: Option<AccountLabel>,
    pub(crate) network: Option<NetworkName>,
    pub(crate) node: Option<v2::Endpoint>,
    pub(crate) input_mode: InputMode,
    pub(crate) finalization: FinalizationPolicy,
}

impl PreparedCcdMutationContext {
    pub(crate) fn should_wait_for_finalization(&self) -> bool {
        self.finalization.should_wait()
    }
}

pub(crate) struct MutationContext {
    pub(crate) network_name: String,
    pub(crate) network_genesis_hash: String,
    pub(crate) endpoint_label: String,
    pub(crate) client: v2::Client,
    pub(crate) sender_record: accounts::AccountRecord,
    pub(crate) sender_address: concordium_rust_sdk::common::types::AccountAddress,
    pub(crate) account_unlocks: AccountReferenceUnlocks,
    pub(crate) signer_kind: CcdSignerKind,
    pub(crate) input_mode: InputMode,
}

pub(crate) enum CcdSignerKind {
    Local(WalletAccount),
    Ledger,
}

pub(crate) struct ResolvedTransfer {
    pub(crate) recipient: concordium_rust_sdk::common::types::AccountAddress,
    pub(crate) amount: Amount,
    pub(crate) memo: Option<concordium_rust_sdk::types::Memo>,
}

pub(crate) struct ResolvedScheduledTransfer {
    pub(crate) recipient: concordium_rust_sdk::common::types::AccountAddress,
    pub(crate) schedule: Vec<(Timestamp, Amount)>,
    pub(crate) memo: Option<concordium_rust_sdk::types::Memo>,
}

pub(crate) async fn resolve_mutation_context(
    conn: &Connection,
    prepared: &PreparedCcdMutationContext,
) -> Result<MutationContext> {
    let (network_context, selection) = resolve_signing_account_context(
        conn,
        prepared.sender.as_ref().map(AccountLabel::as_str),
        prepared.network.as_ref().map(NetworkName::as_str),
        prepared.node.clone(),
        !prepared.input_mode.prompts_allowed(),
        !prepared.input_mode.defaults_allowed(),
        false,
    )
    .await?;
    let mut lines = vec![ContextLine {
        label: "network:",
        value: format!(
            "{} @ {}",
            network_context.network_name, network_context.endpoint_label
        ),
        source: network_context.source,
    }];
    lines.extend(local_account_context_lines(
        conn,
        &selection.record,
        selection.source,
    )?);
    log_resolved_context(&lines)?;
    let mut account_unlocks = AccountReferenceUnlocks::new();
    let signer_kind =
        resolve_signer_kind(conn, &network_context, &selection, &mut account_unlocks)?;
    let sender_address = match &signer_kind {
        CcdSignerKind::Local(wallet) => wallet.address,
        CcdSignerKind::Ledger => account::decrypt_local_account_address(
            conn,
            &network_context.network_name,
            &selection.record,
        )?,
    };
    let client = node_config::connect_v2_client(network_context.endpoint.clone())
        .await
        .with_context(|| {
            format!(
                "failed to connect to Concordium node at {}",
                network_context.endpoint_label
            )
        })?;
    Ok(MutationContext {
        network_name: network_context.network_name,
        network_genesis_hash: network_context.network_entry.genesis_hash,
        endpoint_label: network_context.endpoint_label,
        client,
        sender_record: selection.record,
        sender_address,
        account_unlocks,
        signer_kind,
        input_mode: prepared.input_mode,
    })
}

fn resolve_signer_kind(
    conn: &Connection,
    network_context: &ResolvedNetworkContext,
    selection: &ResolvedAccountSelection,
    account_unlocks: &mut AccountReferenceUnlocks,
) -> Result<CcdSignerKind> {
    if selection.record.source_kind == accounts::AccountSourceKind::Imported {
        let wallet = build_export_wallet_account_with_unlocks(
            conn,
            &network_context.network_name,
            &network_context.network_entry,
            &selection.record,
            account_unlocks,
        )?;
        return Ok(CcdSignerKind::Local(wallet));
    }
    let owner =
        signer_owners::find_by_id(conn, &selection.record.signer_owner_id)?.with_context(|| {
            format!(
                "derived account '{}' references unknown key source",
                selection.record.label
            )
        })?;
    match owner.kind {
        SignerOwnerKind::Seed => {
            let wallet = build_export_wallet_account_with_unlocks(
                conn,
                &network_context.network_name,
                &network_context.network_entry,
                &selection.record,
                account_unlocks,
            )?;
            Ok(CcdSignerKind::Local(wallet))
        }
        SignerOwnerKind::Ledger => Ok(CcdSignerKind::Ledger),
    }
}

pub(crate) fn resolve_recipient(
    conn: &Connection,
    context: &mut MutationContext,
    explicit: Option<AccountReference>,
) -> Result<concordium_rust_sdk::common::types::AccountAddress> {
    let input = explicit.as_ref().map(ToString::to_string);
    resolve_account_reference(
        conn,
        AccountReferenceContext {
            network_name: &context.network_name,
            network_genesis_hash: &context.network_genesis_hash,
        },
        input.as_deref(),
        "Recipient account address or local label:",
        "recipient",
        !context.input_mode.prompts_allowed(),
        &mut context.account_unlocks,
    )
}

pub(crate) fn prompt_amount(prompt_text: &str) -> Result<Amount> {
    let value: String = input(prompt_text).interact()?;
    parse_decimal_ccd_amount(Some(&value))
}

pub(crate) fn prompt_optional_memo() -> Result<Option<concordium_rust_sdk::types::Memo>> {
    let value: String = input("Memo (leave empty for none):")
        .default_input("")
        .interact()?;
    parse_memo_option(if value.trim().is_empty() {
        None
    } else {
        Some(value)
    })
}

pub(crate) fn parse_memo_option(
    value: Option<String>,
) -> Result<Option<concordium_rust_sdk::types::Memo>> {
    value
        .filter(|memo| !memo.trim().is_empty())
        .map(|memo| cbor::cbor_encode(&memo).try_into().context("invalid memo"))
        .transpose()
}

pub(crate) fn render_memo_for_review(memo: &concordium_rust_sdk::types::Memo) -> String {
    cbor::cbor_decode::<String>(memo.as_ref())
        .unwrap_or_else(|_| String::from_utf8_lossy(memo.as_ref()).into_owned())
}

pub(crate) fn confirm_submission(prompt_text: &str, declined_message: &str) -> Result<bool> {
    if confirm(prompt_text).initial_value(false).interact()? {
        return Ok(true);
    }
    cliclack::log::warning(declined_message)?;
    Ok(false)
}

pub(crate) async fn submit_transfer(
    conn: &Connection,
    context: &mut MutationContext,
    payload: ResolvedTransfer,
) -> Result<TransactionHash> {
    let nonce = context
        .client
        .get_next_account_sequence_number(&context.sender_address)
        .await
        .context("failed to query next account nonce")?
        .nonce;
    let expiry = TransactionTime::seconds_after(DEFAULT_EXPIRY_SECONDS);
    if matches!(context.signer_kind, CcdSignerKind::Ledger) {
        return submit_ledger_transfer(conn, context, nonce, expiry, payload).await;
    }
    let wallet = match &context.signer_kind {
        CcdSignerKind::Local(wallet) => wallet,
        CcdSignerKind::Ledger => unreachable!(),
    };
    submit_local_transfer(
        &mut context.client,
        context.sender_address,
        wallet,
        nonce,
        expiry,
        payload,
    )
    .await
}

async fn submit_local_transfer(
    client: &mut v2::Client,
    sender_address: concordium_rust_sdk::common::types::AccountAddress,
    wallet: &WalletAccount,
    nonce: concordium_rust_sdk::types::Nonce,
    expiry: TransactionTime,
    payload: ResolvedTransfer,
) -> Result<TransactionHash> {
    let transaction = match payload.memo {
        Some(memo) => send::transfer_with_memo(
            wallet,
            sender_address,
            nonce,
            expiry,
            payload.recipient,
            payload.amount,
            memo,
        ),
        None => send::transfer(
            wallet,
            sender_address,
            nonce,
            expiry,
            payload.recipient,
            payload.amount,
        ),
    };
    let block_item = BlockItem::AccountTransaction(transaction);
    client
        .send_block_item(&block_item)
        .await
        .context("failed to submit CCD transfer transaction")
}

async fn submit_ledger_transfer(
    conn: &Connection,
    context: &mut MutationContext,
    nonce: concordium_rust_sdk::types::Nonce,
    expiry: TransactionTime,
    payload: ResolvedTransfer,
) -> Result<TransactionHash> {
    let path = ledger_account_path(&context.sender_record)?;
    let pre = match payload.memo.clone() {
        Some(memo) => construct::transfer_with_memo(
            1,
            context.sender_address,
            nonce,
            expiry,
            payload.recipient,
            payload.amount,
            memo,
        ),
        None => construct::transfer(
            1,
            context.sender_address,
            nonce,
            expiry,
            payload.recipient,
            payload.amount,
        ),
    };
    let raw_signature = sign_transfer_with_ledger(
        conn,
        context,
        &path,
        &pre,
        payload.recipient,
        payload.amount,
        payload.memo.as_ref(),
    )
    .await?;
    let signer = SingleSignatureSigner::new(raw_signature);
    let transaction = pre.sign(&signer);
    let block_item = BlockItem::AccountTransaction(transaction);
    context
        .client
        .send_block_item(&block_item)
        .await
        .context("failed to submit CCD transfer transaction")
}

pub(crate) async fn submit_scheduled_transfer(
    conn: &Connection,
    context: &mut MutationContext,
    payload: ResolvedScheduledTransfer,
) -> Result<TransactionHash> {
    let nonce = context
        .client
        .get_next_account_sequence_number(&context.sender_address)
        .await
        .context("failed to query next account nonce")?
        .nonce;
    let expiry = TransactionTime::seconds_after(DEFAULT_EXPIRY_SECONDS);
    if matches!(context.signer_kind, CcdSignerKind::Ledger) {
        return submit_ledger_scheduled_transfer(conn, context, nonce, expiry, payload).await;
    }
    let wallet = match &context.signer_kind {
        CcdSignerKind::Local(wallet) => wallet,
        CcdSignerKind::Ledger => unreachable!(),
    };
    let transaction = match payload.memo {
        Some(memo) => send::transfer_with_schedule_and_memo(
            wallet,
            context.sender_address,
            nonce,
            expiry,
            payload.recipient,
            payload.schedule,
            memo,
        ),
        None => send::transfer_with_schedule(
            wallet,
            context.sender_address,
            nonce,
            expiry,
            payload.recipient,
            payload.schedule,
        ),
    };
    let block_item = BlockItem::AccountTransaction(transaction);
    context
        .client
        .send_block_item(&block_item)
        .await
        .context("failed to submit CCD scheduled-transfer transaction")
}

async fn submit_ledger_scheduled_transfer(
    conn: &Connection,
    context: &mut MutationContext,
    nonce: concordium_rust_sdk::types::Nonce,
    expiry: TransactionTime,
    payload: ResolvedScheduledTransfer,
) -> Result<TransactionHash> {
    let path = ledger_account_path(&context.sender_record)?;
    let pre = match payload.memo.clone() {
        Some(memo) => construct::transfer_with_schedule_and_memo(
            1,
            context.sender_address,
            nonce,
            expiry,
            payload.recipient,
            payload.schedule.clone(),
            memo,
        ),
        None => construct::transfer_with_schedule(
            1,
            context.sender_address,
            nonce,
            expiry,
            payload.recipient,
            payload.schedule.clone(),
        ),
    };
    let raw_signature = sign_scheduled_transfer_with_ledger(
        conn,
        context,
        &path,
        &pre,
        payload.recipient,
        &payload.schedule,
        payload.memo.as_ref(),
    )
    .await?;
    let signer = SingleSignatureSigner::new(raw_signature);
    let transaction = pre.sign(&signer);
    let block_item = BlockItem::AccountTransaction(transaction);
    context
        .client
        .send_block_item(&block_item)
        .await
        .context("failed to submit CCD scheduled-transfer transaction")
}

async fn sign_transfer_with_ledger(
    conn: &Connection,
    context: &MutationContext,
    path: &DerivationPath,
    pre: &construct::PreAccountTransaction,
    recipient: concordium_rust_sdk::common::types::AccountAddress,
    amount: Amount,
    memo: Option<&concordium_rust_sdk::types::Memo>,
) -> Result<RawSignature> {
    let transport = HidTransport::open_first()
        .context("failed to open Ledger device; connect a Ledger with the Concordium app open")?;
    let mut app = ConcordiumLedgerApp::new(transport);
    ledger::verify_connected_ledger_owner(conn, &context.sender_record.signer_owner_id, &mut app)?;
    let spin = spinner();
    spin.start("Requesting Ledger signature for CCD transfer...");
    let result = match memo {
        Some(memo) => {
            let request =
                build_transfer_with_memo_request(path, &pre.header, recipient, memo, amount);
            app.sign_transfer_with_memo(&request)
        }
        None => {
            let request = ChunkedSigningRequest::new(path.clone(), serialize_transaction(pre))?;
            app.sign_transfer(&request)
        }
    };
    spin.clear();
    result.map_err(|err| match err {
        ccd_wallet_ledger::LedgerError::UserDeclined => {
            anyhow::anyhow!("Ledger signing was declined on the device")
        }
        other => anyhow::anyhow!("Ledger signing failed: {other}"),
    })
}

async fn sign_scheduled_transfer_with_ledger(
    conn: &Connection,
    context: &MutationContext,
    path: &DerivationPath,
    pre: &construct::PreAccountTransaction,
    recipient: concordium_rust_sdk::common::types::AccountAddress,
    schedule: &[(Timestamp, Amount)],
    memo: Option<&concordium_rust_sdk::types::Memo>,
) -> Result<RawSignature> {
    let transport = HidTransport::open_first()
        .context("failed to open Ledger device; connect a Ledger with the Concordium app open")?;
    let mut app = ConcordiumLedgerApp::new(transport);
    ledger::verify_connected_ledger_owner(conn, &context.sender_record.signer_owner_id, &mut app)?;
    let spin = spinner();
    spin.start("Requesting Ledger signature for CCD scheduled transfer...");
    let result = match memo {
        Some(memo) => {
            let request = build_scheduled_transfer_with_memo_request(
                path,
                &pre.header,
                recipient,
                schedule,
                memo,
            )?;
            app.sign_scheduled_transfer_with_memo(&request)
        }
        None => {
            let request = build_scheduled_transfer_request(path, &pre.header, recipient, schedule)?;
            app.sign_scheduled_transfer(&request)
        }
    };
    spin.clear();
    result.map_err(|err| match err {
        ccd_wallet_ledger::LedgerError::UserDeclined => {
            anyhow::anyhow!("Ledger signing was declined on the device")
        }
        other => anyhow::anyhow!("Ledger signing failed: {other}"),
    })
}

fn ledger_account_path(record: &accounts::AccountRecord) -> Result<DerivationPath> {
    DerivationPath::concordium_account(
        CCD_COIN_TYPE,
        record.ip_identity,
        record.identity_index,
        record.credential_counter,
    )
    .context("failed to derive Ledger account path")
}

fn serialize<T: Serial>(value: &T) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.serial(&mut bytes);
    bytes
}

fn serialize_transaction(pre: &construct::PreAccountTransaction) -> Vec<u8> {
    let mut bytes = serialize(&pre.header);
    pre.encoded.serial(&mut bytes);
    bytes
}

fn serialize_schedule(schedule: &[(Timestamp, Amount)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (timestamp, amount) in schedule {
        timestamp.serial(&mut bytes);
        amount.serial(&mut bytes);
    }
    bytes
}

fn build_transfer_with_memo_request(
    path: &DerivationPath,
    header: &transactions::TransactionHeader,
    recipient: concordium_rust_sdk::common::types::AccountAddress,
    memo: &concordium_rust_sdk::types::Memo,
    amount: Amount,
) -> TransferWithMemoSigningRequest {
    let memo_bytes = serialize(memo);
    let (memo_length, memo_value) = memo_bytes.split_at(2);
    let mut header_address_memo_length = path.to_ledger_bytes();
    header_address_memo_length.extend_from_slice(&serialize(header));
    header_address_memo_length.push(22);
    header_address_memo_length.extend_from_slice(&serialize(&recipient));
    header_address_memo_length.extend_from_slice(memo_length);
    TransferWithMemoSigningRequest {
        header_address_memo_length,
        memo: memo_value.to_vec(),
        amount: serialize(&amount),
    }
}

fn build_scheduled_transfer_request(
    path: &DerivationPath,
    header: &transactions::TransactionHeader,
    recipient: concordium_rust_sdk::common::types::AccountAddress,
    schedule: &[(Timestamp, Amount)],
) -> Result<ScheduledTransferSigningRequest> {
    let schedule_len = u8::try_from(schedule.len())
        .context("scheduled transfer supports at most 255 release entries")?;
    let mut header_address_schedule_length = path.to_ledger_bytes();
    header_address_schedule_length.extend_from_slice(&serialize(header));
    header_address_schedule_length.push(19);
    header_address_schedule_length.extend_from_slice(&serialize(&recipient));
    header_address_schedule_length.push(schedule_len);
    Ok(ScheduledTransferSigningRequest {
        header_address_schedule_length,
        schedule: serialize_schedule(schedule),
    })
}

fn build_scheduled_transfer_with_memo_request(
    path: &DerivationPath,
    header: &transactions::TransactionHeader,
    recipient: concordium_rust_sdk::common::types::AccountAddress,
    schedule: &[(Timestamp, Amount)],
    memo: &concordium_rust_sdk::types::Memo,
) -> Result<ScheduledTransferWithMemoSigningRequest> {
    let schedule_len = u8::try_from(schedule.len())
        .context("scheduled transfer supports at most 255 release entries")?;
    let memo_bytes = serialize(memo);
    let (memo_length, memo_value) = memo_bytes.split_at(2);
    let mut header_address_schedule_memo_length = path.to_ledger_bytes();
    header_address_schedule_memo_length.extend_from_slice(&serialize(header));
    header_address_schedule_memo_length.push(24);
    header_address_schedule_memo_length.extend_from_slice(&serialize(&recipient));
    header_address_schedule_memo_length.push(schedule_len);
    header_address_schedule_memo_length.extend_from_slice(memo_length);
    Ok(ScheduledTransferWithMemoSigningRequest {
        header_address_schedule_memo_length,
        memo: memo_value.to_vec(),
        schedule: serialize_schedule(schedule),
    })
}

struct SingleSignatureSigner {
    signature: Signature,
}

impl SingleSignatureSigner {
    fn new(signature: RawSignature) -> Self {
        Self {
            signature: Signature {
                sig: signature.0.to_vec(),
            },
        }
    }
}

impl TransactionSigner for SingleSignatureSigner {
    fn sign_transaction_hash(
        &self,
        _hash_to_sign: &concordium_rust_sdk::base::hashes::TransactionSignHash,
    ) -> TransactionSignature {
        let mut keys = BTreeMap::new();
        keys.insert(KeyIndex(0), self.signature.clone());
        let mut credentials = BTreeMap::new();
        credentials.insert(CredentialIndex::from(0), keys);
        TransactionSignature {
            signatures: credentials,
        }
    }
}

pub(crate) async fn wait_for_finalization(
    client: &mut v2::Client,
    transaction_hash: &TransactionHash,
    network_name: &str,
    endpoint_label: &str,
) -> Result<()> {
    let spin = spinner();
    spin.start("Waiting for transaction finalization...");
    let (block_hash, summary) = client
        .wait_until_finalized(transaction_hash)
        .await
        .context("failed while waiting for transaction finalization")?;
    spin.clear();
    let block_time = client.get_block_info(block_hash).await.ok().map(|info| {
        info.response
            .block_slot_time
            .to_rfc3339_opts(SecondsFormat::Secs, true)
    });
    println!(
        "{}",
        render_finalized_summary(
            transaction_hash,
            &format!("{network_name} @ {endpoint_label}"),
            &block_hash,
            &summary,
            block_time.as_ref(),
        )?
    );
    Ok(())
}

pub(crate) fn render_release_schedule(schedule: &[(Timestamp, Amount)]) -> Vec<String> {
    schedule
        .iter()
        .map(|(timestamp, amount)| {
            format!(
                "{} = {}",
                chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                    timestamp.timestamp_millis() as i64
                )
                .map(|value| value.to_rfc3339_opts(SecondsFormat::Secs, true))
                .unwrap_or_else(|| timestamp.timestamp_millis().to_string()),
                amount
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccd_wallet_core::store::{config::NetworkEntry, migrations};
    use concordium_rust_sdk::types::{Nonce, transactions::PayloadSize};
    use rusqlite::Connection;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    fn header() -> transactions::TransactionHeader {
        transactions::TransactionHeader {
            sender: "47b6Qe2XtZANHetanWKP1PbApLKtS3AyiCtcXaqLMbypKjCaRw"
                .parse()
                .unwrap(),
            nonce: Nonce { nonce: 7 },
            energy_amount: 501u64.into(),
            payload_size: PayloadSize::from(0u32),
            expiry: TransactionTime {
                seconds: 1_719_792_000,
            },
        }
    }

    fn network_context() -> ResolvedNetworkContext {
        ResolvedNetworkContext {
            network_name: "testnet".to_owned(),
            network_entry: NetworkEntry {
                node_endpoint: "https://grpc.testnet.concordium.com:20000".to_owned(),
                genesis_hash: "genesis".to_owned(),
                wallet_proxy: None,
            },
            endpoint: "https://grpc.testnet.concordium.com:20000".parse().unwrap(),
            endpoint_label: "https://grpc.testnet.concordium.com:20000".to_owned(),
            source: crate::commands::ui::ResolutionSource::Explicit,
        }
    }

    fn ledger_record(signer_owner_id: String) -> accounts::AccountRecord {
        accounts::AccountRecord {
            id: 1,
            signer_owner_id,
            network_genesis_hash: "genesis".to_owned(),
            ip_identity: 7,
            identity_index: 2,
            credential_counter: 3,
            source_kind: accounts::AccountSourceKind::Derived,
            imported_vault_id: None,
            import_kind: None,
            source_metadata_json: None,
            label: "ledger-account".to_owned(),
            status: accounts::AccountStatus::Finalized,
            transaction_hash: None,
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn resolve_signer_kind_detects_ledger_backed_accounts() {
        let conn = conn();
        let owner = signer_owners::create(&conn, SignerOwnerKind::Ledger, "ledger-main").unwrap();
        signer_owners::create_vault(&conn, &owner.id, "password").unwrap();
        signer_owners::insert_ledger_details(
            &conn,
            signer_owners::NewLedgerOwnerDetails {
                signer_owner_id: &owner.id,
                canonical_public_key: &[7; 32],
                fingerprint: "07070707",
                enrollment_path: signer_owners::LEDGER_OWNER_ENROLLMENT_PATH,
                app_name: Some("Concordium"),
            },
        )
        .unwrap();

        let signer_kind = resolve_signer_kind(
            &conn,
            &network_context(),
            &ResolvedAccountSelection {
                record: ledger_record(owner.id),
                source: crate::commands::ui::ResolutionSource::Explicit,
            },
            &mut AccountReferenceUnlocks::new(),
        )
        .unwrap();

        assert!(matches!(signer_kind, CcdSignerKind::Ledger));
    }

    #[test]
    fn ledger_account_path_uses_account_coordinates() -> Result<()> {
        let path = ledger_account_path(&ledger_record("owner".to_owned()))?;

        assert_eq!(
            path.indices(),
            DerivationPath::concordium_account(CCD_COIN_TYPE, 7, 2, 3)?.indices()
        );
        Ok(())
    }

    #[test]
    fn transfer_with_memo_request_splits_memo_length_from_bytes() {
        let request = build_transfer_with_memo_request(
            &DerivationPath::concordium_account(CCD_COIN_TYPE, 7, 2, 3).unwrap(),
            &header(),
            header().sender,
            &concordium_rust_sdk::types::Memo::try_from(b"invoice 7".to_vec()).unwrap(),
            Amount::from_micro_ccd(12_500_000),
        );

        assert_eq!(request.header_address_memo_length[0], 5);
        assert_eq!(*request.header_address_memo_length.last().unwrap(), 9);
        assert_eq!(request.memo, b"invoice 7");
        assert_eq!(request.amount.len(), 8);
    }

    #[test]
    fn scheduled_transfer_request_rejects_more_than_255_entries() {
        let schedule = (0..256)
            .map(|_| {
                (
                    Timestamp::from_timestamp_millis(1_719_792_000_000),
                    Amount::from_micro_ccd(1),
                )
            })
            .collect::<Vec<_>>();

        let err = build_scheduled_transfer_request(
            &DerivationPath::concordium_account(CCD_COIN_TYPE, 7, 2, 3).unwrap(),
            &header(),
            header().sender,
            &schedule,
        )
        .unwrap_err();

        assert!(err.to_string().contains("255 release entries"));
    }

    #[test]
    fn scheduled_transfer_with_memo_request_keeps_memo_and_schedule_lengths_separate() -> Result<()>
    {
        let schedule = vec![(
            Timestamp::from_timestamp_millis(1_719_792_000_000),
            Amount::from_micro_ccd(10_000_000),
        )];
        let request = build_scheduled_transfer_with_memo_request(
            &DerivationPath::concordium_account(CCD_COIN_TYPE, 7, 2, 3)?,
            &header(),
            header().sender,
            &schedule,
            &concordium_rust_sdk::types::Memo::try_from(b"vesting".to_vec())?,
        )?;

        assert_eq!(
            *request.header_address_schedule_memo_length.last().unwrap(),
            7
        );
        assert_eq!(request.memo, b"vesting");
        assert_eq!(request.schedule.len(), 16);
        Ok(())
    }

    #[test]
    fn render_release_schedule_formats_rfc3339_output() {
        let rendered = render_release_schedule(&[(
            Timestamp::from_timestamp_millis(1_719_792_000_000),
            Amount::from_micro_ccd(10_000_000),
        )]);

        assert_eq!(rendered, vec!["2024-07-01T00:00:00Z = 10.0".to_owned()]);
    }

    #[test]
    fn parse_memo_option_ignores_empty_values() -> Result<()> {
        assert_eq!(parse_memo_option(None)?, None);
        assert_eq!(parse_memo_option(Some("   ".to_owned()))?, None);
        Ok(())
    }

    #[test]
    fn parse_memo_option_preserves_non_empty_memo() -> Result<()> {
        let memo = parse_memo_option(Some("invoice 7".to_owned()))?.expect("memo present");

        let expected = cbor::cbor_encode(&"invoice 7".to_owned());
        assert_eq!(memo.as_ref(), expected.as_slice());
        Ok(())
    }

    #[test]
    fn render_memo_for_review_decodes_cbor_text() -> Result<()> {
        let memo = parse_memo_option(Some("this is a memo".to_owned()))?.expect("memo present");

        assert_eq!(render_memo_for_review(&memo), "this is a memo");
        Ok(())
    }
}
