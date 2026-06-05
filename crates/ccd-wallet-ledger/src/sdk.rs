//! Optional conversions from Concordium Rust SDK values into Ledger request types.
//!
//! The conversions in this module are enabled by the crate's `sdk` feature. They keep the
//! high-level SDK dependency out of the default build while providing a small bridge for request
//! types whose byte layout can be derived directly from SDK transaction values.

use crate::types::{
    ChunkedSigningRequest, ContractSigningRequest, DeployModuleSigningRequest, DerivationPath,
    RegisterDataSigningRequest,
};
use concordium_rust_sdk::{
    base::protocol_level_tokens::{TokenOperationsPayload, meta_operations::MetaUpdatePayload},
    common::Serial,
    types::{
        smart_contracts::WasmModule,
        transactions::{
            ConfigureDelegationPayload, InitContractPayload, Payload, PayloadLike, RegisteredData,
            TransactionHeader, UpdateContractPayload,
        },
    },
};

/// SDK account-transaction data together with the Ledger derivation path used for signing.
///
/// SDK account transaction payload bodies do not carry the Ledger signing path or the transaction
/// header. This wrapper supplies the missing context so body structs such as
/// [`InitContractPayload`] or [`RegisteredData`] can be converted into crate-local Ledger request
/// types.
///
/// # Arguments
///
/// * `P` - SDK payload or payload-body type.
///
/// # Examples
///
/// ```no_run
/// use ccd_wallet_ledger::{DerivationPath, sdk::SdkAccountTransactionInput};
/// # use concordium_rust_sdk::types::transactions::{RegisteredData, TransactionHeader};
/// # fn example(path: DerivationPath, header: TransactionHeader, payload: RegisteredData) {
/// let input = SdkAccountTransactionInput::new(path, header, payload);
/// # let _ = input;
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct SdkAccountTransactionInput<P> {
    /// Derivation path for the Ledger signing key.
    pub path: DerivationPath,
    /// SDK transaction header to serialize before the payload.
    pub header: TransactionHeader,
    /// SDK transaction payload or payload body.
    pub payload: P,
}

impl<P> SdkAccountTransactionInput<P> {
    /// Construct SDK-backed Ledger signing input.
    ///
    /// # Arguments
    ///
    /// * `path` - Derivation path for the Ledger signing key.
    /// * `header` - SDK account transaction header.
    /// * `payload` - SDK account transaction payload or payload body.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ccd_wallet_ledger::{DerivationPath, sdk::SdkAccountTransactionInput};
    /// # use concordium_rust_sdk::types::transactions::{RegisteredData, TransactionHeader};
    /// # fn example(path: DerivationPath, header: TransactionHeader, payload: RegisteredData) {
    /// let input = SdkAccountTransactionInput::new(path, header, payload);
    /// # let _ = input;
    /// # }
    /// ```
    pub fn new(path: DerivationPath, header: TransactionHeader, payload: P) -> Self {
        Self {
            path,
            header,
            payload,
        }
    }
}

impl From<SdkAccountTransactionInput<Payload>> for ChunkedSigningRequest {
    fn from(value: SdkAccountTransactionInput<Payload>) -> Self {
        let transaction = serialize_payload(&value.header, &value.payload);
        Self {
            path: value.path,
            transaction,
        }
    }
}

impl From<SdkAccountTransactionInput<ConfigureDelegationPayload>> for ChunkedSigningRequest {
    fn from(value: SdkAccountTransactionInput<ConfigureDelegationPayload>) -> Self {
        let payload = Payload::ConfigureDelegation {
            data: value.payload,
        };
        let transaction = serialize_payload(&value.header, &payload);
        Self {
            path: value.path,
            transaction,
        }
    }
}

impl From<SdkAccountTransactionInput<TokenOperationsPayload>> for ChunkedSigningRequest {
    fn from(value: SdkAccountTransactionInput<TokenOperationsPayload>) -> Self {
        let payload = Payload::TokenUpdate {
            payload: value.payload,
        };
        let transaction = serialize_payload(&value.header, &payload);
        Self {
            path: value.path,
            transaction,
        }
    }
}

impl From<SdkAccountTransactionInput<MetaUpdatePayload>> for ChunkedSigningRequest {
    fn from(value: SdkAccountTransactionInput<MetaUpdatePayload>) -> Self {
        let payload = Payload::MetaUpdate {
            payload: value.payload,
        };
        let transaction = serialize_payload(&value.header, &payload);
        Self {
            path: value.path,
            transaction,
        }
    }
}

impl From<SdkAccountTransactionInput<WasmModule>> for DeployModuleSigningRequest {
    fn from(value: SdkAccountTransactionInput<WasmModule>) -> Self {
        let module = serialize(&value.payload);
        let (version_and_source_length, source) = module.split_at(8);
        let mut header_and_version = serialize(&value.header);
        header_and_version.push(0);
        header_and_version.extend_from_slice(version_and_source_length);

        Self {
            path: value.path,
            header_and_version,
            source: source.to_vec(),
        }
    }
}

impl From<SdkAccountTransactionInput<InitContractPayload>> for ContractSigningRequest {
    fn from(value: SdkAccountTransactionInput<InitContractPayload>) -> Self {
        let payload = value.payload;
        let mut header_and_data = serialize(&value.header);
        header_and_data.push(1);
        append_serialized(&mut header_and_data, &payload.amount);
        append_serialized(&mut header_and_data, &payload.mod_ref);

        Self {
            path: value.path,
            header_and_data,
            name: split_serialized_u16_prefixed_bytes(&serialize(&payload.init_name)).to_vec(),
            parameter: split_serialized_u16_prefixed_bytes(&serialize(&payload.param)).to_vec(),
        }
    }
}

impl From<SdkAccountTransactionInput<UpdateContractPayload>> for ContractSigningRequest {
    fn from(value: SdkAccountTransactionInput<UpdateContractPayload>) -> Self {
        let payload = value.payload;
        let mut header_and_data = serialize(&value.header);
        header_and_data.push(2);
        append_serialized(&mut header_and_data, &payload.amount);
        append_serialized(&mut header_and_data, &payload.address);

        Self {
            path: value.path,
            header_and_data,
            name: split_serialized_u16_prefixed_bytes(&serialize(&payload.receive_name)).to_vec(),
            parameter: split_serialized_u16_prefixed_bytes(&serialize(&payload.message)).to_vec(),
        }
    }
}

impl From<SdkAccountTransactionInput<RegisteredData>> for RegisterDataSigningRequest {
    fn from(value: SdkAccountTransactionInput<RegisteredData>) -> Self {
        let serialized_data = serialize(&value.payload);
        let (data_length, data) = serialized_data.split_at(2);
        let mut header = value.path.to_ledger_bytes();
        header.extend_from_slice(&serialize(&value.header));
        header.push(21);
        header.extend_from_slice(data_length);

        Self {
            header,
            data: data.to_vec(),
        }
    }
}

fn serialize<T: Serial>(value: &T) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.serial(&mut bytes);
    bytes
}

fn append_serialized<T: Serial>(bytes: &mut Vec<u8>, value: &T) {
    value.serial(bytes);
}

fn serialize_payload(header: &TransactionHeader, payload: &Payload) -> Vec<u8> {
    let mut transaction = serialize(header);
    payload.encode_to_buffer(&mut transaction);
    transaction
}

fn split_serialized_u16_prefixed_bytes(bytes: &[u8]) -> &[u8] {
    &bytes[2..]
}
